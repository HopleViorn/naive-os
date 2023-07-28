#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(const_trait_impl)]
#![feature(map_first_last)]
#![allow(unused)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::format;
use alloc::string::{ToString, String};
use async_task::Runnable;
use lazy_static::{lazy_static, __Deref};
use riscv::register::sstatus::{self, FS};
use spin::Mutex;
use spin::mutex::SpinMutex;
use sync::UPSafeCell;
use xmas_elf::header::sanity_check;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
extern crate bitflags;
#[macro_use]
mod console;
mod config;
mod fs;
mod lang_items;
mod signal;
mod mm;
mod sbi;
mod sync;
mod task;

pub mod syscall;
pub mod timer;
pub mod trap;
use crate::fs::block_dev::{init_block_dev};
use crate::fs::file::{RegFileINode, OpenFlags};
use crate::mm::memory_set::{MapArea, MapPermission, MapType};
use crate::mm::VirtAddr;
use crate::task::{TASK_QUEUE, Thread, PID_ALLOCATOR, Process, GLOBAL_DENTRY_CACHE};
use crate::trap::{user_loop, set_kernel_trap};
use crate::{
    config::{KERNEL_STACK_SIZE, TRAMPOLINE, USER_STACK_SIZE},
    console::print,
    mm::{translated_byte_buffer, MemorySet, KERNEL_SPACE},
    sbi::{console_getchar, console_putchar, shutdown},
    task::{ ProcessContext, PCB},
    timer::{get_time, set_next_trigger},
};
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use config::TRAPFRAME;
use core::future::Future;
use core::ops::DerefMut;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::{
    arch::asm,
    arch::global_asm,
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell},
    slice::{self, SliceIndex},
    str::Bytes,
    sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
};
use trap::TrapFrame;
use xmas_elf::ElfFile;

use riscv::register::{
    mhartid,
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("user_bin.S"));

fn crate_task_from_elf(userbin: &[u8]) {
    // let userbin=include_bytes!("../../../testsuits-for-oskernel/riscv-syscalls-testing/user/build/riscv64/write");
    let elf_file = ElfFile::new(userbin).unwrap();

	let pid =PID_ALLOCATOR.alloc_pid();
	let mut task=PCB::new();

    // let user_pagetable=&mut task.memory_set;
    let (user_pagetable,heap_pos, user_stack, entry) = MemorySet::from_elf(&elf_file);
    task.heap_pos=heap_pos.into();
    task.mmap_pos=(0x10000_0000).into();
    println!("entry:{:#x}", entry);
    KERNEL_SPACE.lock().insert_framed_area(
        (TRAMPOLINE - KERNEL_STACK_SIZE * (pid + 1)).into(),
        (TRAMPOLINE - KERNEL_STACK_SIZE * pid).into(),
        MapPermission::R | MapPermission::W,
    );
    //trapframe
    task.trapframe_ppn = user_pagetable
        .translate(VirtAddr::from(TRAPFRAME).into())
        .unwrap()
        .ppn();

    task.memory_set = user_pagetable;

    *task.trapframe_ppn.get_mut() = TrapFrame::app_init_context(
        entry,
        user_stack - 8,
        KERNEL_SPACE.lock().token(),
        TRAMPOLINE - KERNEL_STACK_SIZE * pid,
        0 as usize,
    );
    task.context.sp = TRAMPOLINE - KERNEL_STACK_SIZE * pid;
    task.context.ra = 0 as usize;
	let new_proc= Arc::new(Process::new(task));
	
	let thread=Arc::new(Thread::new(new_proc));

	unsafe{
		let (r,t)=async_task::spawn(user_loop(thread), |runnable|{TASK_QUEUE.push(runnable);});
		r.schedule();
		t.detach();
	}
}

pub fn insert_file(path:&str,name:&str,content:&[u8]){
    unsafe{
		let inode=RegFileINode::new_from_existed(
			path.to_string(),
			name.to_string(),
			OpenFlags::CREATE,
			true,true,
            content,
		);
		let inode=Arc::new(Mutex::new(inode));
		GLOBAL_DENTRY_CACHE.insert(format!("{}/{}",path,name).as_str(),inode);
	}
}


#[no_mangle]
fn load_core_program() {
    unsafe{
        extern "C" {
            fn init_start();
            fn init_end();
            fn shell_start();
            fn shell_end();
        }
        insert_file("/core","shell",slice::from_raw_parts(shell_start as *const u8, shell_end as usize - shell_start as usize));
        // insert_file("/core","init",slice::from_raw_parts(init_start as *const u8, init_end as usize - init_start as usize));

        crate_task_from_elf(slice::from_raw_parts(
            init_start as *const u8,
            init_end as usize - init_start as usize,
        ));

        insert_file("/etc", "localtime", "0000".as_bytes());
    }
}

// static LOCK: AtomicU8 = AtomicU8::new(0);
struct Booting{
	state:bool
}

impl Booting{
	pub const fn new()->Self{
		Self{
			state:false
		}
	}
	pub fn do_finish(&self)->bool{
		self.state
	}
	pub fn set_finish(& mut self){
		self.state=true;
	}
}

static BOOT:Mutex<Booting>=Mutex::new(Booting :: new());

#[link_section = "data"]
static FIRST_HART: AtomicBool = AtomicBool::new(false);
static INIT_START: AtomicBool = AtomicBool::new(false);

macro_rules! smp_v {
    ($a: ident -= $v: literal) => {
        $a.fetch_sub($v, Ordering::Release);
    };
    ($a: ident => $v: literal) => {
        while $a.load(Ordering::Acquire) != $v {
            core::hint::spin_loop();
        }
    };
    ($v: expr => $a: ident) => {
        $a.store($v, Ordering::Release);
    };
    ($a: ident => $v: expr) => {
        while $a.load(Ordering::Acquire) != $v {
            core::hint::spin_loop();
        }
    };
}

fn init_fp(){
    unsafe{
        sstatus::set_fs(FS::Clean);
    }
}


#[no_mangle]
pub fn rust_main(hart_id:usize) -> ! {
	if FIRST_HART
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
	{
		println!("");
		println!("     _   _           ____   ______");
		println!("    / | / |        / __  \\/ _____/");
		println!("   /  |/  | ____  / /  / / /__");
		println!("  / /| /| |/ __ \\/ /  / /\\___ \\");
		println!(" / / |/ | / /_/ / /__/ /____/ /");
		println!("/_/     |_\\____/\\_____/______/");
		println!("");
		clear_bss();
        init_fp();
		mm::init();
		trap::init();
		KERNEL_SPACE.lock().activate();
		init_block_dev();
		// unsafe {sie::set_stimer();}
		Thread::sys_mount();
		load_core_program();
		smp_v!(true => INIT_START);
	}else{
		smp_v!(INIT_START => true);
		println!("hart {} booting.",hart_id);
		// trap::init();
        loop{}
	}
    println!("Mount Success.");
    println!("Entering Loop.");
	//enter userloop
	loop{
		if let Some(runnable)=TASK_QUEUE.fetch(){
			// println!("{}",TASK_QUEUE.len());
			// println!("hart_id:{}",hart_id);
			runnable.run();
		}else{
		}
	}

    println!("[main] unreachable part.");
    loop {}
}

#[inline(always)]
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(map_first_last)]
#![allow(unused)]

extern crate alloc;

use alloc::collections::VecDeque;
use async_task::Runnable;
use lazy_static::lazy_static;
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
mod mm;
mod sbi;
mod sync;
mod task;

pub mod syscall;
pub mod timer;
pub mod trap;
use crate::fs::block_dev::block_device_test;
use crate::mm::memory_set::{MapArea, MapPermission, MapType};
use crate::mm::VirtAddr;
use crate::task::{TASK_QUEUE, Thread, PID_ALLOCATOR, Process};
use crate::trap::{user_loop, set_kernel_trap};
use crate::{
    config::{KERNEL_STACK_SIZE, TRAMPOLINE, USER_STACK_SIZE},
    console::print,
    mm::{translated_byte_buffer, MemorySet, KERNEL_SPACE},
    sbi::{console_getchar, console_putchar, shutdown},
    task::{task_list, ProcessContext, PCB},
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




// lazy_static!{
// 	static ref pid_top:Arc<Mutex<usize>>=Arc::new(Mutex::new(0));
// }


fn crate_task_from_elf(userbin: &[u8]) {
    // let userbin=include_bytes!("../../../testsuits-for-oskernel/riscv-syscalls-testing/user/build/riscv64/write");
    let elf_file = ElfFile::new(userbin).unwrap();

	let pid =PID_ALLOCATOR.alloc_pid();
	let mut task=PCB::new();

    // let user_pagetable=&mut task.memory_set;
    let (user_pagetable, user_stack, entry) = MemorySet::from_elf(&elf_file);
    println!("entry:{:#x}", entry);
    KERNEL_SPACE.exclusive_access().insert_framed_area(
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
        KERNEL_SPACE.exclusive_access().token(),
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


#[no_mangle]
unsafe fn load_user_file() {
    extern "C" {
        fn init_start();
        fn init_end();
		fn forktest_start();
        fn forktest_end();
		fn busybox_start();
        fn busybox_end();
    }
    // crate_task_from_elf(slice::from_raw_parts(
    //     busybox_start as *const u8,
    //     busybox_end as usize - init_start as usize,
    // ));
    crate_task_from_elf(slice::from_raw_parts(
        init_start as *const u8,
        init_end as usize - init_start as usize,
    ));
	loop{
		let runnable: Runnable=TASK_QUEUE.fetch();
		runnable.run();
	}
}

static LOCK: AtomicU8 = AtomicU8::new(0);

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    while (!LOCK
        .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok())
		{}
		// println!("-----------NAIVE-OS-----------");
		println!("");
		println!("     _   _           ____   ______");
		println!("    / | / |        / __  \\/ _____/");
		println!("   /  |/  | ____  / /  / / /__");
		println!("  / /| /| |/ __ \\/ /  / /\\___ \\");
		println!(" / / |/ | / /_/ / /__/ /____/ /");
		println!("/_/     |_\\____/\\_____/______/");
		println!("");
		trap::init();
		mm::init();
    // unsafe {sie::set_stimer();}
    // set_next_trigger();
	// block_device_test();
	// panic!("success.");
    unsafe {
        load_user_file();
    }
    println!("unreachable part.");
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

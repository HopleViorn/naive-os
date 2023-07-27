
use alloc::{task, vec::Vec, string::String};
use async_task::Runnable;
use xmas_elf::ElfFile;

use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAPFRAME, USER_STACK_SIZE, PRINT_SYSCALL},
    console::print,
    mm::{
        memory_set::{self, MapArea, MapPermission, KERNEL_SPACE},
        page_table::{translate_str, PageTable},
        translated_byte_buffer, MemorySet, VirtAddr,
    }, trap::TrapFrame,
};
use super::{ProcessContext, ProcessState, __switch, PCB, TASK_QUEUE, Thread};

impl Thread{

pub unsafe fn exec_from_elf(&self ,elf_file: &ElfFile, argv: usize) -> isize {
	let (user_pagetable,heap_pos, mut user_stack, entry) = MemorySet::from_elf(&elf_file);
    let mut nowproc = &mut self.proc.inner.lock();
	nowproc.trapframe_ppn = user_pagetable
		.translate(VirtAddr::from(TRAPFRAME).into())
		.unwrap()
		.ppn();
	nowproc.heap_pos=heap_pos.into();
	// nowproc.mmap_pos=(TRAPFRAME-USER_STACK_SIZE*2).into();
	nowproc.mmap_pos=0x10000_0000.into();
	nowproc.fd_manager.close_on_exec();

	let mut user_stack_kernel: usize = PageTable::from_token(user_pagetable.token())
		.translate_va(VirtAddr::from(user_stack - 8))
		.unwrap()
		.get_mut() as *mut u8 as usize
		+ 8;

		let mut argc = 0;
		let mut pos: Vec<usize> = Vec::new();
		let mut pos_env: Vec<usize> = Vec::new();

		pos_env.push(0);

		let mut debug_info:Vec<String> =Vec::new();

		if (argv != 0) {
			loop {
            let argv_i_ptr = *(self.translate(argv + argc * 8) as *mut usize);
            if (argv_i_ptr == 0) {
				break;
            }
            let argv_i = argv_i_ptr as *mut u8;
            let mut s = translate_str(nowproc.memory_set.token(), argv_i);
            s.push(0 as char);
            let src = s.as_bytes();
            user_stack_kernel -= s.len();
            user_stack -= s.len();
			
            let dst = core::slice::from_raw_parts_mut((user_stack_kernel) as *mut u8, s.len());
            dst.copy_from_slice(src);
            pos.push(user_stack);
            argc += 1;

			debug_info.push(s);
        }
    }
	pos.push(0);

	//AT_NULL
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=0;
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=0;

	//AT_PAGESIZE
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=0x1000;
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=6;

	//AT_RANDOM
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=user_stack;
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=25;

	let len=pos_env.len();
    for i in 0..len {
		user_stack_kernel -= 8;
        user_stack -= 8;
        *(user_stack_kernel as *mut usize) = pos_env[len - i -1 ];
		// println!("{:#x}:::{:#x}",user_stack,pos[len-i-1]);
    }
	let env_begin=user_stack;

	let len=pos.len();
    for i in 0..len {
		user_stack_kernel -= 8;
        user_stack -= 8;
        *(user_stack_kernel as *mut usize) = pos[len - i -1 ];
		// println!("{:#x}:::{:#x}",user_stack,pos[len-i-1]);
    }
	let argv_begin=user_stack;

	// **envp
	// user_stack_kernel -= 8;
	// user_stack -= 8;
	// *(user_stack_kernel as *mut usize) = env_begin;

	// // **argv
	// user_stack_kernel -= 8;
	// user_stack -= 8;
	// *(user_stack_kernel as *mut usize) = argv_begin;
	// println!("argv_begin:{:#x}",argv_begin);
	
	
	//argc
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize) = argc;
	if PRINT_SYSCALL{
		print!("[execve] ");
		for arg in debug_info{
			print!("{} ",arg);
		}
		println!("");
		println!("         usert_stack:{:#x}",user_stack);
		println!("         entry:{:#x}",entry);
	}
	
    *(nowproc.trapframe_ppn.get_mut() as *mut TrapFrame) = TrapFrame::app_init_context(
		entry,
        user_stack,
        KERNEL_SPACE.lock().token(),
        TRAMPOLINE - KERNEL_STACK_SIZE * nowproc.pid,
        0 as usize,
    );
    nowproc.memory_set = user_pagetable;
    0
}


pub unsafe fn kill() {
}

}
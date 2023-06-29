use alloc::{task, vec::Vec};
use async_task::Runnable;
use xmas_elf::ElfFile;

use crate::{
    config::{KERNEL_STACK_SIZE, PAGE_SIZE, TRAMPOLINE, TRAPFRAME, USER_STACK_SIZE},
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
	let (user_pagetable, mut user_stack, entry) = MemorySet::from_elf(&elf_file);
    let mut nowproc = &mut self.proc.inner.lock();
	nowproc.trapframe_ppn = user_pagetable
		.translate(VirtAddr::from(TRAPFRAME).into())
		.unwrap()
		.ppn();

	let mut user_stack_kernel: usize = PageTable::from_token(user_pagetable.token())
		.translate_va(VirtAddr::from(user_stack - 8))
		.unwrap()
		.get_mut() as *mut u8 as usize
		+ 8;

		let mut argc = 0;
		let mut pos: Vec<usize> = Vec::new();

		
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
        }
    }

	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize)=0x2333;
	let at_random=user_stack;
	//env
	pos.push(0);

	//AT_RANDOM
	pos.push(25);
	pos.push(at_random);

	//AT_NULL
	pos.push(0);
	pos.push(0);
	
	let len=pos.len();
    for i in 0..len {
		user_stack_kernel -= 8;
        user_stack -= 8;
        *(user_stack_kernel as *mut usize) = pos[len - i -1 ];
		// println!("{:#x}:::{:#x}",user_stack,pos[len-i-1]);
    }
	let argv_begin=user_stack;

	// **argv
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize) = argv_begin;
	// println!("{:#x}",argv_begin);
	
	
	//argc
	user_stack_kernel -= 8;
	user_stack -= 8;
	*(user_stack_kernel as *mut usize) = argc;
	// println!("argc:{}",argc);
	// println!("usert_stack:{:#x}",user_stack);
	
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
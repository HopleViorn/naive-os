//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.

const SYSCALL_GETCWD: usize = 17;
const SYSCALL_DUP: usize = 23;
const SYSCALL_DUP3: usize = 24;
const SYSCALL_MKDIRAT: usize = 34;
const SYSCALL_UMOUNT: usize = 39;
const SYSCALL_MOUNT: usize = 40;
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYS_GETDENTS64: usize = 61;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_FSSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_NANOSLEEP: usize =101;
const SYSCALL_SCHED_YIELD: usize = 124;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_GETTIMEOFDAY: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_BRK: 	usize = 214;
const SYSCALL_MUNMAP:	usize = 215;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXECVE: usize = 221;
const SYSCALL_MMAP:	usize = 222;
const SYSCALL_WAITPID: usize = 260;
pub mod fs;
pub mod process;
pub mod interrupt;
pub mod mm;

use fs::*;
use process::*;
use interrupt::*;
use mm::*;

use crate::{task::{cpu::mycpu, task_list, myproc}, mm::{VirtAddr, page_table::{PageTable, translate_str}}};

#[repr(C)]
pub struct timespec{
	tv_sec: usize,
	tv_nsec: usize
}

pub fn translate(ptr: usize)-> usize{
	PageTable::from_token(myproc().memory_set.token()).translate_va(VirtAddr::from(ptr as usize)).unwrap().get_mut() as *mut u8 as usize
}

/// handle syscall exception with `syscall_id` and other arguments
pub unsafe fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
		SYSCALL_NANOSLEEP => sys_nanosleep(translate(args[0]) as *mut timespec,translate(args[1]) as *mut timespec),
		SYSCALL_READ => sys_read(args[0], args[1] as *mut u8, args[2]),
		SYSCALL_SCHED_YIELD=> sys_yield(),
		SYSCALL_GETTIMEOFDAY=> sys_gettimeofday(args[0] as *mut usize),
		SYSCALL_GETPID => sys_getpid(),
		SYSCALL_GETPPID => sys_getppid(),
		SYSCALL_CLONE => sys_clone(args[1]),
		SYSCALL_EXECVE => sys_exec(args[0] as *mut u8,args[1] as usize),
		SYSCALL_WAITPID => sys_waitpid(args[0] as isize,if(args[1]==0){0}else{translate(args[1])} as *mut isize,args[2]),
		SYSCALL_TIMES=> sys_times(translate(args[0])),
		SYSCALL_UMOUNT => 0,
		SYSCALL_MOUNT => 0,
		SYSCALL_BRK => sys_brk(args[0]),
		SYSCALL_OPENAT => sys_openat(args[0] as isize, &translate_str(get_token(), args[1] as *mut u8), args[2] as isize),
		SYSCALL_CLOSE => sys_close(args[0] as isize),
		SYSCALL_GETCWD => sys_getcwd(args[0] as *mut u8, args[1]),
		SYS_GETDENTS64 => sys_getdents64(args[0] as usize, args[1] as *mut u8, args[2] as usize),
		SYSCALL_DUP => sys_dup(args[0] as isize),
		SYSCALL_DUP3 => sys_dup3(args[0] as isize, args[1] as isize, args[2] as isize),
		SYSCALL_MKDIRAT => sys_mkdirat(args[0] as isize, &translate_str(get_token(), args[1] as *mut u8), args[2] as usize),
		// SYSCALL_CHDIR => sys_chdir(&translate_str(get_token(), args[0] as *mut u8)),
		// SYSCALL_FSSTAT => sys_fsstat(args[0] as isize, args[1] as *mut u8),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

pub fn get_token() -> usize {
		task_list.exclusive_access()[mycpu().proc_idx].memory_set.token()
	}

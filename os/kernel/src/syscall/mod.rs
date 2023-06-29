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
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_UMOUNT: usize = 39;
const SYSCALL_MOUNT: usize = 40;
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE2: usize = 59;
const SYSCALL_GETDENTS64: usize = 61;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_READV: usize = 65;
const SYSCALL_WRITEV: usize = 66;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_SET_TID_ADDRESS: usize = 96;
const SYSCALL_SET_ROBUST_LIST: usize = 99;
const SYSCALL_GET_ROBUST_LIST: usize = 100;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_SCHED_YIELD: usize = 124;
const SYSCALL_TGKILL: usize = 131;
const SYSCALL_SIGACTION: usize = 134;
const SYSCALL_SIGPROCMASK: usize = 135;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_UNAME: usize = 160;
const SYSCALL_GETTIMEOFDAY: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_GETUID: usize = 174;
const SYSCALL_GETEUID: usize = 175;
const SYSCALL_GETGID: usize = 176;
const SYSCALL_GETEGID: usize = 177;
const SYSCALL_GETTID: usize = 178;
const SYSCALL_BRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXECVE: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_WAITPID: usize = 260;

pub mod fs;
pub mod interrupt;
pub mod mm;
pub mod process;
pub mod raw_ptr;

use fs::*;
use interrupt::*;
use mm::*;
use process::*;
use raw_ptr::*;

use crate::{
    mm::{
        page_table::{translate_str, PageTable},
        VirtAddr,
    },
    task::{PCB, proc, Thread},
};

#[repr(C)]
pub struct timespec {
    tv_sec: usize,
    tv_nsec: usize,
}


/// handle syscall exception with `syscall_id` and other arguments

impl Thread{
	pub fn translate(& self,ptr: usize) -> usize {
		unsafe{self.proc.inner.force_unlock();};
		PageTable::from_token(self.proc.inner.lock().memory_set.token())
			.translate_va(VirtAddr::from(ptr as usize))
			.unwrap()
			.get_mut() as *mut u8 as usize
	}
	pub async unsafe fn syscall(& self, syscall_id: usize, args: [usize; 6]) -> isize {
		println!("[syscall] id={}",syscall_id);
		let result = match syscall_id {
			SYSCALL_WRITE => self.sys_write(args[0], args[1] as *const u8, args[2]),
			SYSCALL_WRITEV => self.sys_writev(args[0], self.translate(args[1]) as *const usize, args[2]),
			SYSCALL_EXIT =>  self.sys_exit(args[0] as i32),
			SYSCALL_NANOSLEEP => Thread::sys_nanosleep(
				self.translate(args[0]),
				self.translate(args[1]),
			).await,
			SYSCALL_READ => self.sys_read(args[0] as usize, args[1], args[2]).await,
			SYSCALL_SCHED_YIELD => {Thread::async_yield().await;0},
			SYSCALL_GETTIMEOFDAY => self.sys_gettimeofday(args[0] as *mut usize),
			SYSCALL_GETPID => self.sys_getpid(),
			SYSCALL_GETPPID => self.sys_getppid(),
			SYSCALL_CLONE => self.sys_clone(args[1]),
			SYSCALL_EXECVE => self.sys_exec(args[0] as *mut u8, args[1] as usize),
			SYSCALL_WAITPID => self.sys_waitpid(
				args[0] as isize,
				if (args[1] == 0) {
					UserPtr::<isize,Out>::from_usize(0)
				} else {
					UserPtr::<isize,Out>::from_usize(self.translate(args[1]))
				} ,
				args[2],
			).await,
			SYSCALL_TIMES => self.sys_times(self.translate( args[0])),
			SYSCALL_UMOUNT => self.sys_umount(),
			SYSCALL_MOUNT => self.sys_mount(),
			SYSCALL_BRK => self.sys_brk(args[0]),
			SYSCALL_OPENAT => self.sys_openat(
				args[0] as isize,
				args[1],
				args[2] as isize,
			),
			SYSCALL_CLOSE => self.sys_close(args[0] as isize),
			SYSCALL_GETCWD => self.sys_getcwd(args[0] as *mut u8, args[1]),
			SYSCALL_GETDENTS64 => self.sys_getdents64(
				args[0] as usize,
				self.translate(args[1]) as *mut u8,
				args[2] as usize,
			),
			SYSCALL_DUP => self.sys_dup(args[0] as isize),
			SYSCALL_DUP3 => self.sys_dup3(args[0] as isize, args[1] as isize, args[2] as isize),
			SYSCALL_MKDIRAT => self.sys_mkdirat(
				args[0] as isize,
				args[1],
				args[2] as usize,
			),
			SYSCALL_CHDIR => self.sys_chdir(args[0]),
			SYSCALL_FSTAT => self.sys_fstat(args[0] as isize, args[1] as *mut u8),
			SYSCALL_UNLINKAT => self.sys_unlinkat(
				args[0] as isize,
				args[1],
				args[2] as usize,
			),
			SYSCALL_UNAME => Thread::sys_uname(self.translate(args[0]) as *mut u8),
			SYSCALL_MUNMAP => Thread::sys_munmap(args[0] as *mut usize, args[1] as usize),
			SYSCALL_MMAP => self.sys_mmap(
				args[0] as usize,
				args[1] as u32 as usize,
				args[2] as i32,
				args[3] as i32,
				args[4] as usize,
				args[5] as usize,
			),
			SYSCALL_PIPE2 => {
				self.sys_pipe2(self.translate(args[0]) as *mut u32)
			}
			SYSCALL_GETEUID => 0,
			SYSCALL_GETUID => 0,
			SYSCALL_GETGID => 0,
			SYSCALL_GETEGID => 0,
			SYSCALL_SET_ROBUST_LIST => 0,
			SYSCALL_SET_TID_ADDRESS => 0, //TODO
			SYSCALL_SIGPROCMASK=>0,//TODO
			SYSCALL_GETTID =>0,//TODO
			SYSCALL_TGKILL=>0,//TODO
			SYSCALL_SIGACTION=>0,//TODO
			_ => panic!("Unsupported syscall_id: {}", syscall_id),
		};
		result
	}
}

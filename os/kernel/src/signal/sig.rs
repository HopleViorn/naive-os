use crate::{task::Thread, config::PRINT_SYSCALL};



impl Thread {
	pub fn sys_sigaction(&self, signum:usize) -> isize{
		if PRINT_SYSCALL{println!("[sigaction] {}",signum);}
		0
	}
}
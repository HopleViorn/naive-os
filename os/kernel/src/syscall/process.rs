//! App management syscalls

use core::{clone, future::Future, pin::Pin, task::{Context, Poll}, ops::DerefMut};

use alloc::{
    boxed::Box,
    slice,
    string::{String, ToString},
    sync::Arc,
    task,
    vec::Vec,
};
use lazy_static::lazy_static;
use xmas_elf::ElfFile;

use crate::{
    mm::{page_table::translate_str, translated_byte_buffer, MemorySet, VirtAddr, KERNEL_SPACE, MapPermission},
    sync::UPSafeCell,
    task::{
         ProcessState, PCB, Thread, TASK_QUEUE, PID_ALLOCATOR, ProcessContext, Process,
    }, config::{PAGE_SIZE, TRAPFRAME, TRAMPOLINE, KERNEL_STACK_SIZE}, trap::{TrapFrame, user_loop},
};

use super::raw_ptr::{UserPtr, Out};

struct YieldFuture(bool);

impl Future for YieldFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl Thread{
/// task exits and submit an exit code
	pub unsafe fn sys_exit(&self,exit_code: i32)->isize{
		let proc = &mut self.proc.inner.lock();
		proc.state = ProcessState::ZOMBIE;
		proc.exit_code = exit_code as isize;
		if let Some(nuclear)=proc.parent.as_ref(){
			let mut x=nuclear.inner.lock();
			x.children.turn_into_zombie(proc.pid);
		}else{
			println!("init exited.");
		}
		self.inner.exclusive_access().exit=true;
		0
	}

	pub unsafe fn sys_getpid(& self) -> isize {
		self.proc.pid as isize
	}
	pub unsafe fn sys_getppid(&self) -> isize {
		self.proc.inner.lock().parent.as_ref().unwrap().pid as isize
	}

	pub unsafe fn sys_clone(&self, stack: usize) -> isize {
		let mut pcb = self.proc.inner.lock();
		let mut pcb =pcb.deref_mut();
		let pid=pcb.pid;
		let new_pid= PID_ALLOCATOR.alloc_pid();

		let mut new_pcb=PCB::new();
		new_pcb.parent=Some(self.proc.clone());
		new_pcb.fd_manager=pcb.fd_manager.clone();
		new_pcb.memory_set=MemorySet::from_existed_user(&pcb.memory_set);
		new_pcb.heap_pos = VirtAddr::from(pcb.memory_set.get_areas_end());
		new_pcb.heap_pos.0 += PAGE_SIZE;
		new_pcb.trapframe_ppn = new_pcb
			.memory_set
			.translate(VirtAddr::from(TRAPFRAME).into())
			.unwrap()
			.ppn();
		*(new_pcb.trapframe_ppn.get_mut() as *mut TrapFrame) = *(pcb.trapframe_ppn.get_mut() as *mut TrapFrame);
		(*(new_pcb.trapframe_ppn.get_mut() as *mut TrapFrame)).x[10] = 0;
		(*(new_pcb.trapframe_ppn.get_mut() as *mut TrapFrame)).kernel_sp =
			TRAMPOLINE - KERNEL_STACK_SIZE * new_pid;
		KERNEL_SPACE.lock().insert_framed_area(
			(TRAMPOLINE - KERNEL_STACK_SIZE * (new_pid + 1)).into(),
			(TRAMPOLINE - KERNEL_STACK_SIZE * new_pid).into(),
			MapPermission::R | MapPermission::W,
		);
		if (stack != 0) {
			(*(new_pcb.trapframe_ppn.get_mut() as *mut TrapFrame)).x[2] = stack;
		}
		
		new_pcb.context = pcb.context;
		new_pcb.context.ra = user_loop as usize;
		new_pcb.context.sp = TRAMPOLINE - KERNEL_STACK_SIZE * new_pid;
		new_pcb.state = ProcessState::READY;
		new_pcb.pid = new_pid;
		
		let new_proc=Arc::new(Process::new(new_pcb));
		pcb.children.alive.insert(new_pid, new_proc.clone());
		
		let (r,t)=async_task::spawn(user_loop(Arc::new(Thread::new(new_proc.clone()))), |runnable|{TASK_QUEUE.push(runnable);});
		r.schedule();
		t.detach();
		return new_pid as isize;
	}

	pub unsafe fn sys_exec(& self,buf: *mut u8, argv: usize) -> isize {
		let pcb=self.proc.inner.lock();
		let path = translate_str(
				pcb
				.memory_set
				.token(),
			buf,
		);
		drop(pcb);
		extern "C" {
			fn _app_num();
		}
		let num = (_app_num as usize as *const usize).read_volatile();
		let range = ((0..num).find(|&i| APP_NAMES[i] == path).map(get_location));
		if (range == None) {
			println!("{} : not found.", path);
			self.sys_exit(-1);
			return 1;
		}

		let (start, end) = range.unwrap();

		let elf_file: Result<ElfFile, &str> =
			ElfFile::new(slice::from_raw_parts(start as *const u8, end - start));
		match elf_file {
			Ok(elf) => self.exec_from_elf(&elf, argv),
			Err(e) => 1,
		}
	}

	pub async fn async_yield(){
		YieldFuture(false).await
	}

	pub async unsafe fn sys_waitpid(&self, pid: isize, status:UserPtr<isize,Out>, options: usize) -> isize {
		let mut pcb_lock=self.proc.inner.lock();
		let mut pcb=pcb_lock.deref_mut();
		let nowpid = pcb.pid;
		if (pid == -1) {
			loop {
				let pid={
					let mut children= &mut pcb.children.zombie;
					self.proc.inner.force_unlock();
					while children.is_empty() {
						Thread::async_yield().await;
					}
					let mut pcb_lock = self.proc.inner.lock();
					let (pid,process) = children.first_key_value().unwrap();
					if (status.as_usize() as usize != 0) {
						let status=status.raw_ptr_mut();
						*status = (process.inner.lock().exit_code << 8) | (0);
					}
					*pid
				};
				let mut children= &mut pcb.children.zombie;
				children.remove_entry(&pid);
				return pid as isize;
			}
		} else {
			let mut children= &mut pcb.children.zombie;
			if let Some(process) = children.get(&(pid as usize)){
				if (status.as_usize() as usize != 0) {
					let status=status.raw_ptr_mut();
					*status = (process.inner.lock().exit_code << 8) | (0);
				}
				children.remove(&(pid as usize) );
			}else{
				return -1;
			}
		}
		0
	}
}


lazy_static! {
    ///All of app's name
    static ref APP_NAMES: Vec<&'static str> = unsafe{
        extern "C" {
            fn _app_num();
            fn _app_names();
        }
        let num_app = (_app_num as usize as *const usize).read_volatile();
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        for _ in 0..num_app {
            let mut end = start;
            while end.read_volatile() != b'\0' {
                end = end.add(1);
            }
            let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
            let str = core::str::from_utf8(slice).unwrap();
            v.push(str);
            start = end.add(1);
        }
        v
    };
}

fn get_location(id: usize) -> (usize, usize) {
    extern "C" {
        fn _app_num();
    }
    unsafe {
        let start = (_app_num as usize as *const usize)
            .add(id + 1)
            .read_volatile();
        let end = (_app_num as usize as *const usize)
            .add(id + 2)
            .read_volatile();
        (start, end)
    }
}
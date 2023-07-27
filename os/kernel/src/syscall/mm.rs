use core::{ops::DerefMut, panic, arch::asm};

use alloc::{task, borrow::ToOwned, vec::Vec};
use lazy_static::__Deref;
use riscv::register::mstatus;

use crate::{
    config::{PAGE_SIZE, PAGE_SIZE_BITS, PRINT_SYSCALL},
    mm::{
        memory_set::{MapArea, MapType},
        MapPermission, VirtAddr, VirtPageNum, page_table::PageTable,
    },
    task::{Thread, FdManager, OpenFile},
};

impl Thread{
	pub fn sys_brk(&self, _brk: usize) -> isize {
		let mut pcb=self.proc.inner.lock();
		let end_: usize = pcb.heap_pos.into();

		if (_brk == 0) {
			return end_ as isize;
		}

		if (end_ == _brk) {
			0
		} else if (end_ < _brk) {
			pcb.memory_set.push(
				MapArea::new(
					end_.into(),
					_brk.into(),
					MapType::Framed,
					MapPermission::R | MapPermission::W | MapPermission::U,
				),None);
			pcb.heap_pos.0 = _brk;
			return _brk as isize;
		} else {
			panic!("shrink.");
			// need to change
			let mset = &mut pcb.memory_set;
			let flag = mset.shrink_to(
				VirtAddr::from(
					mset.areas
						.get(mset.areas.len() - 2)
						.unwrap()
						.vpn_range
						.get_start(),
				),
				VirtAddr::from(_brk),
			);
			if flag {
				return 0;
			} else {
				return -1;
			}
		}
	}

	pub fn sys_mmap(&self, start: usize, len: usize, prot: i32, flag: i32, fd: usize, off: usize) -> isize {
		if PRINT_SYSCALL {println!("[mmap] start={:#x},len={:#x},fd={}",start,len,fd as isize);}
		let mut pcb = self.proc.inner.lock();
		let mut pcb=pcb.deref_mut();
		
		let startva = if start == 0 {
			pcb.mmap_pos.ceil_align().0
			// pcb.heap_pos.ceil_align().0
		} else {
			start
		};
		// let x=PageTable::from_token(pcb.memory_set.token());
		// for area in &pcb.memory_set.areas{
		// 	let a:usize=area.vpn_range.get_start().into();
		// 	let b:usize=area.vpn_range.get_end().into();
		// 	print!("[{:#x},{:#x}]",a,b);
		// }
		if PRINT_SYSCALL {println!("[mmap] startva={:#x}",startva);}
		
		if fd==usize::MAX {
			if(start>0 &&start<=pcb.heap_pos.ceil_align().0){
				return startva as isize;
			}
			// let len=len.max(PAGE_SIZE);
			println!("{:#x},{:#x},",startva,startva+len);
			pcb.memory_set.push(
				MapArea::new(
					startva.into(),
					(startva + len).into(),
					MapType::Framed,
					MapPermission::R | MapPermission::W | MapPermission::U,
				),None
			);
			// pcb.heap_pos=(startva+len).into();
			pcb.mmap_pos=(startva+len).into();
			// pcb.mmap_pos=(pcb.mmap_pos.ceil_align().0).into();
		}else{
			pcb.memory_set.push(
				MapArea::new(
					startva.into(),
					(startva + len).into(),
					MapType::Framed,
					MapPermission::R | MapPermission::W | MapPermission::U,
				),
				Some(
					pcb.fd_manager.fd_array[fd]
						.lock()
						.inode
						.lock()
						.file_data()
						.as_slice()
					)
				);
				pcb.mmap_pos=(startva+len).into();
				// pcb.mmap_pos=(pcb.mmap_pos.ceil_align().0).into();
				// pcb.heap_pos=(startva+len).into();
			}
			// println!("{:#x},{:#x}",startva,pcb.heap_pos.0);
			return startva as isize;
		}
		
		pub fn sys_munmap(start: *mut usize, len: usize) -> isize {
			return 0;
		}
		pub fn sys_fcntl(&self, fd: usize, cmd: usize, arg :usize) -> isize{
			if PRINT_SYSCALL{
				println!("[fcntl] fd:{} cmd:{}",fd,cmd);
			}
			let mut pcb=self.proc.inner.lock();
			let fd_manager=&mut pcb.fd_manager;
			match cmd {
				2=>{
					fd_manager.fd_array[fd].lock().set_close_on_exec((arg &1)!=0)
				}
				//DUPFD_CLOEXEC
				1030=>{
					fd_manager.fd_array[fd].lock().set_close_on_exec((arg &1)!=0);
					return fd_manager.dup(fd) as isize;
				}
				_=>{
					return 0;
				}
			}
		}
		pub fn sys_lseek(&self, fd: usize, offset: usize, whence :usize) -> isize{
			if PRINT_SYSCALL{
				println!("[lseek] fd:{} offset:{} whence:{}",fd,offset,whence);
			}
			let mut pcb=self.proc.inner.lock();
			let fd_manager=&mut pcb.fd_manager;
			let mut open_file=&fd_manager.fd_array[fd];
			match whence{
				//SEEK_SET
				0 => {
					open_file.lock().offset=offset;
					offset as isize
				},
				//SEEK_CUR
				1 =>{
					open_file.lock().offset+=offset;
					open_file.lock().offset as isize
				},
				//SEK_END
				2=>{
					let len=open_file.lock().inode.lock().file_size();
					open_file.lock().offset=len+offset;
					(len+offset) as isize
				}
				_=> -1
			}
		}
}
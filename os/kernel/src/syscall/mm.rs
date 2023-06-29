use core::{ops::DerefMut, panic};

use alloc::task;
use lazy_static::__Deref;
use riscv::register::mstatus;

use crate::{
    config::{PAGE_SIZE, PAGE_SIZE_BITS},
    mm::{
        memory_set::{MapArea, MapType},
        MapPermission, VirtAddr, VirtPageNum,
    },
    task::{Thread},
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
		let mut pcb = self.proc.inner.lock();
		let mut pcb=pcb.deref_mut();
		let startva = if start == 0 {
			pcb.heap_pos
		} else {
			start.into()
		}
		.0;

		pcb.memory_set.push(
			MapArea::new(
				startva.into(),
				(startva + len).into(),
				MapType::Framed,
				MapPermission::R | MapPermission::W | MapPermission::U,
			),
			Some(
				pcb.fd_manager.fd_array[fd]
					.open_file
					.lock()
					.inode
					.lock()
					.file_data()
					.as_slice(),
			),
		);
		return startva as isize;
	}

	pub fn sys_munmap(start: *mut usize, len: usize) -> isize {
		return 0;
	}
}
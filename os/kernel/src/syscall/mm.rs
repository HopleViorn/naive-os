use core::ops::DerefMut;

use alloc::task;
use lazy_static::__Deref;

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
		println!("brk:{:#x}",_brk);
		let mut pcb=self.proc.inner.lock();
		let end_: usize = pcb.heap_pos.into();
		// let old_end = usize::from(VirtAddr::from(end_)) + PAGE_SIZE - 1;
		// //let new_end = ((_brk - 1 + PAGE_SIZE) / PAGE_SIZE) << PAGE_SIZE_BITS;

		if (_brk == 0) {
			return end_ as isize;
		}

		if (end_ == _brk) {
			0
		} else if (end_ < _brk) {
			let mset = &mut pcb.memory_set;
			if (_brk < usize::from(VirtAddr::from(mset.get_areas_end())) + PAGE_SIZE) {
				pcb.heap_pos.0 = _brk;
				return 0;
			} else {
				let flag = mset.append_to(
					VirtAddr::from(
						mset.areas
							.get(mset.areas.len() - 2)
							.unwrap()
							.vpn_range
							.get_start(),
					),
					VirtAddr::from(_brk),
				);
				// for v in mset.areas.iter(){
				//     println!("{} {}", usize::from(v.vpn_range.get_start()), usize::from(v.vpn_range.get_end()));
				// }
				if flag {
					pcb.heap_pos.0 = _brk;
					return 0;
				} else {
					return -1;
				}
			}
		} else {
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
use core::str::from_utf8;

use crate::{
    mm::{page_table::PageTable, VirtAddr},
    task::{Thread},
    timer::{get_time_s, get_time_us, set_next_trigger},
};

use super::{ timespec};
#[repr(C)]
#[derive(Debug)]
pub struct utsname {
	pub sysname: [u8; 65],
	pub nodename: [u8; 65],
	pub release: [u8; 65],
	pub version: [u8; 65],
	pub machine: [u8; 65],
	pub domainname: [u8; 65],
}

impl utsname {
	pub fn new() -> Self {
		let sysname = b"MoOS\0";
		let nodename = b"localhost\0";
		let release = b"0.1.0\0";
		let version = b"0.1.0\0";
		let machine = b"riscv64\0";
		let domainname = b"localhost\0";

		let mut sysname_arr = [0; 65];
		let mut nodename_arr = [0; 65];
		let mut release_arr = [0; 65];
		let mut version_arr = [0; 65];
		let mut machine_arr = [0; 65];
		let mut domainname_arr = [0; 65];

		sysname_arr[..sysname.len()].copy_from_slice(sysname);
		nodename_arr[..nodename.len()].copy_from_slice(nodename);
		release_arr[..release.len()].copy_from_slice(release);
		version_arr[..version.len()].copy_from_slice(version);
		machine_arr[..machine.len()].copy_from_slice(machine);
		domainname_arr[..domainname.len()].copy_from_slice(domainname);

		utsname {
			sysname: sysname_arr,
			nodename: nodename_arr,
			release: release_arr,
			version: version_arr,
			machine: machine_arr,
			domainname: domainname_arr,
		}
	}
}

impl Thread{
	pub unsafe fn sys_gettimeofday(&self,ptr: *mut usize) -> isize {
		let t: *mut usize = PageTable::from_token(
			self.proc.inner.lock()
				.memory_set
				.token(),
		)
		.translate_va(VirtAddr::from(ptr as usize))
		.unwrap()
		.get_mut();
		let ts = get_time_us();
		*t = ts / 1000000;
		*(t.add(1)) = ts % 1000000;
		return 0;
	}
	pub async unsafe fn sys_nanosleep(req: usize, rem: usize) -> isize {
		let ed={
			let req=req as *mut timespec;
			let rem=rem as *mut timespec;
			let st = get_time_us();
			let ed = st + (*req).tv_sec * 1000000 + (*req).tv_nsec;
			ed
		};

		while get_time_us() < ed {
			Thread::async_yield().await;
		}
		return 0;
	}

	pub unsafe fn sys_times(&self,tms_addr: usize) -> isize {
		let mut tms = tms_addr as *mut usize;
		let pcb=self.proc.inner.lock();
		*tms.add(0) = pcb.utime;
		*tms.add(1) = pcb.ktime;
		*tms.add(2) = 0;
		*tms.add(3) = 0;
		0
	}

	// struct utsname {
	// 	char sysname[65];
	// 	char nodename[65];
	// 	char release[65];
	// 	char version[65];
	// 	char machine[65];
	// 	char domainname[65];
	// };

	pub fn sys_uname(_name: *mut u8) -> isize {
		// println!("sys_uname: _name = {:x}", _name as usize);
		// fill the utsname with valid string
		let name = unsafe { &mut *(_name as *mut utsname) };

		let new_utsname = utsname::new();
		*name = new_utsname;

		// println!("sys_uname: name = {:?}", name);
		// println!("sys_uname: name.sysname = {:?}", name.sysname);
		// println!("sys_uname: name.nodename = {:?}", name.nodename);
		// println!("sys_uname: name.release = {:?}", name.release);
		// println!("sys_uname: name.version = {:?}", name.version);
		// println!("sys_uname: name.machine = {:?}", name.machine);
		// println!("sys_uname: name.domainname = {:?}", name.domainname);
		// println!(
		//     "Uname: {} {} {} {} {} {}",
		//     from_utf8(&name.sysname).unwrap().trim_end_matches('\0'),
		//     from_utf8(&name.nodename).unwrap().trim_end_matches('\0'),
		//     from_utf8(&name.release).unwrap().trim_end_matches('\0'),
		//     from_utf8(&name.version).unwrap().trim_end_matches('\0'),
		//     from_utf8(&name.machine).unwrap().trim_end_matches('\0'),
		//     from_utf8(&name.domainname).unwrap().trim_end_matches('\0')
		// );
		0
	}
}
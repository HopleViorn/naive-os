use crate::{timer::{get_time_s, get_time_us, set_next_trigger}, task::{task_list, cpu::mycpu}, mm::{page_table::PageTable, VirtAddr}};

use super::{timespec, process::sys_yield};

pub unsafe fn sys_gettimeofday(ptr: *mut usize)->isize{
	let t:*mut usize=PageTable::from_token(task_list.exclusive_access()[mycpu().proc_idx].memory_set.token()).translate_va(VirtAddr::from(ptr as usize)).unwrap().get_mut();
	let ts=get_time_us();
	*t=ts/1000000;
	*(t.add(1))=ts%1000000;
	set_next_trigger();
	return 0;
}
pub unsafe fn sys_nanosleep(req: *mut timespec,rem: *mut timespec)->isize{
	let st=get_time_us();
	let ed=st+(*req).tv_sec*1000000+(*req).tv_nsec;
	while get_time_us()<ed {
		sys_yield();
	}
	return 0;
}
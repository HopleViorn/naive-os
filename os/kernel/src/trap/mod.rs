mod context;

use crate::config::TRAMPOLINE;
use crate::mm::VirtAddr;
use crate::mm::page_table::PageTable;
use crate::task::{ProcessContext, PCB};
use crate::task::Thread;
use crate::{
    config::TRAPFRAME,
    task::{ proc::*},
    timer::{get_time_ms, set_next_trigger},
};
use core::arch::asm;
use core::arch::global_asm;
use core::task::Context;
use alloc::sync::Arc;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    stval, stvec,
};

global_asm!(include_str!("trampoline.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_from_kernel() ->! {
    // println!("Kernel trap");
    // let scause = scause::read();
    // let stval = stval::read();
    // println!("stval= {:#x}",stval);
    panic!("kernel trap");
}

#[no_mangle]
pub fn set_kernel_trap() {
	unsafe{
		stvec::write(trap_from_kernel as usize, TrapMode::Direct);
		asm!("fence");
	}
}
#[no_mangle]
pub fn set_user_trap() {
	unsafe{
		stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
	}
}

#[no_mangle]
pub async unsafe fn user_loop(thread: Arc<Thread>){
	{
		let mut pcb=thread.proc.inner.lock();
		println!("[New Thread] pid={},spec={:#x},sp={:#x}",pcb.pid,(*(pcb.trapframe_ppn.get_mut() as *mut TrapFrame)).sepc,(*(pcb.trapframe_ppn.get_mut() as *mut TrapFrame)).x[2]);
	}
	
	loop{
		let user_satp={
			let mut pcb=thread.proc.inner.lock();
			set_user_trap();
			pcb.ktime +=get_time_ms() - pcb.otime;
			pcb.otime = get_time_ms();
			
			let trapframe_ptr = TRAPFRAME;
			pcb.memory_set.token()
			
		};
		extern "C" {
			fn __alltraps();
			fn __restore();
		}
		let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
		let mut cx=ProcessContext::new();
		
		asm!(
			"fence.i",
			"jalr {restore_va}",             // jump to new addr of __restore asm function
			restore_va = in(reg) restore_va,
			in("a0") &mut cx,      // a0 = virt addr of Trap Context
			in("a1") user_satp,        // a1 = phy addr of usr page table
		);
		
		
		set_kernel_trap();
		{
			let mut pcb=thread.proc.inner.lock();
			// println!("USER TRAP: stval={:#x}",stval);
			// println!("spec={:#x}",(*(pcb.trapframe_ppn.get_mut() as *mut TrapFrame)).sepc);
			pcb.utime+=get_time_ms()-pcb.otime;
			pcb.otime=get_time_ms();
		}
		let stval = stval::read(); // get extra value
		let scause = scause::read(); // get trap cause
		
		match scause.cause() {
			Trap::Exception(Exception::UserEnvCall) => {
				let mut cx={
					let mut pcb=thread.proc.inner.lock();
					let mut cx: &mut TrapFrame = pcb
					.trapframe_ppn
					.get_mut();
					cx.sepc += 4;
					println!("sepc={:#x}",cx.sepc);
					cx
				};
			// println!("[syscall] id= {}",cx.x[17]);
			let result = thread.syscall(
				cx.x[17],
					[cx.x[10], cx.x[11], cx.x[12], cx.x[13], cx.x[14], cx.x[15]],
				).await;
				{
					let mut pcb=thread.proc.inner.lock();
					cx = pcb
						.trapframe_ppn
						.get_mut();
					cx.x[10] = result as usize;
				}
				if(thread.inner.exclusive_access().exit){
					break;
				}
			}
			Trap::Exception(Exception::StoreFault)
			| Trap::Exception(Exception::StorePageFault)
			| Trap::Exception(Exception::LoadFault)
			| Trap::Exception(Exception::LoadPageFault) => {
				let mut pcb=thread.proc.inner.lock();
				let cx: &mut TrapFrame = pcb
					.trapframe_ppn
					.get_mut();
				println!("USER TRAP: stval={:#x},pc={:#x}", stval, cx.sepc);
				println!("[kernel] PageFault in application, kernel killed it.");
				match scause.cause() {
					Trap::Exception(Exception::StoreFault) => {
						println!("StoreFault");
					}
					Trap::Exception(Exception::StorePageFault) => {
						println!("StorePageFault");
					}
					Trap::Exception(Exception::LoadFault) => {
						println!("LoadFault");
					}
					Trap::Exception(Exception::LoadPageFault) => {
						println!("LoadPageFault");
					}
					_ => {}
				}
				panic!("pg ft.");
				// kill();
			}
			Trap::Exception(Exception::IllegalInstruction) => {
				println!("[kernel] IllegalInstruction in application, kernel killed it.");
			}
			Trap::Interrupt(Interrupt::SupervisorTimer) => {
				set_next_trigger();
				// sys_yield();
			}
			_ => {
			let mut pcb=thread.proc.inner.lock();
			let cx: &mut TrapFrame = pcb
				.trapframe_ppn
				.get_mut();
			panic!(
				"Unsupported trap {:?}, stval = {:#x},sepc = {:#x},pid = {}",
				scause.cause(),
				stval,
				cx.sepc,
				pcb.pid
			);
		}
    }
}

}

pub use context::TrapFrame;

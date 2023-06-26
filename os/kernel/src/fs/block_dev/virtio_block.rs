use core::{arch::asm, panic, slice::Chunks};

use alloc::{vec::Vec, format};
use lazy_static::lazy_static;
use spin::Mutex;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

use crate::{ mm::{PhysAddr, frame_alloc, VirtAddr, PhysPageNum, FrameTracker, StepByOne, KERNEL_SPACE, page_table::PageTable}};
use crate::mm::frame_allocator::frame_dealloc;

use super::{block_device::BlockDevice, BLOCK_DEVICE};


#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

pub struct VirtIOBlock(Mutex<VirtIOBlk<'static>>);

lazy_static! {
    static ref QUEUE_FRAMES: Mutex<Vec<FrameTracker>> = Mutex::new(Vec::new());
}


impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect(&format!("Error when reading VirtIOBlk,block_id:{:#x}", block_id));
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

#[derive(Copy,Clone)]
pub struct Nulcear;

impl ::block_device::BlockDevice for Nulcear{
	type Error = usize;
	fn read(
        &self,
        buf: &mut [u8],
        address: usize,
        number_of_blocks: usize,
    ) -> Result<(), Self::Error>{
		let mut i=0;
		for chunk in buf.chunks_mut(512){
			let mut tmp=[0;512];
			BLOCK_DEVICE.read_block(address/512+i,&mut tmp);
			chunk.copy_from_slice(&tmp[..chunk.len()]);
			i+=1;
		}
		Ok(())
	}   
	fn write(&self, buf: &[u8], address: usize, number_of_blocks: usize)
        -> Result<(), Self::Error>{
			let mut i=0;
		for chunk in buf.chunks(512){
			BLOCK_DEVICE.write_block(address/512+i,chunk);
			i+=1;
		}
			Ok(())
		}
}


impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            Self(Mutex::new(
                VirtIOBlk::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            ))
        }
    }
}
/// 这里用 new_contiguous 可以要求分配一段连续的内存。
/// 因为这个函数是面向 virtio-drivers 的接口，而且仅在内核启动时初始化，
/// 所以这里默认可以拿到需要的空间，不处理分配失败导致的异常
#[no_mangle]
pub extern "C" fn virtio_dma_alloc(pages: usize) -> PhysAddr {
    let mut ppn_base = PhysPageNum(0);
    for i in 0..pages {
        let frame = frame_alloc().unwrap();
        if i == 0 { ppn_base = frame.ppn; }
        assert_eq!(frame.ppn.0, ppn_base.0 + i);
        QUEUE_FRAMES.lock().push(frame);
    }
    ppn_base.into()
}

#[no_mangle]
pub extern "C" fn virtio_dma_dealloc(pa: PhysAddr, pages: usize) -> i32 {
    let mut ppn_base: PhysPageNum = pa.into();
    for _ in 0..pages {
        frame_dealloc(ppn_base);
        ppn_base.step();
    }
    0
}

#[no_mangle]
pub extern "C" fn virtio_phys_to_virt(paddr: PhysAddr) -> VirtAddr {
    VirtAddr(paddr.0)
}

#[no_mangle]
pub extern "C" fn virtio_virt_to_phys(vaddr: VirtAddr) -> PhysAddr {
	PageTable::from_token(KERNEL_SPACE.exclusive_access().token()).translate_va(vaddr).unwrap()
}
use core::ops::DerefMut;

use alloc::{sync::Arc, string::{String, ToString}};
use fat32::{volume::Volume, dir::Dir};
use lazy_static::__Deref;
use spin::Mutex;

mod block_device;
pub mod virtio_block;
use crate::fs::block_dev::virtio_block::{VirtIOBlock, Nuclear};

use ::block_device::BlockDevice;

lazy_static::lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<VirtIOBlock>= Arc::new(VirtIOBlock::new());
}

lazy_static::lazy_static!{
	pub static ref buf:Arc<Mutex<[u8;512<<8]>>=Arc::new(Mutex::new([0;512<<8]));
}

pub fn init_block_dev(){
	let x=BLOCK_DEVICE.clone();
}

#[allow(unused)]
pub fn block_device_test() {
	let x=Volume::new(Nuclear {});
	let root=x.root_dir();

	for item in root.iter(){
		if let Some(short)=item.sfn{
			let (name,len)=short.get_full_name_bytes();
			let name = core::str::from_utf8(&name).unwrap().to_string();
			println!("[name] {}",&name);
			let file=root.open_file(&name[..len]).unwrap();
			
			let mut xxx=buf.lock();
			let mut xxx=xxx.deref_mut();
			file.read(xxx).unwrap();
			let mut nxx=0;
			let mut tt=0;
			for c in xxx{
				print!("{:02x}",c);
				nxx+=1;
				if nxx==16{
					nxx=0;
					println!("");
				}
				tt+=1;
				if(tt==16*16){
					break;
				}
			}			
		}
		else if let Some(long)=item.lfn{

		}
	}
    println!("block device test passed!");
}

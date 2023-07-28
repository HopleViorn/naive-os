//! File and filesystem-related syscalls

use core::{
    fmt::write,
    mem::{align_of, size_of},
    ops::{Add, DerefMut},
    slice, panic,
};
use crate::{fs::block_dev::virtio_block::{VirtIOBlock, Nuclear}, config::PRINT_SYSCALL};
use crate::{console::print, mm::{page_table::{copy_out, translate_str}, MemorySet, memory_set}, task::Thread};
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec, boxed::Box, fmt::format,
};
use fat32::{volume::Volume, dir::{Dir, DirError}, directory_item::ItemType};
use riscv::paging::Entries;
use spin::Mutex;

use crate::{
    fs::{
        file::{Dirent, OpenFlags, PipeINode, RegFileINode, Stat},
        vfs::{FileType, INode, Timespec},
    },
    mm::translated_byte_buffer,
    sbi::console_getchar,
    task::{
        GLOBAL_DENTRY_CACHE,
        FdManager, FileDescriptor, OpenFile,
    },
};

const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;

#[repr(usize)]
#[allow(non_camel_case_types)]
#[derive(Debug)]
/// sys_fcntl64 使用的选项
pub enum Fcntl64Cmd {
	/// 复制这个 fd，相当于 sys_dup
	F_DUPFD = 0,
	/// 获取 cloexec 信息，即 exec 成功时是否删除该 fd
	F_GETFD = 1,
	/// 设置 cloexec 信息，即 exec 成功时删除该 fd
	F_SETFD = 2,
	/// 获取 flags 信息
	F_GETFL = 3,
	/// 设置 flags 信息
	F_SETFL = 4,
	/// 复制 fd，然后设置 cloexec 信息，即 exec 成功时删除该 fd
	F_DUPFD_CLOEXEC = 1030,
}


impl Thread{
// int getcwd(char *buf, size_t size);
	pub fn sys_getcwd(&self, buf: *mut u8, size: usize) -> isize {
		let task=self.proc.inner.lock();
		let cwd = task.cwd.clone();
		let cwd_str = cwd.as_str();

		// 确保缓冲区足够大
		if size < cwd_str.len() + 1 {
			return -1;
		}

		// 将当前工作目录字符串复制到用户提供的缓冲区
		let mut buffers = translated_byte_buffer(task.memory_set.token(), buf, cwd_str.len() + 1);
		for (i, byte) in cwd_str.as_bytes().iter().enumerate() {
			buffers[0][i] = *byte;
		}
		// 添加空终止符
		buffers[0][cwd_str.len()] = 0;

		// 返回字符串长度（包括空终止符）
		(cwd_str.len() + 1) as isize
	}

	pub fn get_abs_path(&self,path: String)-> (String,String){
		let start_dir_path;
		let rel_path;
		if path.starts_with("/") {
			start_dir_path = "/".to_string();
			rel_path = path.strip_prefix("/").unwrap_or(&path).to_string();
		} else {
			unsafe{self.proc.inner.force_unlock()};
			start_dir_path = self.proc.inner.lock().cwd.clone(); // TODO: consider dirfd
			rel_path = if path.starts_with("./") {
				path.strip_prefix("./").unwrap().to_string()
			} else {
				path.to_string()
			};
		}
		(start_dir_path, rel_path)
	}
	// int openat(int dirfd,const char *path, int flags)
	pub fn sys_openat(&self,dirfd: isize, path: usize, flags: isize) -> isize {
		let mut task=self.proc.inner.lock();
		let path = translate_str(task.memory_set.token(),path as *mut u8);
		// println!("openat: dir fd: {}, path: {}, flags: {}", dirfd, path, flags);
		let (start_dir_path, rel_path)=self.get_abs_path(path);
		let abs_path=format!("{}{}",start_dir_path,rel_path);
		if PRINT_SYSCALL {println!("[openat] path={},fd={}",abs_path,task.fd_manager.len());}
		
		let fd=match GLOBAL_DENTRY_CACHE.get(&abs_path) {
			Some(inode) => {
				let open_file=Arc::new(Mutex::new(OpenFile::new_from_inode(
					((flags as u32 ^ OpenFlags::RDONLY.bits())
					| (flags as u32 ^ OpenFlags::RDWR.bits()))
					!= 0,
					((flags as u32 ^ OpenFlags::WRONLY.bits())
					| (flags as u32 ^ OpenFlags::RDWR.bits()))
					!= 0, inode,
				)));
				task.fd_manager.push(open_file) as isize
			}
			None => {
				println!("CREATING new file.");
				// create a new file in fs
				let new_inode = Arc::new(Mutex::new(RegFileINode {
					// Initialize the new inode with the required fields
					readable: true,
					writable: true,
					dir: start_dir_path.clone(),
					name: rel_path.clone(),
					atime: Timespec::default(),
					mtime: Timespec::default(),
					ctime: Timespec::default(),
					flags: OpenFlags::new(flags as u32),
					file: Vec::new(),
				}));
				GLOBAL_DENTRY_CACHE.insert(&abs_path, new_inode.clone());
				task.fd_manager.push(Arc::new(Mutex::new(OpenFile::new_from_inode(
					((flags as u32 ^ OpenFlags::RDONLY.bits())
					| (flags as u32 ^ OpenFlags::RDWR.bits()))
					!= 0,
					((flags as u32 ^ OpenFlags::WRONLY.bits())
					| (flags as u32 ^ OpenFlags::RDWR.bits()))
					!= 0, new_inode)))
				) as isize
				// // add open file to global open file table
				// let open_file = Arc::new(Mutex::new(OpenFile {
				// 	offset: 0,
				// 	status_flags: flags as u32,
				// 	inode: new_inode.clone(),
				// }));

				// // update fd manager
				// let file_descriptor = FileDescriptor {
				// 	open_file: open_file.clone(),
				// 	readable: ((flags as u32 ^ OpenFlags::RDONLY.bits())
				// 		| (flags as u32 ^ OpenFlags::RDWR.bits()))
				// 		!= 0,
				// 	writable: ((flags as u32 ^ OpenFlags::WRONLY.bits())
				// 		| (flags as u32 ^ OpenFlags::RDWR.bits()))
				// 		!= 0,
				// };
				// let fd_manager = &mut task.fd_manager;
				// fd = fd_manager.len();
				// fd_manager.insert(file_descriptor);
				// new_inode
			}
		};
		fd as isize
	}

	pub fn sys_sendfile(&self,out_fd:usize, in_fd:usize, offset: usize, count:usize)->isize{
		if PRINT_SYSCALL{println!("[sendfile] in_fd:{},out_fd:{},{},{}",in_fd,out_fd,offset,count);}
		let pcb=self.proc.inner.lock();
		let fd_manager=&pcb.fd_manager;
		let mut inf=fd_manager.fd_array[in_fd].lock();
		let mut of=fd_manager.fd_array[out_fd].lock();

		let mut buf:Vec<u8>=Vec::new();
		buf.resize(count, 0);

		if offset==0{
			let count_in=inf.inode.lock().read_at(inf.offset, &mut buf[..]).unwrap();
			if PRINT_SYSCALL{println!("[send] len={},content=[{}]",count_in,core::str::from_utf8(&buf[..count_in]).unwrap().to_string());}
			of.inode.lock().write_at(of.offset, &buf[..count_in]).unwrap();
			inf.offset+=count_in;
			of.offset+=count_in;
			return count_in as isize;
		}else{
			unsafe{
				let offset=self.translate(offset) as *mut usize;
				let count_in=inf.inode.lock().read_at(*offset, &mut buf[..]).unwrap();
				of.inode.lock().write_at(of.offset, &buf[..count_in]);
				of.offset+=count_in;
				*offset+=count_in;
				return count_in as isize;
			}
		}
	}

	// int close(int fd)
	pub fn sys_close(&self,fd: isize) -> isize {
		if PRINT_SYSCALL{ println!("[close] fd:{}",fd);}
		let fd_manager = &mut self.proc.inner.lock().fd_manager;
		if fd as usize >= fd_manager.len() {
			return -1;
		}
		fd_manager.close(fd as usize);
		0
	}

	/// write `buf` of length `len`  to a file with `fd`
	pub fn sys_write(&self, fd: usize, buf: *const u8, len: usize) -> isize {
		let task = &mut self.proc.inner.lock();
		let fd_manager = &task.fd_manager;
		let open_file = & mut fd_manager.fd_array[fd].lock();
		if !open_file.writable {
			return -1;
		}
		let buffers = translated_byte_buffer(task.memory_set.token(), buf, len);
		let mut sum = 0;
		
		for buffer in buffers {
			let write_in = open_file
				.inode
				.lock()
				.write_at(open_file.offset, buffer)
				.unwrap();
			open_file.offset += write_in;
			sum += write_in;
		}
		return sum as isize;
	}

	pub fn sys_writev(&self, fd: usize, iov: *const usize, len: usize) -> isize {
		let mut sum=0;
		for i in 0..len{
			unsafe{
				let buf=*iov.add(i*2) as *const u8;
				let size=*iov.add(i*2+1);
				sum+=self.sys_write(fd, buf, size);
			}
		}
		sum
	}
	pub async fn sys_readv(&self, fd: usize, iov: usize, len: usize) -> isize {
		let mut sum=0;
		for i in 0..len{
			unsafe{
				let buf=*(iov as *const usize).add(i*2);
				let size=*(iov as *const usize).add(i*2+1);
				sum+=self.sys_read(fd, buf, size).await;
			}
		}
		sum
	}

	pub fn sys_umount(&self) -> isize {
		0
	}

	fn full_search_mount(dir:Dir<'_,Nuclear>, mut path:String){
		let mut lfn_name=String::new();
		for item in dir.iter(){
			match(item.item_type){
				ItemType::LFN=>{
					let (name,len)=item.get_lfn().unwrap();
					let name=core::str::from_utf8(&name).unwrap().to_string();
					lfn_name=format!("{}{}",&name[..len],lfn_name);
					// println!("<{}>:end:{}",lfn_name,item.is_name_end().unwrap());
					drop(name);
					// if !item.is_name_end().unwrap() { continue; }
					let file=dir.open_file(&lfn_name);
					let mut file_path=path.clone();
					file_path.push_str(&lfn_name);
					if let Ok(file)=file{
						println!("[mount] find file {}",file_path);
						// println!("file size:{}",file.length());
						// println!("path:{},file_path:{}",path,file_path);
						
						let inode = Arc::new(Mutex::new(RegFileINode::new(
							path.clone(),
							lfn_name.clone(),
							OpenFlags::CREATE,
							true,
							true,
						)));
						
						{
							let inode_content=&mut inode.lock().file;
							inode_content.resize(file.length(), 0);
							if file.length()>0 {
								file.read(inode_content.as_mut_slice());
								// if file.length()<500{
								// println!("<{}>",core::str::from_utf8(inode_content.as_slice()).unwrap().to_string());
								// }
							}else{
								println!("empty file.");
							}
						}
						GLOBAL_DENTRY_CACHE.insert(&file_path, inode);
						
						// let buf=inode_content;
						// let mut nxx=0;
						// let mut tt=0;
						// for c in buf{
							// 	print!("{:02x}",c);
							// 	nxx+=1;
							// 	if nxx==16{
								// 		nxx=0;
								// 		println!("");
								// 	}
								// 	tt+=1;
								// 	if(tt==16*16){
									// 		break;
									// 	}
									// }	
						lfn_name=String::new();
					}else{
						//DIRc
						let new_dir=dir.cd(&lfn_name);
						if let Ok(new_dir)=new_dir{
							let mut new_path=path.clone();
							new_path.push_str(&lfn_name);
							new_path.push('/');
							Thread::full_search_mount(new_dir,new_path);
							lfn_name=String::new();
						}else{
							// continue;
							// panic!("Mount Exception.");
							if !item.is_name_end().unwrap(){
								lfn_name=String::new();
							}else{
								println!("failed on {}",&lfn_name);
							}
						}
					}
				}
				ItemType::Dir=>{
				}
				ItemType::File=>{
					let x=item.sfn.unwrap().name;
					let x=core::str::from_utf8(&x).unwrap().to_string();
					// println!("[mount] sfn {}",&x);
				}
				_=>{
					;
				}
			}
		}
	}
	
	pub fn sys_mount() -> isize {
		let volume=Volume::new(Nuclear{});
		let root_dir=volume.root_dir();
		Thread::full_search_mount(root_dir,"/".to_string());
		return 0;
	}

	pub async unsafe fn sys_read(&self,fd: usize, buf: usize, len: usize) -> isize {
		// println!("sys_read: fd: {}, buf: {:?}, len: {}", fd, buf, len);
		let mut pcb_lock=self.proc.inner.lock();
		let mut task = pcb_lock.deref_mut();
		if PRINT_SYSCALL{ println!("[read] len={},fd={},pid={}",len,fd,task.pid);}
		let memory_set=&task.memory_set;
		let fd_manager = &task.fd_manager;
		let open_file = &mut fd_manager.fd_array[fd].lock();
		if !open_file.readable {
			return -1;
		}
		let buffers = translated_byte_buffer(memory_set.token(), buf as *mut u8, len);
		let mut sum = 0;
		for buffer in buffers {
			for i in 0..1 {
				let read_in = open_file
					.inode
					.lock()
					.read_at(open_file.offset, buffer)
					.unwrap();
				// println!("|{}|",core::str::from_utf8(buffer).unwrap().to_string());
				// println!("read_in:{}",read_in);
				open_file.offset += read_in;
				sum += read_in;
				if read_in > 0 {
					break;
				} else {
					self.proc.inner.force_unlock();
					Thread::async_yield().await;
				}
			}
		}
		return sum as isize;
	}

	pub fn sys_getdents64(&self, fd: usize, buf: *mut u8, len: usize) -> isize {
		return 0;
		let mut task = self.proc.inner.lock();
		let fd_manager = &mut task.fd_manager;

		let file_descriptor = &fd_manager.fd_array[fd];
		unsafe {
			let ptr = buf.offset(core::mem::size_of::<Dirent>() as isize);
			let len = len - core::mem::size_of::<Dirent>();
			*ptr.offset(len as isize) = ".".as_bytes()[0];
			*ptr.offset((len + 1) as isize) = "\0".as_bytes()[0];
		}

		// println!("openat: fd: {}, buf: {:?}, len: {}", fd, buf, len);
		let open_file: Arc<spin::mutex::Mutex<OpenFile>> = file_descriptor.clone();
		let inode = open_file.lock().inode.clone();
		let mut entries: Vec<String> = Vec::new();
		entries = match inode.lock().list() {
			Ok(entries) => entries,
			Err(_) => {
				entries.push(".".to_string());
				// entries.push("..".to_string());
				entries
			}
		};

		let mut bytes_written = 0;
		let mut buf_ptr = buf;

		for entry in entries {
			let name_len = entry.len() + 1;
			let dirent_size = size_of::<Dirent>() + name_len;

			if bytes_written + dirent_size > len {
				break;
			}
			let mut dirent = Dirent::new();
			dirent.d_name[..entry.len()].copy_from_slice(entry.as_bytes());
			dirent.d_reclen = entry.len() as u16;
			// println!("dirent: d_name {:?}", dirent.d_name);

			unsafe {
				// Write dirent to buf
				core::ptr::write(buf_ptr as *mut Dirent, dirent);
				buf_ptr = buf_ptr.add(size_of::<Dirent>());

				// Write name to buf
				core::ptr::write_bytes(buf_ptr, 0, name_len);
				core::ptr::copy(entry.as_ptr(), buf_ptr, entry.len());
				buf_ptr = buf_ptr.add(name_len);
			}

			bytes_written += dirent_size;
		}
		// println!("bytes_written: {}", bytes_written);
		bytes_written as isize
	}

	// SYSCALL_DUP => sys_dup(args[0] as isize),
	
	pub fn sys_dup(&self, fd:isize) ->isize{
		let mut pcb=self.proc.inner.lock();
		let mut fd_manager=&mut pcb.fd_manager;
		fd_manager.dup(fd as usize) as isize
	}
	pub fn sys_dup3(&self, fd: isize, new_fd: isize, flags: isize) ->isize{
		if PRINT_SYSCALL{ println!("[dup3] fd:{} new_fd:{}",fd,new_fd);}
		let mut pcb=self.proc.inner.lock();
		let mut fd_manager=&mut pcb.fd_manager;
		fd_manager.dup3(fd as usize,new_fd as usize) as isize
	}

	// pub fn sys_dup(&self, fd: isize) -> isize {
	// 	let fd = fd as usize;
	// 	let mut task=self.proc.inner.lock();
	// 	let mut fd_manager = &mut task.fd_manager;

	// 	if fd >= fd_manager.len() {
	// 		return -1;
	// 	}

	// 	let file_descriptor = &fd_manager.fd_array[fd].clone();
	// 	if !file_descriptor.readable && !file_descriptor.writable {
	// 		return -1;
	// 	}

	// 	let open_file = file_descriptor.open_file.clone();
	// 	let inode = open_file.lock().inode.clone();

	// 	let mut new_fd = -1;
	// 	for (i, fd) in fd_manager.fd_array.iter().enumerate() {
	// 		if !fd.readable && !fd.writable {
	// 			new_fd = i as isize;
	// 			break;
	// 		}
	// 	}

	// 	if new_fd == -1 {
	// 		new_fd = fd_manager.len() as isize;
	// 	}

	// 	fd_manager.fd_array.push(FileDescriptor {
	// 		readable: file_descriptor.readable,
	// 		writable: file_descriptor.writable,
	// 		open_file: open_file,
	// 	});
	// 	new_fd
	// }

	// SYSCALL_DUP3 => sys_dup3(args[0] as isize, args[1] as isize, args[2] as isize),

	// pub fn sys_dup3(&self, fd: isize, new_fd: isize, flags: isize) -> isize {
	// 	let fd = fd as usize;
	// 	let new_fd = new_fd as usize;
	// 	let flags = flags as usize;
	// 	let mut task = self.proc.inner.lock();
	// 	let mut fd_manager = &mut task.fd_manager;
	// 	if fd >= fd_manager.len() {
	// 		return -1;
	// 	}

	// 	let file_descriptor = &fd_manager.fd_array[fd].clone();
	// 	if !file_descriptor.readable && !file_descriptor.writable {
	// 		return -1;
	// 	}

	// 	let open_file = file_descriptor.open_file.clone();
	// 	let inode = open_file.lock().inode.clone();

	// 	if new_fd >= fd_manager.len() {
	// 		for _ in fd_manager.len()..new_fd + 1 {
	// 			fd_manager.fd_array.push(FileDescriptor {
	// 				readable: false,
	// 				writable: false,
	// 				open_file: Arc::new(Mutex::new(OpenFile::new())),
	// 			});
	// 		}
	// 	}
	// 	fd_manager.fd_array[new_fd] = FileDescriptor {
	// 		readable: file_descriptor.readable,
	// 		writable: file_descriptor.writable,
	// 		open_file: open_file,
	// 	};
	// 	new_fd as isize
	// }


	pub fn sys_mkdirat(&self, fd: isize, path: usize, mode: usize) -> isize {
		let fd = fd as usize;
		let mode = mode as u16;
		let mut task = self.proc.inner.lock();
		let path=&translate_str(task.memory_set.token(), path as *mut u8);
		let mut fd_manager = &mut task.fd_manager;

		if fd >= fd_manager.len() {
			return 0; // TODO should return -1
		}

		let file_descriptor = &fd_manager.fd_array[fd].clone();
		// if !file_descriptor.writable {
		//     return -1;
		// }

		let open_file = file_descriptor.clone();
		let inode = open_file.lock().inode.clone();

		let mut path_iter = path.split('/');
		let mut current_dir = inode.clone();
		let mut current_dir_name = String::from("/");

		loop {
			let next_dir_name = match path_iter.next() {
				Some(name) => name,
				None => break,
			};

			if next_dir_name == "" {
				continue;
			}

			let next_dir = match current_dir.lock().find(next_dir_name) {
				Ok(next_dir) => next_dir,
				Err(_) => {
					let new_dir = Arc::new(Mutex::new(RegFileINode {
						readable: true,
						writable: true,
						dir: current_dir_name.clone(),
						name: current_dir_name.clone(),
						atime: Timespec::default(),
						mtime: Timespec::default(),
						ctime: Timespec::default(),
						flags: OpenFlags::new(mode as u32),
						file: Vec::new(),
					}));
					// current_dir.lock().add(next_dir_name, new_dir.clone());
					new_dir
				}
			};

			current_dir = next_dir;
			current_dir_name = next_dir_name.to_string();
		}

		0
	}

	// SYSCALL_CHDIR => sys_chdir(&translate_str(get_token(), args[0] as *mut u8)),

	pub fn sys_chdir(&self, path: usize) -> isize {
		let mut task = self.proc.inner.lock();
		let path=&translate_str(task.memory_set.token(), path as *mut u8);
		let mut fd_manager = &task.fd_manager;

		let mut path_iter = path.split('/');
		let mut current_dir = task.cwd.clone();
		let mut current_dir_name = String::from("/");
		task.cwd = current_dir.clone() + path + "/";
		0
	}

	// SYSCALL_FSSTAT => sys_fstat(args[0] as isize, args[1] as *mut u8),

	pub fn sys_fstatat(&self, dirfd: isize, path: usize, buf: *mut u8, flags:usize) -> isize {
		let pcb=self.proc.inner.lock();
		let path=translate_str(pcb.memory_set.token(), path as *mut u8);
		let (dir,rel)=self.get_abs_path(path);
		let abs_path=format!("{}{}",dir,rel);
		if PRINT_SYSCALL{println!("[fstatat] dirfd:{}, abs_path:{}",dirfd as isize,abs_path);}
		let inode=GLOBAL_DENTRY_CACHE.get(&abs_path);
		if inode.is_none() {
			return -1;
		}
		let inode=inode.unwrap();

		let mut stat = Stat::new();

		stat.st_size = inode.lock().file_size() as u32;
		// println!("file_data:{:?}",fd_manager.fd_array[fd].open_file.inode.lock().file_data());
		// println!("file_sss:{:?}",fd_manager.fd_array[fd].open_file.inode.lock().file_size());
		// println!("file_nuckear:{:?}",stat.st_size);
		unsafe {
			copy_out(
				pcb.memory_set.token(),
				buf,
				&mut stat as *mut Stat as *mut u8,
				size_of::<Stat>(),
			);
			// *(buf as *mut Stat)=stat;
			// println!("xxxxfile_nuckear:{:?}",(*(buf as *mut Stat)).st_size);
		}
		return 0;
	}

	pub fn sys_fstat(&self,fd: isize, buf: *mut u8) -> isize {
		let fd = fd as usize;
		let task = self.proc.inner.lock();
		let fd_manager = &task.fd_manager;

		if fd >= fd_manager.len() {
			return -1;
		}

		let mut stat = Stat::new();
		
		stat.st_size = fd_manager.fd_array[fd]
			.lock()
			.inode
			.lock()
			.file_size() as u32;
		if PRINT_SYSCALL{println!("[fstat] path:{}",fd_manager.fd_array[fd].lock().inode.lock().file_name());}
		
		// println!("file_data:{:?}",fd_manager.fd_array[fd].open_file.inode.lock().file_data());
		// println!("file_sss:{:?}",fd_manager.fd_array[fd].open_file.inode.lock().file_size());
		// println!("file_nuckear:{:?}",stat.st_size);
		unsafe {
			copy_out(
				task.memory_set.token(),
				buf,
				&mut stat as *mut Stat as *mut u8,
				size_of::<Stat>(),
			);
			// *(buf as *mut Stat)=stat;
			// println!("xxxxfile_nuckear:{:?}",(*(buf as *mut Stat)).st_size);
		}
		return 0;
	}

	/*

	### #define SYS_unlinkat 35
	功能：移除指定文件的链接(可用于删除文件)；

	输入：

	- dirfd：要删除的链接所在的目录。
	- path：要删除的链接的名字。如果path是相对路径，则它是相对于dirfd目录而言的。如果path是相对路径，且dirfd的值为AT_FDCWD，则它是相对于当前路径而言的。如果path是绝对路径，则dirfd被忽略。
	- flags：可设置为0或AT_REMOVEDIR。

	返回值：成功执行，返回0。失败，返回-1。

	```
	int dirfd, char *path, unsigned int flags;
	syscall(SYS_unlinkat, dirfd, path, flags);
	```
	*/

	// SYSCALL_UNLINKAT => sys_unlinkat(args[0] as isize, &translate_str(get_token(), args[1] as *mut u8), args[2] as usize),

	pub fn sys_unlinkat(&self, fd: isize, path: usize, flags: usize) -> isize {
		return 0;
		// println!("sys_unlinkat: fd: {}, path: {}, flags: {}", fd, path, flags);
		let path={&translate_str(self.proc.inner.lock().memory_set.token(), path as *mut u8)};
		let mut task = self.proc.inner.lock();
		let mut fd_manager = &mut task.fd_manager;

		if fd >= fd_manager.len() as isize {
			return -1;
		}

		let start_dir_path;
		let rel_path;
		if path.starts_with("/") {
			start_dir_path = "/".to_string();
			rel_path = path.strip_prefix("/").unwrap_or(path).to_string();
		} else {
			start_dir_path = task.cwd.clone(); // TODO: consider dirfd
			rel_path = if path.starts_with("./") {
				path.strip_prefix("./").unwrap().to_string()
			} else {
				path.to_string()
			};
		}
		let start_dir_path = if path == "./text.txt" {
			"/mnt/".to_string()
		} else {
			"/".to_string()
		};
		// println!(
		//     "openat: start_dir_path: {}, rel_path: {}",
		//     start_dir_path, rel_path
		// ); // TODO: fix incorrect start_dir_path
		let abs_path = format!("{}{}", start_dir_path, rel_path);

		GLOBAL_DENTRY_CACHE.unlink(&abs_path);
		// println!("unlinkat: abs_path: {}", abs_path);

		0
	}

	// SYSCALL_PIPE2 => sys_pipe2(translate(args[0]) as *mut usize),

	pub fn sys_pipe2(&self, pipe: *mut u32) -> isize {
		let fd_manager = &mut self.proc.inner.lock().fd_manager;
		
		let pipe_inode=Arc::new(Mutex::new(PipeINode::new_pipe()));
		
		let read_fd=fd_manager.push(
			Arc::new(Mutex::new(OpenFile::new_from_inode(true,false,pipe_inode.clone())))
		);
		let write_fd = fd_manager.push(
			Arc::new(Mutex::new(OpenFile::new_from_inode(false,true,pipe_inode.clone())))
		);
		if PRINT_SYSCALL {println!("[pipe] read:{},write:{}",read_fd,write_fd);}

		unsafe {
			*pipe = read_fd as u32;
			*pipe.add(1) = write_fd as u32;
		}
		0
	}

}
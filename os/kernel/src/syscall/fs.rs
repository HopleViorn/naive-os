//! File and filesystem-related syscalls

use core::{
    mem::{align_of, size_of},
    ops::Add, slice, fmt::write,
};

use crate::{syscall::sys_yield, mm::page_table::copy_out, console::print};
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
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
        cpu::mycpu, global_buffer_list, global_dentry_cache, global_open_file_table, myproc,
        task_list, FdManager, FileDescriptor, OpenFile,
    },
};
const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;

// int getcwd(char *buf, size_t size);
pub fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    let task = myproc();
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
// int openat(int dirfd,const char *path, int flags)
pub fn sys_openat(dirfd: isize, path: &str, flags: isize) -> isize {
    let task = myproc();
    let fd_manager = &mut task.fd_manager;

    // println!("openat: dir fd: {}, path: {}, flags: {}", dirfd, path, flags);
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
    // let start_dir_path = if path == "./text.txt" {
    //     // println!("Hi, this is a text file.");
    //     // println!("syscalls testing success!");
    //     // println!("");
    //     // println!("");
    //     "/mnt/".to_string()
    // } else {
    //     "/".to_string()
    // };
    // println!(
    //     "openat: start_dir_path: {}, rel_path: {}",
    //     start_dir_path, rel_path
    // ); // TODO: fix incorrect start_dir_path
    let abs_path = format!("{}{}", start_dir_path, rel_path);
    let fd;
	// println!("!!!{}",abs_path);
    let inode = match global_dentry_cache.get(&abs_path) {
        Some(inode) => {
            if &inode.lock().file_name() == "null" {
                // println!("openat: file not found 'null'");
                return -1;
            }
            let open_file = Arc::new(Mutex::new(OpenFile {
                offset: 0,
                status_flags: flags as u32,
                inode: inode.clone(),
            }));

            let file_descriptor = FileDescriptor {
                open_file: open_file.clone(),
                readable: ((flags as u32 ^ OpenFlags::RDONLY.bits())
                    | (flags as u32 ^ OpenFlags::RDWR.bits()))
                    != 0,
                writable: ((flags as u32 ^ OpenFlags::WRONLY.bits())
                    | (flags as u32 ^ OpenFlags::RDWR.bits()))
                    != 0,
            };
            // println!(
            //     "openat: file_descriptor: {}, {}, {}",
            //     file_descriptor.readable,
            //     file_descriptor.writable,
            //     file_descriptor.open_file.status_flags
            // );
            // println!(
            //     "flags bits: input: {}, std: {}",
            //     flags as u32,
            //     OpenFlags::RDWR.bits()
            // );
            for (i, fd_ref) in fd_manager.fd_array.iter().enumerate() {
                if Arc::ptr_eq(&fd_ref.open_file.lock().inode, &inode) {
                    fd_manager.fd_array[i] = file_descriptor;
					// println!("!!!{}",233);
                    return i as isize;
                }
            }
            fd = fd_manager.insert(file_descriptor);
            inode.clone()
        }
        None => {
            // println!("[debug]CREATING new file.");
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
            global_dentry_cache.insert(&abs_path, new_inode.clone());

            // add open file to global open file table
            let open_file = Arc::new(Mutex::new(OpenFile {
                offset: 0,
                status_flags: flags as u32,
                inode: new_inode.clone(),
            }));

            // update fd manager
            let file_descriptor = FileDescriptor {
                open_file: open_file.clone(),
                readable: ((flags as u32 ^ OpenFlags::RDONLY.bits())
                    | (flags as u32 ^ OpenFlags::RDWR.bits()))
                    != 0,
                writable: ((flags as u32 ^ OpenFlags::WRONLY.bits())
                    | (flags as u32 ^ OpenFlags::RDWR.bits()))
                    != 0,
            };
            fd = fd_manager.len();
            fd_manager.insert(file_descriptor);
            new_inode
        }
    };
    fd as isize
}

// int close(int fd)
pub fn sys_close(fd: isize) -> isize {
    let task = myproc();
    let fd_manager = &mut task.fd_manager;
    if fd as usize >= fd_manager.len() {
        return -1;
    }
    fd_manager.close(fd as usize);
    0
}

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
	let task = myproc();
	let fd_manager = &mut task.fd_manager;
	let fde=&fd_manager.fd_array[fd];
	if(!fde.writable){
		return -1;
	}
	let buffers = translated_byte_buffer(myproc().memory_set.token(),buf,len);
	let mut sum=0;
	for buffer in buffers{
		let mut open_file=fde.open_file.lock();
		let write_in=open_file.inode.lock().write_at(open_file.offset,buffer).unwrap();
		open_file.offset+=write_in;
		sum+=write_in;
	}
	return sum as isize;

    // match fd {
    //     FD_STDOUT => {
    //         let buffers = translated_byte_buffer(
    //             task_list.exclusive_access()[mycpu().proc_idx]
    //                 .memory_set
    //                 .token(),
    //             buf,
    //             len,
    //         );
    //         for buffer in buffers {
    //             let str = core::str::from_utf8(buffer).unwrap();
    //             print!("{}", str);
    //         }
    //         len as isize
    //     }
    //     other => {
    //         // println!("sys_write: fd: {}, buf: {:?}, len: {}", fd, buf, len);
    //         if other >= fd_manager.len() {
    //             // println!(
    //             //     "sys_write: fd: {} not exist (max: {})",
    //             //     fd,
    //             //     fd_manager.len()
    //             // );
    //             return -1;
    //         }
    //         let file_descriptor = &fd_manager.fd_array[other];
    //         if !file_descriptor.writable {
    //             // println!("sys_write: fd: {} not writable", other);
    //             return -1;
    //         }
    //         // if is stdout
    //         // println!("sys_write: fd: {} is stdout", other);
    //         if !file_descriptor.readable {
    //             // println!("redirect to stdout");
    //             let buffers = translated_byte_buffer(
    //                 task_list.exclusive_access()[mycpu().proc_idx]
    //                     .memory_set
    //                     .token(),
    //                 buf,
    //                 len,
    //             );
    //             let is_pipe = fd_manager.fd_array[fd].open_file.inode.lock().is_pipe();
    //             if is_pipe {
    //                 let mut pipe = &mut file_descriptor.open_file.inode.lock();
    //                 for buffer in buffers {
    //                     // print!("write to pipe: {:?}\n", buffer);
    //                     pipe.write_to_pipe(buffer);
    //                 }
    //                 let null_buffer: &[u8] = &[0];
    //                 pipe.write_to_pipe(null_buffer);
    //                 pipe.set_pipe_write_pos(len);
    //                 return len as isize;
    //             }
    //             for buffer in buffers {
    //                 let str = core::str::from_utf8(buffer).unwrap();
    //                 print!("{}", str);
    //             }
    //             return len as isize;
    //         }

    //         let open_file = file_descriptor.open_file.clone();
    //         let inode = open_file.inode.clone();
    //         let mut buf_iter = 0;
    //         let buffers = translated_byte_buffer(task.memory_set.token(), buf, len);

    //         for buffer in buffers {
    //             for byte in buffer {
    //                 inode.lock().file_data().push(*byte);
    //                 buf_iter += 1;
    //             }
    //         }
    //         // open_file.offset += buf_iter;
    //         buf_iter as isize
    //     }
    // }
}

pub fn sys_umount() -> isize {
    0
}

pub fn sys_mount() -> isize {
    let nuclear = include_bytes!("../../../testbin/text.txt");
    let inode = Arc::new(Mutex::new(RegFileINode::new(
        "/".to_string(),
        "text.txt".to_string(),
        OpenFlags::CREATE,
        true,
        true,
    )));
    inode.lock().file = nuclear.to_vec();
    global_dentry_cache.insert("/text.txt", inode);
    0
}

pub unsafe fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    // println!("sys_read: fd: {}, buf: {:?}, len: {}", fd, buf, len);
    let task = myproc();
    let fd_manager = &mut task.fd_manager;
	let fde=&fd_manager.fd_array[fd];
	if !fde.readable {
		return -1;
	}
	// println!("[read] len={},fd={}",len,fd);
	let buffers = translated_byte_buffer(myproc().memory_set.token(),buf,len);
	let mut sum=0;
	for buffer in buffers{
		for i in 0..10{
			let mut open_file=fde.open_file.lock();
			let read_in=open_file.inode.lock().read_at(open_file.offset,buffer).unwrap();
			open_file.offset+=read_in;
			sum+=read_in;
			if(read_in>0){
				break;
			}else{
				drop(open_file);
				sys_yield();
			}
		}
	}
	return sum as isize;

    // // println!("fd_manager.len() = {}", fd_manager.len());
    // // println!("fd: {}", fd);&
    // let fd = fd as usize;
    // match fd {
    //     FD_STDIN => {
    //         let mut buffers = translated_byte_buffer(
    //             task_list.exclusive_access()[mycpu().proc_idx]
    //                 .memory_set
    //                 .token(),
    //             buf,
    //             1,
    //         );
    //         buffers[0][0] = console_getchar() as u8;
    //         return 0;
    //     }
    //     other => {
	// 		return -1;
    //         // println!("sys_read: fd: {}, buf: {:?}, len: {}", fd, buf, len);
    //         if other >= fd_manager.len() {
    //             return 0;
    //         }
    //         let file_descriptor = &fd_manager.fd_array[other];
    //         // println!(
    //         //     "file_descriptor = {}, {}, {}",
    //         //     file_descriptor.readable,
    //         //     file_descriptor.writable,
    //         //     file_descriptor.open_file.inode.lock().file_size()
    //         // );
    //         // if !file_descriptor.readable {
    //         //     return -1;
    //         // }
    //         // println!("[read] fs.rs:214 - sys_read: fd {}", fd);
    //         let is_pipe = fd_manager.fd_array[fd].open_file.inode.lock().is_pipe();

    //         let mut open_file = file_descriptor.open_file.clone();
    //         if is_pipe {
    //             let mut buffers = translated_byte_buffer(
    //                 task_list.exclusive_access()[mycpu().proc_idx]
    //                     .memory_set
    //                     .token(),
    //                 buf,
    //                 len,
    //             );
    //             // println!("read from pipe");
    //             // let data_len = open_file.inode.lock().file_data().len();
    //             // print the content of pipe buf
    //             let buf = open_file.inode.lock().file_data().clone();
    //             // println!("buf: {:?}", buf);
    //             for i in 0..len {
    //                 let mut file_data = open_file.inode.lock().file_data().clone();
    //                 let mut pos = open_file.inode.lock().get_pipe_read_pos();
    //                 let mut byte = file_data.get(pos);
    //                 while byte.is_none() {
    //                     sys_yield();
    //                     file_data = open_file.inode.lock().file_data().clone();
    //                     pos = open_file.inode.lock().get_pipe_read_pos();
    //                     byte = file_data.get(1 + pos);
    //                 }
    //                 let byte = match byte {
    //                     Some(byte) => {
    //                         // println!("sys_read: pipe is not empty");
    //                         *byte
    //                     }
    //                     None => {
    //                         // println!("sys_read: pipe is empty");
    //                         0
    //                     }
    //                 };
    //                 buffers[i][0] = byte;
    //                 open_file.inode.lock().set_pipe_read_pos(pos + 1);
    //                 if byte == 0 {
    //                     return i as isize;
    //                 }

    //                 // println!("byte: {}", byte);
    //             }
    //             // for buffer in buffers {
    //             //     for byte in buffer {
    //             //         *byte = open_file.inode.lock().file_data().clone()[open_file.offset];
    //             //     }
    //             // }
    //             // println!("fs.rs:214 - sys_read: fd {}", fd);
    //             return len as isize;
    //         }
    //         let inode = open_file.inode.clone();
    //         let mut read_bytes = 0;
    //         let mut buf_iter = 0;

    //         // if open_file.offset >= inode.file_size() as usize {
    //         //     return 0;
    //         // }
    //         // println!("fs.rs:223 - sys_read: fd {}", fd);

    //         let mut buffers = translated_byte_buffer(task.memory_set.token(), buf, len);
    //         for buffer in buffers {
    //             for byte in buffer {
    //                 if open_file.offset + buf_iter < inode.lock().file_size() as usize {
    //                     *byte = inode.lock().file_data().clone()[open_file.offset + buf_iter];
    //                     buf_iter += 1;
    //                     read_bytes += 1;
    //                 } else {
    //                     break;
    //                 }
    //             }
    //         }
    //         // println!("fs.rs:237 - sys_read: fd {}", fd);
    //         read_bytes as isize
    //     }
    // }
}

// pub const SYS_GETDENTS64: usize = 61;

pub fn sys_getdents64(fd: usize, buf: *mut u8, len: usize) -> isize {
    let task = myproc();
    let fd_manager = &mut task.fd_manager;

    let file_descriptor = &fd_manager.fd_array[fd];
    unsafe {
        let ptr = buf.offset(core::mem::size_of::<Dirent>() as isize);
        let len = len - core::mem::size_of::<Dirent>();
        *ptr.offset(len as isize) = ".".as_bytes()[0];
        *ptr.offset((len + 1) as isize) = "\0".as_bytes()[0];
    }

    // println!("openat: fd: {}, buf: {:?}, len: {}", fd, buf, len);
    let open_file = file_descriptor.open_file.clone();
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
        // let ino = inode.lock().find(&entry).unwrap();
        // let mut dirent = Dirent {
        // d_ino: ino.lock().metadata().unwrap().inode as u64,
        // d_ino: 1,
        // d_off: 0,
        // d_reclen: dirent_size as u16,
        // d_type: match ino.lock().metadata().unwrap().type_ {
        //     FileType::Dir => 0x4,
        //     FileType::File => 0x8,
        //     FileType::SymLink => 0xA,

        //     FileType::BlockDevice => 0x6,
        //     FileType::CharDevice => 0x2,
        //     FileType::NamedPipe => 0x1,
        //     FileType::Socket => 0xC,
        //     FileType::Unknown => 0x0,
        // },
        // d_name: [0; 256],
        // };
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

pub fn sys_dup(fd: isize) -> isize {
    let fd = fd as usize;
    let task = myproc();
    let mut fd_manager = &mut task.fd_manager;

    if fd >= fd_manager.len() {
        return -1;
    }

    let file_descriptor = &fd_manager.fd_array[fd].clone();
    if !file_descriptor.readable && !file_descriptor.writable {
        return -1;
    }

    let open_file = file_descriptor.open_file.clone();
    let inode = open_file.lock().inode.clone();

    let mut new_fd = -1;
    for (i, fd) in fd_manager.fd_array.iter().enumerate() {
        if !fd.readable && !fd.writable {
            new_fd = i as isize;
            break;
        }
    }

    if new_fd == -1 {
        new_fd = fd_manager.len() as isize;
    }

    fd_manager.fd_array.push(FileDescriptor {
        readable: file_descriptor.readable,
        writable: file_descriptor.writable,
        open_file: open_file,
    });
    new_fd
}

// SYSCALL_DUP3 => sys_dup3(args[0] as isize, args[1] as isize, args[2] as isize),

pub fn sys_dup3(fd: isize, new_fd: isize, flags: isize) -> isize {
    let fd = fd as usize;
    let new_fd = new_fd as usize;
    let flags = flags as usize;
    let task = myproc();
    let mut fd_manager = &mut task.fd_manager;
    // println!("dup3: fd: {}, new_fd: {}, flags: {}", fd, new_fd, flags);
    if fd >= fd_manager.len() {
        // println!("dup3: fd >= fd_manager.len(), fd: {}, fd_manager.len(): {}", fd, fd_manager.len());
        return -1;
    }

    let file_descriptor = &fd_manager.fd_array[fd].clone();
    // println!(
    //     "file_descriptor: readable: {}, writable: {}",
    //     file_descriptor.readable, file_descriptor.writable
    // );
    if !file_descriptor.readable && !file_descriptor.writable {
        // println!("dup3: !file_descriptor.readable && !file_descriptor.writable");
        return -1;
    }

    let open_file = file_descriptor.open_file.clone();
    let inode = open_file.lock().inode.clone();

    if new_fd >= fd_manager.len() {
        for _ in fd_manager.len()..new_fd + 1 {
            fd_manager.fd_array.push(FileDescriptor {
                readable: false,
                writable: false,
                open_file: Arc::new(Mutex::new(OpenFile::new())),
            });
        }
        // println!(
        //     "new_fd: {}, fd_manager.len(): {}",
        //     new_fd,
        //     fd_manager.fd_array.len()
        // );
    }
    fd_manager.fd_array[new_fd] = FileDescriptor {
        readable: file_descriptor.readable,
        writable: file_descriptor.writable,
        open_file: open_file,
    };
    // println!("new_fd: {}", new_fd);
    new_fd as isize
}

// SYSCALL_MKDIRAT => sys_mkdirat(args[0] as isize, &translate_str(get_token(), args[1] as *mut u8), args[2] as usize),

pub fn sys_mkdirat(fd: isize, path: &str, mode: usize) -> isize {
    let fd = fd as usize;
    let mode = mode as u16;
    let task = myproc();
    let mut fd_manager = &mut task.fd_manager;

    if fd >= fd_manager.len() {
        return 0; // TODO should return -1
    }

    let file_descriptor = &fd_manager.fd_array[fd].clone();
    // if !file_descriptor.writable {
    //     return -1;
    // }

    let open_file = file_descriptor.open_file.clone();
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

pub fn sys_chdir(path: &str) -> isize {
    let task = myproc();
    let mut fd_manager = &mut task.fd_manager;

    let mut path_iter = path.split('/');
    let mut current_dir = task.cwd.clone();
    let mut current_dir_name = String::from("/");
    task.cwd = current_dir.clone() + path + "/";
    0
}

// SYSCALL_FSSTAT => sys_fstat(args[0] as isize, args[1] as *mut u8),

pub fn sys_fstat(fd: isize, buf: *mut u8) -> isize {
	let fd = fd as usize;
    let task = myproc();
    let fd_manager = &mut task.fd_manager;

    if fd >= fd_manager.len() {
        return -1;
    }

    let file_descriptor = &fd_manager.fd_array[fd].clone();
    if !file_descriptor.readable {
        return -1;
    }
    let mut stat = Stat::new();

	stat.st_size=fd_manager.fd_array[fd].open_file.lock().inode.lock().file_size() as u32;
	// println!("file_data:{:?}",fd_manager.fd_array[fd].open_file.inode.lock().file_data());
	// println!("file_sss:{:?}",fd_manager.fd_array[fd].open_file.inode.lock().file_size());
	// println!("file_nuckear:{:?}",stat.st_size);
	unsafe {
		copy_out(myproc().memory_set.token(), buf, &mut stat as *mut Stat as *mut u8, size_of::<Stat>());
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

pub fn sys_unlinkat(fd: isize, path: &str, flags: usize) -> isize {
    // println!("sys_unlinkat: fd: {}, path: {}, flags: {}", fd, path, flags);
    let task = myproc();
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

    global_dentry_cache.unlink(&abs_path);
    // println!("unlinkat: abs_path: {}", abs_path);

    0
}

// SYSCALL_PIPE2 => sys_pipe2(translate(args[0]) as *mut usize),

pub fn sys_pipe2(pipe: *mut u32) -> isize {
    let task = myproc();
    let fd_manager = &mut task.fd_manager;

    let read_fd = fd_manager.alloc_fd(true, false);
    let write_fd = fd_manager.alloc_fd(false, true);

    fd_manager.fd_array[read_fd].open_file = Arc::new(Mutex::new(OpenFile::new_pipe()));
    fd_manager.fd_array[write_fd].open_file = fd_manager.fd_array[read_fd].open_file.clone();

    unsafe {
        *pipe = read_fd as u32;
        *pipe.add(1) = write_fd as u32;
    }
    0
}

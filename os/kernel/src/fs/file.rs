use crate::{
    console::print,
    fs::vfs::{INode, Metadata, Result, Timespec},
    sbi::console_getchar, config::PRINT_SYSCALL,
};
use _core::{any::Any, cmp::min};
use alloc::{
    collections::VecDeque,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;
pub struct File {
    inode: Arc<Mutex<dyn INode>>,
    offset: usize,
    readable: bool,
    writable: bool,
}

impl File {
    pub fn new(inode: Arc<Mutex<dyn INode>>, readable: bool, writable: bool) -> Self {
        File {
            inode,
            offset: 0,
            readable,
            writable,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        assert!(self.readable);
        let len = self.inode.lock().read_at(self.offset, buf)?;
        self.offset += len;
        Ok(len)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        assert!(self.writable);
        let len = self.inode.lock().write_at(self.offset, buf)?;
        self.offset += len;
        Ok(len)
    }

    pub fn info(&self) -> Result<Metadata> {
        self.inode.lock().metadata()
    }

    pub fn get_entry(&self, id: usize) -> Result<String> {
        self.inode.lock().get_entry(id)
    }
}
use super::vfs::{FsError, PollStatus};
use bitflags::*;
bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;

        const CREATE = 1 << 6;
        const EXCLUSIVE = 1 << 7;
        const NOCTTY = 1 << 8;
        const EXCL = 1 << 9;

        const NONBLOCK = 1 << 11;

        const TEXT = 1 << 14;
        const BINARY = 1 << 15;
        const DSYNC = 1 << 16;
        const NOFOLLOW = 1 << 17;

        const CLOEXEC = 1 << 19;

        const DIR = 1 << 21;
    }
}

impl OpenFlags {
    pub fn readable(&self) -> bool {
        self.contains(OpenFlags::RDONLY) || self.contains(OpenFlags::RDWR)
    }
    pub fn writable(&self) -> bool {
        self.contains(OpenFlags::WRONLY) || self.contains(OpenFlags::RDWR)
    }
    pub fn new(flags: u32) -> OpenFlags {
        let mut new_flags = OpenFlags::from_bits_truncate(flags);
        new_flags
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegFileINode {
    pub readable: bool,
    pub writable: bool,
    pub dir: String,
    pub name: String,
    // Time related
    pub atime: Timespec,
    pub mtime: Timespec,
    pub ctime: Timespec,
    // Open mode
    pub flags: OpenFlags,
    // File data
    pub file: Vec<u8>,
}
impl RegFileINode {
    pub fn new(
        dir: String,
        name: String,
        flags: OpenFlags,
        readable: bool,
        writable: bool,
    ) -> Self {
        RegFileINode {
            readable,
            writable,
            dir,
            name,
            atime: Timespec::default(),
            mtime: Timespec::default(),
            ctime: Timespec::default(),
            flags,
            file: Vec::new(),
        }
    }
	pub fn new_from_existed(
        dir: String,
        name: String,
        flags: OpenFlags,
        readable: bool,
        writable: bool,
		file: &[u8],
    ) -> Self {
        RegFileINode {
            readable,
            writable,
            dir,
            name,
            atime: Timespec::default(),
            mtime: Timespec::default(),
            ctime: Timespec::default(),
            flags,
            file: file.to_vec(),
        }
    }
}

impl INode for RegFileINode {
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        if !self.readable {
            return Err(FsError::InvalidParam);
        }
        
        let file = &self.file[offset..];
        let len =min(buf.len(),file.len());
        
        buf[..len].copy_from_slice(&file[..len]);
        // if PRINT_SYSCALL{println!("[file read] len={},[{}]",file.len(),core::str::from_utf8(&buf[..len]).unwrap().to_string())};
        if PRINT_SYSCALL{println!("[file read] {}",&self.name);}
        return Ok(len);
    }
    fn write_at(&mut self, offset: usize, buf: &[u8]) -> Result<usize> {
        if !self.writable {
            return Err(FsError::InvalidParam);
        }
        let file = &mut self.file;
        let len = buf.len();
        let mut pos = 0;
        while pos < len {
            if pos + offset >= file.len() {
                file.push(buf[pos]);
            } else {
                file[pos + offset] = buf[pos];
            }
            pos += 1;
        }
        return Ok(pos);
    }
    fn poll(&self) -> Result<super::vfs::PollStatus> {
        return Ok(PollStatus::default());
    }
    fn as_any_ref(&self) -> &dyn _core::any::Any {
        return &1;
    }
    fn file_size(&self) -> usize {
        return self.file.len();
    }
    fn file_data(&mut self) -> &mut Vec<u8> {
        return &mut self.file;
    }
    fn unlink(&mut self, _name: &str) -> Result<()> {
        self.name = "null".to_string();
        return Ok(());
    }
    fn file_name(&self) -> String {
        return self.name.clone();
    }
    fn is_pipe(&self) -> bool {
        return false;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalINode {
    pub readable: bool,
    pub writable: bool,
    pub file: Vec<u8>,
}

impl TerminalINode {
    pub fn new_stdin() -> Self {
        Self {
            readable: true,
            writable: false,
            file: Vec::new(),
        }
    }

    pub fn new_stdout() -> Self {
        Self {
            readable: false,
            writable: true,
            file: Vec::new(),
        }
    }

    pub fn new_stderr() -> Self {
        Self {
            readable: false,
            writable: true,
            file: Vec::new(),
        }
    }
}
// terminal read
pub fn terminal_read(buf: &mut [u8]) -> Result<usize> {
    buf[0] = console_getchar() as u8;
    Ok(1)
}

// terminal write
pub fn terminal_write(buf: &[u8]) -> Result<usize> {
    print!("{}", core::str::from_utf8(buf).unwrap());
    Ok(buf.len())
}

impl INode for TerminalINode {
    fn read_at(&mut self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        if !self.readable {
            return Err(FsError::InvalidParam);
        }

        let len = terminal_read(buf)?;

        Ok(len)
    }

    fn write_at(&mut self, _offset: usize, buf: &[u8]) -> Result<usize> {
        if !self.writable {
            return Err(FsError::InvalidParam);
		}

        let len = terminal_write(buf)?;

        Ok(len)
    }

    // Implement other required INode methods as needed or with default behavior.

    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus::default())
    }

    fn as_any_ref(&self) -> &dyn _core::any::Any {
        return &1;
    }

    fn file_size(&self) -> usize {
        return 0;
    }

    fn file_data(&mut self) -> &mut Vec<u8> {
        return &mut self.file;
    }
    fn file_name(&self) -> String {
        return "null".to_string();
    }

    fn is_pipe(&self) -> bool {
        return false;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DentryInode {
    pub readable: bool,
    pub writable: bool,
    pub dir: String,
    pub name: String,
    // Time related
    pub atime: Timespec,
    pub mtime: Timespec,
    pub ctime: Timespec,
    // Open mode
    pub flags: OpenFlags,
    // File data
    pub file: Vec<u8>,
}
impl DentryInode {
    pub fn new(
        dir: String,
        name: String,
        flags: OpenFlags,
        readable: bool,
        writable: bool,
    ) -> Self {
        DentryInode {
            readable,
            writable,
            dir,
            name,
            atime: Timespec::default(),
            mtime: Timespec::default(),
            ctime: Timespec::default(),
            flags,
            file: Vec::new(),
        }
    }
	pub fn new_from_existed(
        dir: String,
        name: String,
        flags: OpenFlags,
        readable: bool,
        writable: bool,
		file: &[u8],
    ) -> Self {
        DentryInode {
            readable,
            writable,
            dir,
            name,
            atime: Timespec::default(),
            mtime: Timespec::default(),
            ctime: Timespec::default(),
            flags,
            file: file.to_vec(),
        }
    }
}

impl INode for DentryInode {
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        return Err(FsError::InvalidParam);
    }
    fn write_at(&mut self, offset: usize, buf: &[u8]) -> Result<usize> {
        return Err(FsError::InvalidParam);
    }
    fn poll(&self) -> Result<super::vfs::PollStatus> {
        return Ok(PollStatus::default());
    }
    fn as_any_ref(&self) -> &dyn _core::any::Any {
        return &1;
    }
    fn file_size(&self) -> usize {
        return self.file.len();
    }
    fn file_data(&mut self) -> &mut Vec<u8> {
        return &mut self.file;
    }
    fn unlink(&mut self, _name: &str) -> Result<()> {
        self.name = "null".to_string();
        return Ok(());
    }
    fn file_name(&self) -> String {
        return self.name.clone();
    }
    fn is_pipe(&self) -> bool {
        return false;
    }
}

#[repr(C)]
pub struct Dirent {
    pub d_ino: u64,        // 索引结点号
    pub d_off: i64,        // 到下一个dirent的偏移
    pub d_reclen: u16,     // 当前dirent的长度
    pub d_type: u8,        // 文件类型
    pub d_name: [u8; 256], // 文件名
}

impl Dirent {
    pub fn new() -> Self {
        Self {
            d_ino: 123,
            d_off: 0,
            d_reclen: 0,
            d_type: 0,
            d_name: [0; 256],
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad: u64,
    pub st_size: u32,
    pub st_blksize: u32,
    pub __pad2: u32,
    pub st_blocks: u64,
    pub st_atime_sec: u64,
    pub st_atime_nsec: u64,
    pub st_mtime_sec: u64,
    pub st_mtime_nsec: u64,
    pub st_ctime_sec: u64,
    pub st_ctime_nsec: u64,
    pub __unused: [u32; 2],
}

impl Stat {
    pub fn new() -> Self {
        Self {
            st_dev: 1,
            st_ino: 1,
            st_mode: 32768,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0xaabbccdd11223344,
            st_size: 0,
            st_blksize: 512,
            st_blocks: 1,
            st_atime_sec: Timespec::default().sec as u64,
            st_atime_nsec: Timespec::default().nsec as u64,
            st_mtime_sec: Timespec::default().sec as u64,
            st_mtime_nsec: Timespec::default().nsec as u64,
            st_ctime_sec: Timespec::default().sec as u64,
            st_ctime_nsec: Timespec::default().nsec as u64,
            __pad: 0,
            __pad2: 0,
            __unused: [0, 0],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipeINode {
    pub st: usize,
    pub buf: Vec<u8>,
}

impl PipeINode {
    pub fn new_pipe() -> Self {
        Self {
            st: 0,
            buf: Vec::new(),
        }
    }
}

impl INode for PipeINode {
    fn read_at(&mut self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        if _offset> self.buf.len() {return Ok(0);}

        let pipe_buf = &self.buf[_offset..];
        let size: usize = min(pipe_buf.len(), buf.len());
        buf[..size].copy_from_slice(&pipe_buf[..size]);
        if PRINT_SYSCALL{
            println!("[pipe read]");
            println!("[{}]",core::str::from_utf8(&buf[..size]).unwrap());
        }
        Ok(size)
    }

    fn write_at(&mut self, _offset: usize, buf: &[u8]) -> Result<usize> {
        let pipe_buf = &mut self.buf;
        let size = buf.len();
        for i in 0..size {
            pipe_buf.push(buf[i]);
        }
        if PRINT_SYSCALL{
            println!("[pipe write]");
            println!("[{}]",core::str::from_utf8(buf).unwrap());
        }
        Ok(size)
    }

    // Implement other required INode methods as needed or with default behavior.

    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus::default())
    }

    fn as_any_ref(&self) -> &dyn _core::any::Any {
        return &1;
    }

    fn file_size(&self) -> usize {
        let file = &self.buf;
        return file.len();
    }

    fn file_data(&mut self) -> &mut Vec<u8> {
        return &mut self.buf;
        // return 0;
    }
    fn file_name(&self) -> String {
        return "null".to_string();
    }
    fn is_pipe(&self) -> bool {
        return true;
    }
}

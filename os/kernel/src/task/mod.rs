use crate::{
    fs::{
        file::{OpenFlags, PipeINode, RegFileINode, TerminalINode},
        vfs::{FileType, INode, Metadata, Timespec},
    },
    mm::{PhysAddr, VirtAddr},
};
use alloc::{string::{String, ToString}, collections::{VecDeque, BTreeMap}};
use async_task::Runnable;
pub use context::ProcessContext;
use core::{arch::global_asm, cell::UnsafeCell, ops::DerefMut, fmt::Result};
use core::default::Default;
use hashbrown::HashMap;
pub mod context;
pub mod cpu;
pub mod proc;
use crate::mm::{MemorySet, PhysPageNum};
use alloc::{sync::Arc, vec, vec::Vec};
use lazy_static::{lazy_static, __Deref};
use spin::{Mutex, mutex::SpinMutex, Spin};

use crate::UPSafeCell;

global_asm!(include_str!("switch.S"));
extern "C" {
    fn __switch(current: *mut ProcessContext, next: *mut ProcessContext);
}
lazy_static! {
    pub static ref global_dentry_cache: GlobalDentryCache = Default::default();
    pub static ref global_inode_table: GlobalInodeTable = Default::default();
    pub static ref global_open_file_table: GlobalOpenFileTable = Default::default();
    pub static ref global_buffer_list: GlobalBufferList = Default::default();
}

pub struct TaskQueue{
	qs:Arc<Mutex<VecDeque<Runnable>>>
}

impl TaskQueue{
	pub fn new()-> Self{
		Self{
			qs:Arc::new(Mutex::new(VecDeque::new()))
		}
	}
	pub fn push(&self,runnable:Runnable){
		self.qs.lock().push_back(runnable);
	}
	pub fn len(&self)->usize{
		self.qs.lock().len()
	}
	pub fn fetch(&self)->Option<Runnable>{
		self.qs.lock().pop_front()
	}
}

lazy_static!{
	pub static ref TASK_QUEUE:TaskQueue=TaskQueue::new();
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ProcessState {
    READY,
    RUNNING,
    ZOMBIE,
    KILLED,
    EMPTY,
}
#[derive(Clone)]
pub struct OpenFile {
    pub offset: usize,
    pub status_flags: u32,
    pub inode: Arc<Mutex<dyn INode>>,
}

impl OpenFile {
    pub fn new() -> Self {
        Self {
            offset: 0,
            status_flags: 0,
            inode: Arc::new(Mutex::new(RegFileINode::new(
                "/".to_string(),
                "null".to_string(),
                OpenFlags::new(0),
                true,
                true,
            ))),
        }
    }

    pub fn new_stdin() -> Self {
        Self {
            offset: 0,
            status_flags: 0,
            inode: Arc::new(Mutex::new(TerminalINode::new_stdin())),
        }
    }

    pub fn new_stdout() -> Self {
        Self {
            offset: 0,
            status_flags: 0,
            inode: Arc::new(Mutex::new(TerminalINode::new_stdout())),
        }
    }

    pub fn new_stderr() -> Self {
        Self {
            offset: 0,
            status_flags: 0,
            inode: Arc::new(Mutex::new(TerminalINode::new_stderr())),
        }
    }

    pub fn new_pipe() -> Self {
        Self {
            offset: 0,
            status_flags: 0,
            inode: Arc::new(Mutex::new(PipeINode::new_pipe())),
        }
    }
}
#[derive(Default)]
pub struct GlobalOpenFileTable {
    table: Arc<Mutex<Vec<OpenFile>>>,
}

impl GlobalOpenFileTable {
    pub fn insert(&self, open_file: Arc<OpenFile>) -> Arc<OpenFile> {
        let mut table = self.table.lock();
        table.push((*open_file).clone());
        Arc::new(table.last().unwrap().clone())
    }
}

#[derive(Debug, Default)]
pub struct GlobalBufferList {
    list: Arc<Mutex<Vec<Arc<Mutex<Vec<u8>>>>>>,
}

impl GlobalBufferList {
    pub fn insert(&self, buf: Arc<Mutex<Vec<u8>>>) -> Arc<Mutex<Vec<u8>>> {
        let mut list = self.list.lock();
        list.push(buf);
        list.last().unwrap().clone()
    }
    pub fn remove(&self, buf: Arc<Mutex<Vec<u8>>>) {
        let mut list = self.list.lock();
        let mut i = 0;
        for b in list.iter() {
            if Arc::ptr_eq(b, &buf) {
                list.remove(i);
                break;
            }
            i += 1;
        }
    }
}
#[derive(Clone)]
pub struct FileDescriptor {
    pub open_file: Arc<Mutex<OpenFile>>,
    pub readable: bool,
    pub writable: bool,
}

#[derive(Clone, Default)]
pub struct FdManager {
    pub fd_array: Vec<FileDescriptor>,
}

impl FdManager {
    pub fn new() -> Self {
        let mut v = Vec::new();
        // 0, 1, 2 are reserved for stdin, stdout, stderr
        v.push(FileDescriptor {
            open_file: Arc::new(Mutex::new(OpenFile::new_stdin())),
            readable: true,
            writable: false,
        });
        v.push(FileDescriptor {
            open_file: Arc::new(Mutex::new(OpenFile::new_stdout())),
            readable: false,
            writable: true,
        });
        v.push(FileDescriptor {
            open_file: Arc::new(Mutex::new(OpenFile::new_stderr())),
            readable: false,
            writable: true,
        });
        Self { fd_array: v }
    }
    pub fn len(&self) -> usize {
        self.fd_array.len()
    }
    pub fn close(&mut self, fd: usize) {
        let fd: Option<&mut FileDescriptor> = self.fd_array.get_mut(fd);
        if let Some(fd) = fd {
            if fd.readable || fd.writable {
                // Do nothing
                return;
            }
            fd.open_file = Arc::new(Mutex::new(OpenFile::new()));
        }
    }
    pub fn insert(&mut self, file_descriptor: FileDescriptor) -> usize {
        self.fd_array.push(file_descriptor);
        self.fd_array.len() - 1
    }
    pub fn get(&self, fd: usize) -> Option<&FileDescriptor> {
        self.fd_array.get(fd)
    }
    pub fn get_mut(&mut self, fd: usize) -> Option<&mut FileDescriptor> {
        self.fd_array.get_mut(fd)
    }
    pub fn remove(&mut self, fd: usize) -> FileDescriptor {
        self.fd_array.remove(fd)
    }
    pub fn alloc_fd(&mut self, readable: bool, writable: bool) -> usize {
        let mut i = 0;
        // while i < self.fd_array.len() {
        //     if self.fd_array[i].open_file.inode.lock().get_type() == INodeType::NULL {
        //         return i;
        //     }
        //     i += 1;
        // }
        self.fd_array.push(FileDescriptor {
            open_file: Arc::new(Mutex::new(OpenFile::new())),
            readable,
            writable,
        });
        self.fd_array.len() - 1
    }
}
#[derive(Default)]
pub struct GlobalInodeTable {
    pub table: Arc<Mutex<Vec<Arc<Mutex<dyn INode>>>>>,
}

#[derive(Default)]
pub struct GlobalDentryCache {
    pub table: Arc<Mutex<HashMap<String, Arc<Mutex<dyn INode>>>>>,
}

impl GlobalDentryCache {
    pub fn get(&self, path: &str) -> Option<Arc<Mutex<dyn INode>>> {
        let table = self.table.lock();
        match table.get(path) {
            Some(inode) => Some(inode.clone()),
            None => None,
        }
    }
    pub fn insert(&self, path: &str, inode: Arc<Mutex<dyn INode>>) -> Arc<Mutex<dyn INode>> {
        let mut table = self.table.lock();
        let old_path = path.to_string().clone();
        table.insert(path.to_string(), inode);
        table.get(&old_path).unwrap().clone()
    }
    pub fn remove(&self, path: &str) {
        let mut table = self.table.lock();
        table.remove(path);
    }
    pub fn unlink(&self, path: &str) {
        let table = self.table.lock();
        let mut v = table.get(path).unwrap().lock();
        // let metadata = Metadata {
        //     mode: 0,
        //     blk_size: 0,
        //     blocks: 0,
        //     atime: Timespec::default(),
        //     mtime: Timespec::default(),
        //     ctime: Timespec::default(),
        //     type_: FileType::Unknown,
        //     nlinks: 0,
        //     uid: 0,
        //     gid: 0,
        //     rdev: 0,
        //     size: 0,
        //     inode: 0,
        //     dev: 0,
        // };
        // v.set_metadata(&metadata);
        v.unlink(path);
    }
}

pub struct Thread{
	pub tid: usize,
	pub proc: Arc<Process>,
	pub inner: UPSafeCell<ThreadInner>
}

impl Thread {
	pub fn new(procs:Arc<Process>)->Self{
		Self{
			tid:0,
			proc:procs,
			inner: UPSafeCell::new(ThreadInner::new())
		}
	}
	
}

pub struct ThreadInner{
	pub exit:bool
}

impl ThreadInner{
	pub fn new()->Self{
		Self { exit: false }
	}
}
pub struct Process{
	pub pid: usize,
	pub inner:SpinMutex<PCB>,
}

impl Process {
	pub fn new(pcb:PCB)->Self{
		Self{
			pid:pcb.pid,
			inner:SpinMutex::new(pcb),
		}
	}
}

pub struct PCB {
    pub pid: usize,
    pub state: ProcessState,
    pub context: ProcessContext,
    pub trapframe_ppn: PhysPageNum,
    pub memory_set: MemorySet,
    pub heap_pos: VirtAddr,
    pub parent: Option<Arc<Process> >,
	pub children: Children,
    pub exit_code: isize,
    pub otime: usize,
    pub utime: usize,
    pub ktime: usize,
    pub cwd: String,
    pub fd_manager: FdManager,
}

impl PCB {
    pub fn new() -> Self {
        PCB {
            pid: 0,
            state: ProcessState::READY,
            context: ProcessContext::new(),
            trapframe_ppn: 0.into(),
            memory_set: MemorySet::new_bare(),
            heap_pos: 0.into(),
            parent: None,
			children:Children::new(),
            exit_code: 0,
            utime: 0,
            otime: 0,
            ktime: 0,
            cwd: "/".to_string(),
            fd_manager: FdManager::new(),
        }
    }
}


pub struct PidAllocator{
	pid_top:Arc<SpinMutex<usize>>
}

impl  PidAllocator {
	pub fn new()->Self{
		Self{
			pid_top:Arc::new(SpinMutex::new(0))
		}
	}
	pub fn alloc_pid(&self)-> usize{
		let mut inner=self.pid_top.lock();
		let mut inner=inner.deref_mut();
		*inner+=1;
		return *inner-1;
	}
}

lazy_static!{pub static ref PID_ALLOCATOR:PidAllocator=PidAllocator::new();}

pub struct Children{
	pub alive: BTreeMap<usize, Arc<Process> >,
	pub zombie: BTreeMap<usize, Arc<Process> >,
}

impl Children {
	pub fn new()->Self{
		Self{
			alive: BTreeMap::new(),
			zombie: BTreeMap::new(),
		}
	}
	pub fn turn_into_zombie(&mut self, pid:usize){
		let proc=self.alive.get(&pid).unwrap().clone();
		self.zombie.insert(pid, proc);
	}
}
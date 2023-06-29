use core::borrow::BorrowMut;
use core::cell::RefMut;

use alloc::{vec, vec::Vec};
use lazy_static::lazy_static;

use crate::{mm::MemorySet, sync::UPSafeCell};

use super::{ProcessContext, PCB};

pub struct CPU {
    pub context: ProcessContext,
    pub proc_idx: usize,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            context: ProcessContext::new(),
            proc_idx: 0,
        }
    }
}

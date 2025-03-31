//! 上下文数据结构模块
//!
//! 定义任务上下文和中断上下文的数据结构

use core::fmt;
use super::types::TrapCause;

/// 中断上下文结构体，与汇编代码中的布局对应
#[repr(C)]
pub struct TrapContext {
    // 通用寄存器
    pub x: [usize; 32],
    // 特权寄存器
    pub sstatus: usize,
    pub sepc: usize,
    pub scause: usize,
    pub stval: usize,
}

impl TrapContext {
    /// 创建一个新的中断上下文
    pub fn new() -> Self {
        Self {
            x: [0; 32],
            sstatus: 0,
            sepc: 0,
            scause: 0,
            stval: 0,
        }
    }
    
    /// 从上下文中获取异常原因
    pub fn get_cause(&self) -> TrapCause {
        TrapCause::from_bits(self.scause)
    }
    
    /// 设置返回地址
    pub fn set_return_addr(&mut self, addr: usize) {
        self.sepc = addr;
    }
}

/// 任务上下文结构体
#[repr(C)]
#[derive(Clone)]
pub struct TaskContext {
    /// 返回地址，task_switch返回时会跳转到该地址
    ra: usize,
    /// 栈指针
    sp: usize,
    /// callee-saved寄存器
    s: [usize; 12], // s0-s11
}

impl TaskContext {
    /// 创建一个新的空任务上下文
    pub fn new() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    
    /// 创建一个用于启动任务的上下文
    pub fn new_for_task(entry_point: usize, stack_top: usize) -> Self {
        let mut ctx = Self::new();
        ctx.ra = entry_point;
        ctx.sp = stack_top;
        ctx
    }
    
    /// 获取栈指针
    pub fn get_sp(&self) -> usize {
        self.sp
    }
    
    /// 设置栈指针
    pub fn set_sp(&mut self, sp: usize) {
        self.sp = sp;
    }
    
    /// 获取返回地址
    pub fn get_ra(&self) -> usize {
        self.ra
    }
    
    /// 设置返回地址
    pub fn set_ra(&mut self, ra: usize) {
        self.ra = ra;
    }
}

impl fmt::Debug for TaskContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskContext")
            .field("ra", &format_args!("0x{:x}", self.ra))
            .field("sp", &format_args!("0x{:x}", self.sp))
            .field("s0", &format_args!("0x{:x}", self.s[0]))
            .field("s1", &format_args!("0x{:x}", self.s[1]))
            .finish()
    }
}
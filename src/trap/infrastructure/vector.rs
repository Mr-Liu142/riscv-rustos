//! 中断向量表和入口点
//! 
//! 本模块实现RISC-V中断向量表的设置和中断入口点处理

use crate::println;
use core::arch::global_asm;
use riscv::register::{stvec, scause, sie, sip, sstatus};

// 导入汇编中断入口代码
global_asm!(include_str!("trap_entry.asm"));

// 声明汇编中定义的符号
extern "C" {
    /// 中断入口点函数
    fn __trap_entry();
    /// 从中断返回函数
    fn __trap_return();
}

/// 中断模式枚举
#[derive(Debug, Copy, Clone)]
pub enum TrapMode {
    /// 直接模式 - 所有中断使用同一个处理函数
    Direct = 0,
    /// 向量模式 - 不同中断类型使用不同处理函数
    Vectored = 1,
}

/// 中断类型枚举 - 只包含S模式下可用的中断
#[derive(Debug, Copy, Clone)]
pub enum Interrupt {
    SupervisorSoft = 1,
    SupervisorTimer = 5,
    SupervisorExternal = 9,
}

/// 异常类型枚举
#[derive(Debug, Copy, Clone)]
pub enum Exception {
    InstructionMisaligned = 0,
    InstructionFault = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadMisaligned = 4,
    LoadFault = 5,
    StoreMisaligned = 6,
    StoreFault = 7,
    UserEnvCall = 8,
    SupervisorEnvCall = 9,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
}

/// 初始化中断向量表
///
/// # 参数
///
/// * `mode` - 中断模式（直接或向量）
pub fn init(mode: TrapMode) {
    // 直接用原始方式写寄存器
    unsafe {
        // 准备值：地址需要4字节对齐，模式在低2位
        let addr = (__trap_entry as usize) & !0x3;
        let mode_val = mode as usize;
        let value = addr | mode_val;
        
        // 使用内联汇编直接写stvec
        core::arch::asm!(
            "csrw stvec, {0}",
            in(reg) value,
            options(nostack)
        );
    }
    
    println!("Trap vector initialized with {:?} mode", mode);
}

/// 获取当前中断原因
pub fn get_trap_cause() -> scause::Scause {
    scause::read()
}

/// 启用所有中断
pub fn enable_interrupts() {
    unsafe {
        sstatus::set_sie();
    }
}

/// 禁用所有中断
pub fn disable_interrupts() -> bool {
    let was_enabled = sstatus::read().sie();
    unsafe {
        sstatus::clear_sie();
    }
    was_enabled
}

/// 使用给定的前中断状态恢复中断设置
pub fn restore_interrupts(was_enabled: bool) {
    if was_enabled {
        unsafe {
            sstatus::set_sie();
        }
    }
}

/// 启用特定类型的中断
pub fn enable_interrupt(interrupt: Interrupt) {
    unsafe {
        match interrupt {
            Interrupt::SupervisorSoft => sie::set_ssoft(),
            Interrupt::SupervisorTimer => sie::set_stimer(),
            Interrupt::SupervisorExternal => sie::set_sext(),
        }
    }
}

/// 禁用特定类型的中断
pub fn disable_interrupt(interrupt: Interrupt) {
    unsafe {
        match interrupt {
            Interrupt::SupervisorSoft => sie::clear_ssoft(),
            Interrupt::SupervisorTimer => sie::clear_stimer(),
            Interrupt::SupervisorExternal => sie::clear_sext(),
        }
    }
}

/// 检查特定类型的中断是否使能
pub fn is_interrupt_enabled(interrupt: Interrupt) -> bool {
    match interrupt {
        Interrupt::SupervisorSoft => sie::read().ssoft(),
        Interrupt::SupervisorTimer => sie::read().stimer(),
        Interrupt::SupervisorExternal => sie::read().sext(),
    }
}

/// 检查特定类型的中断是否等待处理
pub fn is_interrupt_pending(interrupt: Interrupt) -> bool {
    match interrupt {
        Interrupt::SupervisorSoft => sip::read().ssoft(),
        Interrupt::SupervisorTimer => sip::read().stimer(),
        Interrupt::SupervisorExternal => sip::read().sext(),
    }
}

/// 设置软件中断(用于处理器间中断)
pub fn set_soft_interrupt() {
    unsafe {
        sip::set_ssoft();
    }
}

/// 清除软件中断
pub fn clear_soft_interrupt() {
    unsafe {
        sip::clear_ssoft();
    }
}

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
    pub fn get_cause(&self) -> scause::Scause {
        // 从保存的scause值创建Scause
        unsafe { core::mem::transmute(self.scause) }
    }
    
    /// 设置返回地址
    pub fn set_return_addr(&mut self, addr: usize) {
        self.sepc = addr;
    }
}
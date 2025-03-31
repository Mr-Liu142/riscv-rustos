//! 中断向量表和入口点
//! 
//! 本模块实现RISC-V中断向量表的设置和中断入口点处理

use crate::println;
use core::arch::global_asm;
use riscv::register::{stvec, scause, sie, sip, sstatus};
use crate::trap::ds::{TrapMode, Interrupt, TrapContext};

// 导入汇编中断入口代码
global_asm!(include_str!("trap_entry.asm"));

// 声明汇编中定义的符号
extern "C" {
    /// 中断入口点函数
    fn __trap_entry();
    /// 从中断返回函数
    fn __trap_return();
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
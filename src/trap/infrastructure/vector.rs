//! 中断向量表和入口点

use crate::println;
use core::arch::global_asm;
use riscv::register::{stvec, scause};

// 导入汇编中断入口代码
global_asm!(include_str!("trap_entry.asm"));

// 声明汇编中定义的符号
extern "C" {
    fn __trap_entry();
    fn __trap_return();
}

/// 中断模式枚举
#[derive(Debug, Copy, Clone)]
pub enum TrapMode {
    Direct = 0,
    Vectored = 1,
}

/// 初始化中断向量表
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
    use riscv::register::sstatus;
    unsafe {
        sstatus::set_sie();
    }
}

/// 禁用所有中断
pub fn disable_interrupts() -> bool {
    use riscv::register::sstatus;
    let was_enabled = sstatus::read().sie();
    unsafe {
        sstatus::clear_sie();
    }
    was_enabled
}

/// 使用给定的前中断状态恢复中断设置
pub fn restore_interrupts(was_enabled: bool) {
    use riscv::register::sstatus;
    if was_enabled {
        unsafe {
            sstatus::set_sie();
        }
    }
}
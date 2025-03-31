//! RISC-V中断和异常处理模块

use crate::println;

// 导入子模块
pub mod infrastructure;

// 从基础设施导出主要API
pub use infrastructure::{
    init_trap_system,
    enable_interrupts,
    disable_interrupts,
    restore_interrupts,
};

// 中断类型枚举
#[derive(Debug, Copy, Clone)]
pub enum TrapType {
    TimerInterrupt,
    ExternalInterrupt,
    SoftwareInterrupt,
    SystemCall,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
    InstructionAccessFault,
    IllegalInstruction,
    Unknown,
}

/// 转换RISC-V中断原因为TrapType
pub fn decode_trap_cause(cause: riscv::register::scause::Scause) -> TrapType {
    // 使用正确的路径获取中断/异常类型
    if cause.is_interrupt() {
        match cause.code() {
            5 => TrapType::TimerInterrupt,
            9 => TrapType::ExternalInterrupt,
            1 => TrapType::SoftwareInterrupt,
            _ => TrapType::Unknown,
        }
    } else {
        match cause.code() {
            8 => TrapType::SystemCall,
            12 => TrapType::InstructionPageFault,
            13 => TrapType::LoadPageFault,
            15 => TrapType::StorePageFault,
            1 => TrapType::InstructionAccessFault,
            2 => TrapType::IllegalInstruction,
            _ => TrapType::Unknown,
        }
    }
}

/// 初始化整个中断系统
pub fn init() {
    // 初始化中断基础设施
    infrastructure::init_trap_system();
    
    println!("Trap system fully initialized");
}
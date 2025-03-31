//! 中断系统类型定义
//!
//! 定义中断系统所需的各种枚举类型和标志

use core::fmt;

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

/// 综合中断类型枚举
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

/// 中断原因封装
#[derive(Copy, Clone)]
pub struct TrapCause {
    bits: usize,
}

impl TrapCause {
    /// 从原始值创建中断原因
    pub const fn from_bits(bits: usize) -> Self {
        Self { bits }
    }
    
    /// 获取原始值
    pub const fn bits(&self) -> usize {
        self.bits
    }
    
    /// 判断是否为中断（而非异常）
    pub fn is_interrupt(&self) -> bool {
        self.bits & (1 << (core::mem::size_of::<usize>() * 8 - 1)) != 0
    }
    
    /// 获取中断/异常代码
    pub fn code(&self) -> usize {
        self.bits & !(1 << (core::mem::size_of::<usize>() * 8 - 1))
    }
    
    /// 转换为TrapType
    pub fn to_trap_type(&self) -> TrapType {
        if self.is_interrupt() {
            match self.code() {
                5 => TrapType::TimerInterrupt,
                9 => TrapType::ExternalInterrupt,
                1 => TrapType::SoftwareInterrupt,
                _ => TrapType::Unknown,
            }
        } else {
            match self.code() {
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
}

impl fmt::Debug for TrapCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TrapCause {{ interrupt: {}, code: {} }}", 
               self.is_interrupt(), self.code())
    }
}
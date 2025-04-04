//! Trap system type definitions
//!
//! Defines various enum types and flags needed for the trap system

use core::fmt;

/// Trap mode enum
#[derive(Debug, Copy, Clone)]
pub enum TrapMode {
    /// Direct mode - all traps use the same handler function
    Direct = 0,
    /// Vectored mode - different trap types use different handler functions
    Vectored = 1,
}

/// Interrupt type enum - only includes interrupts available in S mode
#[derive(Debug, Copy, Clone)]
pub enum Interrupt {
    SupervisorSoft = 1,
    SupervisorTimer = 5,
    SupervisorExternal = 9,
}

/// Exception type enum
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

/// Comprehensive trap type enum
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    Breakpoint,     // 新增断点类型
    InstructionMisaligned,
    LoadMisaligned,
    StoreMisaligned,
    LoadAccessFault,     // 新增：加载访问错误
    StoreAccessFault,    // 新增：存储访问错误
    Unknown,
}

/// Trap cause wrapper
#[derive(Copy, Clone)]
pub struct TrapCause {
    bits: usize,
}

impl TrapCause {
    /// Create trap cause from raw bits
    pub const fn from_bits(bits: usize) -> Self {
        Self { bits }
    }
    
    /// Get raw bits
    pub const fn bits(&self) -> usize {
        self.bits
    }
    
    /// Check if this is an interrupt (vs exception)
    pub fn is_interrupt(&self) -> bool {
        self.bits & (1 << (core::mem::size_of::<usize>() * 8 - 1)) != 0
    }
    
    /// Get the interrupt/exception code
    pub fn code(&self) -> usize {
        self.bits & !(1 << (core::mem::size_of::<usize>() * 8 - 1))
    }
    
    /// Convert to TrapType
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
                0 => TrapType::InstructionMisaligned,  // 新增：指令地址未对齐映射
                1 => TrapType::InstructionAccessFault,
                2 => TrapType::IllegalInstruction,
                3 => TrapType::Breakpoint,    // 新增断点异常映射
                4 => TrapType::LoadMisaligned,         // 新增：加载地址未对齐映射
                5 => TrapType::LoadAccessFault,    // 正确映射：加载访问错误
                6 => TrapType::StoreMisaligned,        // 新增：存储地址未对齐映射
                7 => TrapType::StoreAccessFault,   // 正确映射：存储访问错误
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

impl TrapType {
    /// Number of trap types
    pub const COUNT: usize = 15; // Includes all defined types
    
    /// Convert from index to trap type
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => TrapType::TimerInterrupt,
            1 => TrapType::ExternalInterrupt,
            2 => TrapType::SoftwareInterrupt,
            3 => TrapType::SystemCall,
            4 => TrapType::InstructionPageFault,
            5 => TrapType::LoadPageFault,
            6 => TrapType::StorePageFault,
            7 => TrapType::InstructionAccessFault,
            8 => TrapType::IllegalInstruction,
            9 => TrapType::Breakpoint,    // 新增断点映射
            10 => TrapType::InstructionMisaligned, // 新增
            11 => TrapType::LoadMisaligned,        // 新增
            12 => TrapType::StoreMisaligned,       // 新增
            13 => TrapType::LoadAccessFault,
            14 => TrapType::StoreAccessFault,
            _ => TrapType::Unknown,
        }
    }
}
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

// 导出上下文管理API
pub use infrastructure::{
    TaskContext,
    task_switch,
    prepare_task_context,
    trap_return,
    save_full_context,
    restore_full_context,
    TrapContext,
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

/// 上下文切换功能
/// 
/// 安全地封装底层的task_switch函数
/// 
/// # 参数
/// 
/// * `current` - 当前任务上下文指针
/// * `next` - 下一个任务上下文指针
pub fn switch_to_context(current: &mut TaskContext, next: &TaskContext) {
    unsafe {
        infrastructure::task_switch(current, next);
    }
}

/// 创建任务上下文
/// 
/// 封装prepare_task_context函数，提供更简单的接口
/// 
/// # 参数
/// 
/// * `entry` - 任务入口点函数
/// * `user_stack` - 用户栈顶
/// * `kernel_stack` - 内核栈顶
/// 
/// # 返回值
/// 
/// 返回配置好的陷阱上下文
pub fn create_task_context(
    entry: usize,
    user_stack: usize,
    kernel_stack: usize
) -> TrapContext {
    infrastructure::prepare_task_context(entry, user_stack, kernel_stack, 0)
}
//! RISC-V中断和异常处理模块

use crate::println;

// 导入子模块
pub mod infrastructure;
pub mod ds;  // 数据结构模块

// 从基础设施导出主要API
pub use infrastructure::{
    init_trap_system,
    enable_interrupts,
    disable_interrupts,
    restore_interrupts,
};

// 从数据结构模块导出类型
pub use ds::{TrapContext, TaskContext, TrapType, TrapCause, TrapHandler, TrapHandlerResult, TrapError};

// 从基础设施导出任务切换函数
pub use infrastructure::{
    task_switch,
    prepare_task_context,
    trap_return,
    save_full_context,
    restore_full_context,
};

// 导出处理器注册API
pub use infrastructure::{
    register_handler,
    unregister_handler,
    handler_count,
    print_handlers,
};

/// 转换RISC-V中断原因为TrapType
pub fn decode_trap_cause(cause: riscv::register::scause::Scause) -> TrapType {
    // 使用新的TrapCause类型包装scause
    let trap_cause = TrapCause::from_bits(cause.bits());
    trap_cause.to_trap_type()
}

/// 初始化整个中断系统
pub fn init() {
    // 初始化中断基础设施
    infrastructure::init_trap_system();
    
    println!("中断系统完全初始化");
}

/// 上下文切换功能
pub fn switch_to_context(current: &mut TaskContext, next: &TaskContext) {
    unsafe {
        infrastructure::task_switch(current, next);
    }
}

/// 创建任务上下文
pub fn create_task_context(
    entry: usize,
    user_stack: usize,
    kernel_stack: usize
) -> TrapContext {
    infrastructure::prepare_task_context(entry, user_stack, kernel_stack, 0)
}
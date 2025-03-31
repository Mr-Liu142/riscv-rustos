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
pub use ds::{
    TrapContext, TaskContext, TrapType, TrapCause, 
    TrapHandler, TrapHandlerResult, TrapError, 
    ContextManager, ContextError, ContextType, ContextState,
    InterruptContextGuard, is_in_interrupt_context, get_interrupt_nest_level,
};

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


// 从上下文管理器导出全局API
pub use ds::{
    init_global_context_manager,
    get_context_manager,
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

    // 初始化全局上下文管理器
    ds::init_global_context_manager();
    
    println!("中断系统完全初始化");
}

/// 上下文切换功能
pub fn switch_to_context(current: &mut TaskContext, next: &TaskContext) {
    // 使用全局上下文管理器
    let manager = ds::get_context_manager();
    manager.switch_task_context(current, next);
}

/// 创建任务上下文
pub fn create_task_context(
    entry: usize,
    user_stack: usize,
    kernel_stack: usize
) -> TrapContext {
    // 使用全局上下文管理器
    let manager = ds::get_context_manager();
    manager.create_task_context(entry, user_stack, kernel_stack, 0)
}
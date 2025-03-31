//! 中断基础设施模块
//!
//! 提供中断系统基础功能和API

mod vector;
mod context; // 现在已实现上下文管理
mod registry; // 新增：处理器注册模块
//mod handler; // 将在后续实现 
//mod csr;     // 将在后续实现
pub mod test; // 测试功能公开

use crate::println;
use crate::trap::ds::{TrapContext, TaskContext, TrapMode, Interrupt, Exception, TrapType, TrapHandlerResult, TrapError};

// 对外导出API
pub use vector::{
    init, 
    enable_interrupts, 
    disable_interrupts, 
    restore_interrupts,
    enable_interrupt,
    disable_interrupt,
    is_interrupt_enabled,
    is_interrupt_pending,
    set_soft_interrupt,
    clear_soft_interrupt,
};

// 导出上下文管理API
pub use context::{
    task_switch,
    prepare_task_context,
    trap_return,
    save_full_context,
    restore_full_context,
    create_test_context,
    test_context_switch,
};

// 导出处理器注册API
pub use registry::{
    register_handler,
    unregister_handler,
    dispatch_trap,
    handler_count,
    print_handlers,
};

/// 初始化中断系统
///
/// 这个函数完成基础中断系统的初始化工作
pub fn init_trap_system() {
    // 初始化中断向量表，使用直接模式
    vector::init(TrapMode::Direct);

    // 注册默认处理器
    register_default_handlers();
    
    println!("Trap infrastructure initialized");
}

/// 注册默认处理器
fn register_default_handlers() {
    // 时钟中断默认处理器
    registry::register_handler(
        TrapType::TimerInterrupt,
        default_timer_handler,
        100, // 低优先级，允许用户注册更高优先级的处理器
        "Default Timer Handler"
    );
    
    // 软件中断默认处理器
    registry::register_handler(
        TrapType::SoftwareInterrupt,
        default_software_handler,
        100,
        "Default Software Handler"
    );
    
    // 外部中断默认处理器
    registry::register_handler(
        TrapType::ExternalInterrupt,
        default_external_handler,
        100,
        "Default External Handler"
    );
    
    // 系统调用默认处理器
    registry::register_handler(
        TrapType::SystemCall,
        default_syscall_handler,
        100,
        "Default System Call Handler"
    );
    
    // 页面错误默认处理器
    registry::register_handler(
        TrapType::InstructionPageFault,
        default_page_fault_handler,
        100,
        "Default Page Fault Handler"
    );
    registry::register_handler(
        TrapType::LoadPageFault,
        default_page_fault_handler,
        100,
        "Default Page Fault Handler"
    );
    registry::register_handler(
        TrapType::StorePageFault,
        default_page_fault_handler,
        100,
        "Default Page Fault Handler"
    );
    
    // 非法指令默认处理器
    registry::register_handler(
        TrapType::IllegalInstruction,
        default_illegal_instruction_handler,
        100,
        "Default Illegal Instruction Handler"
    );
    
    // 未知中断默认处理器
    registry::register_handler(
        TrapType::Unknown,
        default_unknown_handler,
        100,
        "Default Unknown Handler"
    );
}

// 默认处理器实现
fn default_timer_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Timer interrupt occurred");
    TrapHandlerResult::Handled
}

fn default_software_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Software interrupt occurred");
    vector::clear_soft_interrupt();
    TrapHandlerResult::Handled
}

fn default_external_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("External interrupt occurred");
    TrapHandlerResult::Handled
}

fn default_syscall_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("System call occurred");
    // 系统调用返回时，PC需要加4跳过ecall指令
    ctx.set_return_addr(ctx.sepc + 4);
    TrapHandlerResult::Handled
}

fn default_page_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("页错误发生，地址: {:#x}", ctx.stval);
    TrapHandlerResult::Handled
}

fn default_illegal_instruction_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("非法指令: {:#x}", ctx.stval);
    TrapHandlerResult::Handled
}

fn default_unknown_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("未知中断: cause={:#x}, addr={:#x}", ctx.scause, ctx.stval);
    TrapHandlerResult::Handled
}

/// Interrupt handler function
/// 
/// This function is the central entry point for all traps/interrupts in the system.
/// It dispatches the interrupt to appropriate registered handlers based on the interrupt type.
/// 
/// # Parameters
/// 
/// * `context` - Pointer to the trap context saved by the assembly entry point
/// 中断处理函数
/// 
/// 参数为指向上下文结构的指针
#[no_mangle]
pub extern "C" fn handle_trap(context: *mut TrapContext) {
    // 通过上下文管理器创建中断守卫
    let mut ctx = unsafe { &mut *context };
    let cause = ctx.get_cause();
    
    // 记录当前嵌套层级
    let nest_level = crate::trap::ds::get_interrupt_nest_level();
    
    // 转换中断/异常为TrapType
    let trap_type = cause.to_trap_type();
    
    // 记录中断发生
    if cause.is_interrupt() {
        println!("Interrupt occurred: {:?}, code: {}, nest level: {}", 
                 trap_type, cause.code(), nest_level);
    } else {
        println!("Exception occurred: {:?}, code: {}, addr: {:#x}, nest level: {}", 
                 trap_type, cause.code(), ctx.stval, nest_level);
    }
    
    // 分发给注册的处理器
    match registry::dispatch_trap(trap_type, ctx) {
        TrapHandlerResult::Handled => {
            // 处理成功，无需额外操作
            println!("Interrupt handled successfully by registered handler");
        },
        TrapHandlerResult::Pass => {
            // 所有处理器都传递了此中断，使用默认处理
            println!("All handlers passed the interrupt: {:?}", trap_type);
            
            // 默认处理逻辑...
            if cause.is_interrupt() {
                match trap_type {
                    TrapType::TimerInterrupt => {
                        println!("Fallback handling for timer interrupt");
                    },
                    TrapType::SoftwareInterrupt => {
                        vector::clear_soft_interrupt();
                    },
                    TrapType::ExternalInterrupt => {
                        println!("Fallback handling for external interrupt");
                    },
                    _ => {
                        println!("No fallback handler for interrupt type: {:?}", trap_type);
                    }
                }
            } else {
                // 异常处理
                match trap_type {
                    TrapType::SystemCall => {
                        println!("Fallback handling for system call");
                        // 系统调用返回时，PC需要加4跳过ecall指令
                        ctx.set_return_addr(ctx.sepc + 4);
                    },
                    TrapType::InstructionPageFault | 
                    TrapType::LoadPageFault | 
                    TrapType::StorePageFault => {
                        println!("Unhandled page fault at address {:#x}", ctx.stval);
                    },
                    _ => {
                        println!("Unhandled exception: {:?} at {:#x}", trap_type, ctx.sepc);
                    }
                }
            }
        },
        TrapHandlerResult::Failed(err) => {
            // 处理失败
            println!("Failed to handle interrupt: {:?}, error: {:?}", trap_type, err);
        }
    }
    
    println!("Exiting trap handler for {:?}, nest level: {}", trap_type, nest_level);
}
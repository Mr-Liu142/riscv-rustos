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
#[no_mangle]
pub extern "C" fn handle_trap(context: *mut TrapContext) {
    let ctx = unsafe { &mut *context };
    let cause = ctx.get_cause();
    
    // Convert interrupt/exception to TrapType for handler dispatch
    let trap_type = cause.to_trap_type();
    
    // Log the trap occurrence with detailed information
    if cause.is_interrupt() {
        println!("Interrupt occurred: {:?}, code: {}", trap_type, cause.code());
    } else {
        println!("Exception occurred: {:?}, code: {}, addr: {:#x}", 
                 trap_type, cause.code(), ctx.stval);
    }
    
    // Dispatch to registered handlers with priority handling
    match registry::dispatch_trap(trap_type, ctx) {
        TrapHandlerResult::Handled => {
            // Successfully handled, nothing more to do
            println!("Interrupt handled successfully by registered handler");
        },
        TrapHandlerResult::Pass => {
            // All handlers passed this interrupt, apply fallback handling
            println!("All handlers passed the interrupt: {:?}", trap_type);
            
            // Basic fallback logic for common interrupts
            if cause.is_interrupt() {
                match trap_type {
                    TrapType::TimerInterrupt => {
                        println!("Fallback handling for timer interrupt");
                        // Could reset timer or acknowledge interrupt here
                    },
                    TrapType::SoftwareInterrupt => {
                        println!("Fallback handling for software interrupt");
                        vector::clear_soft_interrupt();
                    },
                    TrapType::ExternalInterrupt => {
                        println!("Fallback handling for external interrupt");
                        // Could acknowledge external interrupt controller here
                    },
                    _ => {
                        println!("No fallback handler for interrupt type: {:?}", trap_type);
                    }
                }
            } else {
                // Exception handling
                match trap_type {
                    TrapType::SystemCall => {
                        println!("Fallback handling for system call");
                        // Advance PC past the ecall instruction
                        ctx.set_return_addr(ctx.sepc + 4);
                    },
                    TrapType::InstructionPageFault | 
                    TrapType::LoadPageFault | 
                    TrapType::StorePageFault => {
                        println!("Unhandled page fault at address {:#x}", ctx.stval);
                        // In a real system, this might try to load the page or terminate the process
                    },
                    _ => {
                        println!("Unhandled exception: {:?} at {:#x}", trap_type, ctx.sepc);
                    }
                }
            }
        },
        TrapHandlerResult::Failed(err) => {
            // Handler execution failed
            println!("Failed to handle interrupt: {:?}, error: {:?}", trap_type, err);
            
            // For critical failures, we might want to take emergency action
            match err {
                TrapError::NoHandler => {
                    println!("No handler registered for this trap type");
                },
                TrapError::HandlerFailed => {
                    println!("Handler execution failed, possible system instability");
                    // Could potentially trigger system reset in production
                },
                TrapError::Unknown => {
                    println!("Unknown error during trap handling");
                }
            }
        }
    }
    
    // Log trap exit
    println!("Exiting trap handler for {:?}", trap_type);
}
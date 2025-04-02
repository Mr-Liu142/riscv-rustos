//! Trap system infrastructure module
//!
//! Provides the core functionality and API for the trap system

mod vector;
mod context;
mod registry;
pub mod test;
pub mod di;  // New dependency injection module
pub mod error_handler;  // Error handling module
pub mod error_test;  // Error handling tests
pub mod enhanced_handlers;  // 增强型异常处理器
pub mod test_enhanced;  // 增强型异常处理器测试

use crate::println;
use crate::trap::ds::{TrapContext, TaskContext, TrapMode, Interrupt, Exception, TrapType, TrapHandlerResult, TrapError};

// Export APIs from submodules
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

// Export context management API
pub use context::{
    task_switch,
    prepare_task_context,
    trap_return,
    save_full_context,
    restore_full_context,
    create_test_context,
    test_context_switch,
};

// Export handler registry API
pub use registry::{
    register_handler,
    unregister_handler,
    dispatch_trap,
    handler_count,
    print_handlers,
};

// Export error handling API with renamed functions
pub use error_handler::{
    init as init_error_system,
    register_handler as register_error_handler,
    unregister_handler as unregister_error_handler,
    handle_error as handle_system_error,
    create_error as create_system_error,
    print_handlers as print_error_handlers,
    print_error_log,
    clear_error_log,
    is_panic_mode as is_in_panic_mode,
    reset_panic_mode,
};

/// Initialize the trap system
///
/// This function initializes the basic trap system
pub fn init_trap_system() {
    // Initialize the trap vector with direct mode
    vector::init(TrapMode::Direct);

    // Register default handlers
    register_default_handlers();
    
    println!("Trap infrastructure initialized");
}

/// Register default handlers
fn register_default_handlers() {
    // Timer interrupt default handler
    registry::register_handler(
        TrapType::TimerInterrupt,
        default_timer_handler,
        100, // Low priority, allows user to register higher priority handlers
        "Default Timer Handler"
    );
    
    // Software interrupt default handler
    registry::register_handler(
        TrapType::SoftwareInterrupt,
        default_software_handler,
        100,
        "Default Software Handler"
    );
    
    // External interrupt default handler
    registry::register_handler(
        TrapType::ExternalInterrupt,
        default_external_handler,
        100,
        "Default External Handler"
    );
    
    // System call default handler
    registry::register_handler(
        TrapType::SystemCall,
        default_syscall_handler,
        100,
        "Default System Call Handler"
    );
    
    // Page fault default handlers
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
    
    // Illegal instruction default handler
    registry::register_handler(
        TrapType::IllegalInstruction,
        default_illegal_instruction_handler,
        100,
        "Default Illegal Instruction Handler"
    );

    // Breakpoint default handler
    registry::register_handler(
        TrapType::Breakpoint,
        default_breakpoint_handler,
        100,
        "Default Breakpoint Handler"
    );
    
    // Unknown trap default handler
    registry::register_handler(
        TrapType::Unknown,
        default_unknown_handler,
        100,
        "Default Unknown Handler"
    );
}

// Default handler implementations
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
    // System calls need to advance PC past the ecall instruction
    ctx.set_return_addr(ctx.sepc + 4);
    TrapHandlerResult::Handled
}

fn default_page_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Page fault occurred, address: {:#x}", ctx.stval);
    TrapHandlerResult::Handled
}

fn default_illegal_instruction_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Illegal instruction: {:#x}", ctx.stval);
    TrapHandlerResult::Handled
}

fn default_breakpoint_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Breakpoint occurred at: {:#x}", ctx.sepc);
    // 断点处理需要手动前进PC
    ctx.set_return_addr(ctx.sepc + 4);
    TrapHandlerResult::Handled
}

fn default_unknown_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Unknown trap: cause={:#x}, addr={:#x}", ctx.scause, ctx.stval);
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
    // If the DI system is initialized, use it
    if di::get_trap_system_initialized() {
        // DI system will handle the trap
        di::internal_handle_trap(context);
        return;
    }
    
    // Otherwise, fall back to the original implementation
    let mut ctx = unsafe { &mut *context };
    let cause = ctx.get_cause();
    
    // Record current nesting level
    let nest_level = crate::trap::ds::get_interrupt_nest_level();
    
    // Convert trap/exception to TrapType
    let trap_type = cause.to_trap_type();
    
    // Record trap occurrence
    if cause.is_interrupt() {
        println!("Interrupt occurred: {:?}, code: {}, nest level: {}", 
                 trap_type, cause.code(), nest_level);
    } else {
        println!("Exception occurred: {:?}, code: {}, addr: {:#x}, nest level: {}", 
                 trap_type, cause.code(), ctx.stval, nest_level);
    }
    
    // Dispatch to registered handlers
    match registry::dispatch_trap(trap_type, ctx) {
        TrapHandlerResult::Handled => {
            // Successfully handled
            println!("Interrupt handled successfully by registered handler");
        },
        TrapHandlerResult::Pass => {
            // All handlers passed this interrupt
            println!("All handlers passed the interrupt: {:?}", trap_type);
            
            // Default handling logic...
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
                // Exception handling
                match trap_type {
                    TrapType::SystemCall => {
                        println!("Fallback handling for system call");
                        // System calls need to advance PC past the ecall instruction
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
            // Handling failed
            println!("Failed to handle interrupt: {:?}, error: {:?}", trap_type, err);
        }
    }
    
    println!("Exiting trap handler for {:?}, nest level: {}", trap_type, nest_level);
}
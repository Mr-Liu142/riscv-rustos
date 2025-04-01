//! RISC-V trap and exception handling module

use crate::println;

// Import submodules
pub mod infrastructure;
pub mod ds;  // Data structures module

// Export DI system from infrastructure module
pub use infrastructure::di::{
    initialize_trap_system,
    register_handler,
    unregister_handler,
    handler_count,
    print_handlers,
    enable_interrupts,
    disable_interrupts,
    restore_interrupts,
    is_in_interrupt_context,
    get_interrupt_nest_level,
    create_task_context,
    switch_task_context,
};

// Export task context handling functions from infrastructure
pub use infrastructure::{
    task_switch,
    prepare_task_context,
    trap_return,
    save_full_context,
    restore_full_context,
};

// Export types from data structures module
pub use ds::{
    TrapContext, TaskContext, TrapType, TrapCause, 
    TrapHandler, TrapHandlerResult, TrapError, 
    ContextManager, ContextError, ContextType, ContextState,
    InterruptContextGuard, TrapMode, Interrupt, Exception,
};

// Export error handling system
pub use infrastructure::{
    init_error_system,
    register_error_handler,
    unregister_error_handler,
    handle_system_error,
    create_system_error,
    print_error_handlers,
    print_error_log,
    clear_error_log,
    is_in_panic_mode,
    reset_panic_mode,
};


/// Initialize the trap system
pub fn init() {
    // Initialize the trap system using the DI system
    infrastructure::di::initialize_trap_system(TrapMode::Direct);
    
    // Initialize global context manager (for backward compatibility)
    ds::init_global_context_manager();

    // Initialize error handling system
    infrastructure::error_handler::init();
    
    println!("Trap system fully initialized");
}

/// Convert RISC-V trap cause to TrapType
pub fn decode_trap_cause(cause: riscv::register::scause::Scause) -> TrapType {
    // Use the TrapCause wrapper to convert scause
    let trap_cause = TrapCause::from_bits(cause.bits());
    trap_cause.to_trap_type()
}
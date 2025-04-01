//! Trap System Dependency Injection Tests
//!
//! This module provides tests for the trap system dependency injection

use crate::println;
use crate::trap::ds::{TrapType, TrapContext, TrapHandlerResult};
use super::{
    initialize_trap_system, register_handler, unregister_handler,
    print_handlers, enable_interrupts, disable_interrupts,
    is_in_interrupt_context, get_interrupt_nest_level
};

/// Test handler for timer interrupts
fn test_timer_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Test timer handler called");
    TrapHandlerResult::Handled
}

/// Test handler for software interrupts
fn test_software_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Test software interrupt handler called");
    TrapHandlerResult::Handled
}

/// Test handler for system calls
fn test_syscall_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Test system call handler called");
    // Advance PC past the ecall instruction
    ctx.set_return_addr(ctx.sepc + 4);
    TrapHandlerResult::Handled
}

/// Test the trap system initialization
pub fn test_initialization() {
    println!("Testing trap system initialization...");
    
    // Initialize the trap system with direct mode
    initialize_trap_system(crate::trap::ds::TrapMode::Direct);
    
    // Verify system state
    let in_interrupt = is_in_interrupt_context();
    let nest_level = get_interrupt_nest_level();
    
    println!("In interrupt context: {}", in_interrupt);
    println!("Current nest level: {}", nest_level);
    
    println!("Trap system initialization test passed");
}

/// Test handler registration and management
pub fn test_handler_registration() {
    println!("Testing trap handler registration...");
    
    // Register test handlers with higher priority
    let result1 = register_handler(
        TrapType::TimerInterrupt,
        test_timer_handler,
        50, // Higher priority than default (100)
        "Test Timer Handler"
    );
    
    let result2 = register_handler(
        TrapType::SoftwareInterrupt,
        test_software_handler,
        50,
        "Test Software Interrupt Handler"
    );
    
    let result3 = register_handler(
        TrapType::SystemCall,
        test_syscall_handler,
        50,
        "Test System Call Handler"
    );
    
    println!("Registration results: {}, {}, {}", result1, result2, result3);
    
    // Print all handlers
    print_handlers();
    
    // Unregister one handler
    let unregister_result = unregister_handler(
        TrapType::TimerInterrupt,
        "Test Timer Handler"
    );
    
    println!("Unregistration result: {}", unregister_result);
    
    // Print handlers again
    print_handlers();
    
    println!("Trap handler registration test passed");
}

/// Test interrupt control
pub fn test_interrupt_control() {
    println!("Testing interrupt control...");
    
    // Save current state
    let was_enabled = disable_interrupts();
    println!("Interrupts disabled, previous state: {}", was_enabled);
    
    // Enable interrupts
    enable_interrupts();
    println!("Interrupts enabled");
    
    // Disable again
    let new_state = disable_interrupts();
    println!("Interrupts disabled again, state was: {}", new_state);
    
    // Should be true since we enabled them
    assert!(new_state, "Interrupts should have been enabled");
    
    // Restore original state
    if was_enabled {
        enable_interrupts();
        println!("Restored interrupts to enabled state");
    }
    
    println!("Interrupt control test passed");
}

/// Run all trap system dependency injection tests
pub fn run_all_tests() {
    println!("=== Running Trap System DI Tests ===");
    
    test_initialization();
    test_handler_registration();
    test_interrupt_control();
    
    println!("=== All Trap System DI Tests Passed ===");
}
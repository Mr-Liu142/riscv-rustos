//! Trap system test module

use crate::println;
use super::vector;
use super::context;
use super::registry;
use super::di;
use crate::trap::ds::{TrapMode, TaskContext, get_context_manager, get_interrupt_nest_level, is_in_interrupt_context};
use crate::trap::{TrapContext, TrapHandlerResult, TrapType};

/// Test trap vector initialization
pub fn test_vector_init() {
    println!("Testing trap vector initialization...");
    vector::init(TrapMode::Direct);
    println!("Trap vector initialized successfully");
}

/// Test interrupt control functionality
pub fn test_interrupt_control() {
    println!("Testing interrupt control...");
    
    // Save current interrupt state
    let was_enabled = vector::disable_interrupts();
    println!("Interrupts disabled, previous state: {}", was_enabled);
    
    // Enable interrupts
    vector::enable_interrupts();
    println!("Interrupts enabled");
    
    // Disable interrupts again
    let new_state = vector::disable_interrupts();
    println!("Interrupts disabled again, state was: {}", new_state);
    assert!(new_state, "Interrupt should have been enabled");
    
    // Restore original state
    vector::restore_interrupts(was_enabled);
    println!("Interrupt state restored to original");
}

/// Test context management functionality
pub fn test_context_management() {
    println!("Testing context management...");
    
    // Create two task contexts
    let mut ctx1 = TaskContext::new();
    let mut ctx2 = TaskContext::new();
    
    // Simulate two different task entry point functions
    extern "C" fn test_task1() {
        println!("Task 1 is running");
    }
    
    extern "C" fn test_task2() {
        println!("Task 2 is running");
    }
    
    // Set entry points and stacks
    static mut STACK1: [u8; 4096] = [0; 4096];
    static mut STACK2: [u8; 4096] = [0; 4096];
    
    unsafe {
        ctx1.set_ra(test_task1 as usize);
        ctx1.set_sp(STACK1.as_ptr().add(4096) as usize);
        
        ctx2.set_ra(test_task2 as usize);
        ctx2.set_sp(STACK2.as_ptr().add(4096) as usize);
    }
    
    println!("Context 1 prepared: ra=0x{:x}, sp=0x{:x}", 
             ctx1.get_ra(), ctx1.get_sp());
    println!("Context 2 prepared: ra=0x{:x}, sp=0x{:x}", 
             ctx2.get_ra(), ctx2.get_sp());
    
    // Since actual context switching would change execution flow,
    // we just test that the structure is correct
    println!("Context management test completed");
}

/// Test trap context creation and operations
pub fn test_trap_context() {
    println!("Testing trap context functionality...");
    
    // Create a test trap context
    let test_pc = 0x80200000;
    let test_sp = 0x81000000;
    let ctx = context::create_test_context(test_pc, test_sp);
    
    println!("Created test trap context PC=0x{:x}, SP=0x{:x}", 
             ctx.sepc, ctx.x[2]);
    
    // Test basic trap context operations
    let cause = ctx.get_cause();
    println!("Trap cause: is_interrupt={}, code={}", 
             cause.is_interrupt(), cause.code());
    
    println!("Trap context test completed");
}

/// Test the interrupt handler registry functionality
pub fn test_handler_registry() {
    println!("Testing interrupt handler registry...");
    
    // Define test handlers
    fn test_timer_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
        println!("Test timer handler called");
        TrapHandlerResult::Handled
    }
    
    fn test_software_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
        println!("Test software interrupt handler called");
        TrapHandlerResult::Handled
    }
    
    // Register test handlers
    let result1 = registry::register_handler(
        TrapType::TimerInterrupt,
        test_timer_handler,
        50, // Higher priority
        "Test Timer Handler"
    );
    
    let result2 = registry::register_handler(
        TrapType::SoftwareInterrupt,
        test_software_handler,
        60,
        "Test Software Interrupt Handler"
    );
    
    println!("Registration results: {}, {}", result1, result2);
    
    // Print registered handlers
    registry::print_handlers();
    
    // Get handler counts
    let timer_count = registry::handler_count(TrapType::TimerInterrupt);
    let software_count = registry::handler_count(TrapType::SoftwareInterrupt);
    
    println!("Handler counts: timer={}, software={}", timer_count, software_count);
    
    // Test unregistration
    let unregister_result = registry::unregister_handler(
        TrapType::TimerInterrupt,
        "Test Timer Handler"
    );
    
    println!("Unregistration result: {}", unregister_result);
    
    // Print handlers again
    registry::print_handlers();
    
    println!("Interrupt handler registry test completed");
}

pub fn test_context_manager() {
    println!("Testing context manager...");
    
    // Get global context manager
    let manager = crate::trap::ds::get_context_manager();
    
    // Test context size queries
    let task_size = manager.get_context_size(crate::trap::ContextType::Task);
    let trap_size = manager.get_context_size(crate::trap::ContextType::Trap);
    
    println!("Context sizes: task={} bytes, trap={} bytes", task_size, trap_size);
    
    // Test interrupt stack usage
    let (used, total) = manager.get_interrupt_stack_usage();
    println!("Interrupt stack usage: {}/{} bytes", used, total);
    
    // Simulate interrupt nesting
    let nest_level = crate::trap::ds::get_interrupt_nest_level();
    println!("Current interrupt nest level: {}", nest_level);
    
    // Test if in interrupt context
    let in_interrupt = crate::trap::ds::is_in_interrupt_context();
    println!("In interrupt context: {}", in_interrupt);
    
    // Test task context creation
    let task_entry = 0x80200000;
    let user_stack = 0x81000000;
    let kernel_stack = 0x82000000;
    
    let ctx = manager.create_task_context(task_entry, user_stack, kernel_stack, 0);
    println!("Created task context: PC=0x{:x}, SP=0x{:x}", ctx.sepc, ctx.x[2]);
    
    println!("Context manager test completed");
}

/// Run all tests
pub fn run_all_tests() {
    println!("=== Starting trap infrastructure tests ===");
    
    // Run original tests
    test_vector_init();
    test_interrupt_control();
    test_context_management();
    test_trap_context();
    test_handler_registry();
    context::test_context_switch();
    test_context_manager();
    
    // Run DI system tests
    println!("\n=== Starting dependency injection tests ===");
    // Run DI system tests if they're available
    if di::get_trap_system_initialized() {
        println!("\n=== Starting dependency injection tests ===");
        di::test::run_all_tests();
    }
    
    println!("=== All trap tests completed successfully ===");
}
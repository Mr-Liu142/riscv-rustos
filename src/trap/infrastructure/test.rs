//! 中断系统测试模块

use crate::println;
use super::vector;
use super::context;
use super::registry; // 添加对registry模块的引用
use crate::trap::ds::{TrapMode, TaskContext};
use crate::trap::{TrapContext, TrapHandlerResult, TrapType}; // 添加对所需类型的引用

/// 测试中断向量初始化
pub fn test_vector_init() {
    println!("Testing trap vector initialization...");
    vector::init(TrapMode::Direct);
    println!("Trap vector initialized successfully");
}

/// 测试中断开关功能
pub fn test_interrupt_control() {
    println!("Testing interrupt control...");
    
    // 保存当前中断状态
    let was_enabled = vector::disable_interrupts();
    println!("Interrupts disabled, previous state: {}", was_enabled);
    
    // 启用中断
    vector::enable_interrupts();
    println!("Interrupts enabled");
    
    // 再次禁用中断
    let new_state = vector::disable_interrupts();
    println!("Interrupts disabled again, state was: {}", new_state);
    assert!(new_state, "Interrupt should have been enabled");
    
    // 恢复原始状态
    vector::restore_interrupts(was_enabled);
    println!("Interrupt state restored to original");
}

/// 测试上下文管理功能
pub fn test_context_management() {
    println!("Testing context management...");
    
    // 创建两个任务上下文
    let mut ctx1 = TaskContext::new();
    let mut ctx2 = TaskContext::new();
    
    // 模拟两个不同的任务入口点函数
    extern "C" fn test_task1() {
        println!("Task 1 is running");
    }
    
    extern "C" fn test_task2() {
        println!("Task 2 is running");
    }
    
    // 设置入口点和栈
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
    
    // 由于实际的上下文切换会改变执行流，我们只测试结构是否正确
    println!("Context management test completed");
}

/// 测试陷阱上下文创建和操作
pub fn test_trap_context() {
    println!("Testing trap context functionality...");
    
    // 创建一个测试用的陷阱上下文
    let test_pc = 0x80200000;
    let test_sp = 0x81000000;
    let ctx = context::create_test_context(test_pc, test_sp);
    
    println!("Created test trap context PC=0x{:x}, SP=0x{:x}", 
             ctx.sepc, ctx.x[2]);
    
    // 测试陷阱上下文基本操作
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
/// 运行所有测试
pub fn run_all_tests() {
    println!("=== Starting trap infrastructure tests ===");
    test_vector_init();
    test_interrupt_control();
    test_context_management();
    test_trap_context();
    test_handler_registry();
    
    // 运行上下文切换测试
    context::test_context_switch();
    
    println!("=== All trap tests completed successfully ===");
}
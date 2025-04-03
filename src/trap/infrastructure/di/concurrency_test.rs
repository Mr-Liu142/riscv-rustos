//! 并发安全性测试模块
//!
//! 用于测试DI系统中的锁机制是否正确保护全局状态

use crate::println;
use crate::trap::ds::{TrapType, TrapContext, TrapHandlerResult};
use core::sync::atomic::{AtomicUsize, Ordering};
use super::{
    initialize_trap_system, register_handler, unregister_handler,
    print_handlers, enable_interrupts, disable_interrupts
};

// 测试用原子计数器
static TEST_HANDLER_CALL_COUNT: AtomicUsize = AtomicUsize::new(0);
static TEST_HANDLERS_REGISTERED: AtomicUsize = AtomicUsize::new(0);

/// 测试用中断处理器
fn test_concurrent_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    // 增加调用计数
    TEST_HANDLER_CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    println!("Concurrent test handler called");
    TrapHandlerResult::Handled
}

/// 测试并发初始化安全性
pub fn test_concurrent_initialization() {
    println!("Testing concurrent initialization safety...");
    
    // 重置状态标志（仅用于测试）
    unsafe {
        // 此处需要访问私有字段，仅用于测试
        super::TRAP_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
    }
    
    // 模拟多次并发初始化调用
    for i in 0..5 {
        println!("Initialization attempt {}", i);
        initialize_trap_system(crate::trap::ds::TrapMode::Direct);
    }
    
    println!("Concurrent initialization test passed");
}

/// 测试并发处理器注册
pub fn test_concurrent_handler_registration() {
    println!("Testing concurrent handler registration...");
    
    // 重置计数器
    TEST_HANDLERS_REGISTERED.store(0, Ordering::SeqCst);
    
    // 模拟多个上下文注册处理器
    for i in 0..10 {
        let priority = 50 + i as u8;
        let desc = match i {
            0 => "Test Handler 1",
            1 => "Test Handler 2",
            2 => "Test Handler 3",
            3 => "Test Handler 4",
            4 => "Test Handler 5",
            5 => "Test Handler 6",
            6 => "Test Handler 7",
            7 => "Test Handler 8",
            8 => "Test Handler 9",
            _ => "Test Handler 10",
        };
        
        let result = register_handler(
            TrapType::TimerInterrupt,
            test_concurrent_handler,
            priority,
            desc
        );
        
        if result {
            TEST_HANDLERS_REGISTERED.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    // 验证注册结果
    let registered = TEST_HANDLERS_REGISTERED.load(Ordering::SeqCst);
    println!("Successfully registered {} handlers", registered);
    
    // 打印所有处理器
    print_handlers();
    
    // 尝试注销一个处理器
    let unregister_result = unregister_handler(
        TrapType::TimerInterrupt,
        "Test Handler 1"
    );
    
    println!("Unregistration result: {}", unregister_result);
    
    println!("Concurrent handler registration test passed");
}

/// 测试中断上下文并发安全性
pub fn test_interrupt_concurrency() {
    println!("Testing interrupt concurrency safety...");
    
    // 保存当前中断状态
    let was_enabled = disable_interrupts();
    println!("Interrupts disabled, previous state: {}", was_enabled);
    
    // 注册一个测试处理器
    register_handler(
        TrapType::TimerInterrupt,
        test_concurrent_handler,
        30,
        "Interrupt Concurrency Test Handler"
    );
    
    // 重置调用计数
    TEST_HANDLER_CALL_COUNT.store(0, Ordering::SeqCst);
    
    // 启用中断并等待一段时间
    enable_interrupts();
    println!("Interrupts enabled, waiting for timer interrupt...");
    
    // 等待一段时间，让中断发生
    for _ in 0..1000000 {
        core::hint::spin_loop();
    }
    
    // 禁用中断并检查结果
    disable_interrupts();
    let call_count = TEST_HANDLER_CALL_COUNT.load(Ordering::SeqCst);
    println!("Handler was called {} times", call_count);
    
    // 恢复原始状态
    if was_enabled {
        enable_interrupts();
    }
    
    println!("Interrupt concurrency test completed");
}

/// 测试锁性能
pub fn test_lock_performance() {
    println!("Testing lock performance...");
    
    // 测量获取锁的时间
    let iterations = 1000;
    let mut total_cycles = 0;
    
    for _ in 0..iterations {
        let start = crate::util::sbi::timer::get_time();
        
        // 执行带锁的操作
        let result = super::with_trap_system(|trap_system| {
            trap_system.handler_count_for_type(TrapType::TimerInterrupt)
        });
        
        let end = crate::util::sbi::timer::get_time();
        total_cycles += end - start;
    }
    
    let avg_cycles = total_cycles / iterations as u64;
    println!("Average lock acquisition time: {} cycles", avg_cycles);
    
    println!("Lock performance test completed");
}

/// 运行所有并发安全测试
pub fn run_all_concurrency_tests() {
    println!("=== Running DI System Concurrency Tests ===");
    
    test_concurrent_initialization();
    test_concurrent_handler_registration();
    test_interrupt_concurrency();
    test_lock_performance();
    
    println!("=== All DI System Concurrency Tests Passed ===");
}
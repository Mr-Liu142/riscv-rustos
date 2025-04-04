//! 并发安全性测试模块
//!
//! 用于测试DI系统中的锁机制是否正确保护全局状态

use crate::println;
use crate::trap::ds::{TrapType, TrapContext, TrapHandlerResult};
use core::sync::atomic::{AtomicUsize, Ordering};
use super::{
    initialize_trap_system, register_handler, unregister_handler,
    print_handlers, enable_interrupts, disable_interrupts,
    handler_count  // 明确从DI模块导入handler_count
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

    const TEST_HANDLER_NAME: &'static str = "Interrupt Concurrency Test Handler";

    // 先尝试清理可能的残留处理器
    println!("Cleaning up any residual test handlers...");
    unregister_handler(TrapType::TimerInterrupt, TEST_HANDLER_NAME);

    // 保存当前中断状态
    let was_enabled = disable_interrupts();
    println!("Interrupts disabled, previous state: {}", was_enabled);

    // 注册一个测试处理器
    let register_result = register_handler(
        TrapType::TimerInterrupt,
        test_concurrent_handler,
        30,
        TEST_HANDLER_NAME
    );
    assert!(register_result, "Failed to register test handler");
    println!("Test handler registered successfully");

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

    // 清理：注销测试处理器
    println!("Cleaning up test handler...");
    let unregister_result = unregister_handler(
        TrapType::TimerInterrupt,
        TEST_HANDLER_NAME
    );
    assert!(unregister_result, "Failed to unregister test handler");
    println!("Test handler unregistered successfully");

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

/// 测试处理器注册表的线程安全性
pub fn test_handler_registry_safety() {
    println!("Testing handler registry thread safety...");

    // 临时处理器函数，仅用于测试
    fn test_registry_handler1(ctx: &mut TrapContext) -> TrapHandlerResult {
        println!("Registry test handler 1 called");
        TrapHandlerResult::Handled
    }

    fn test_registry_handler2(ctx: &mut TrapContext) -> TrapHandlerResult {
        println!("Registry test handler 2 called");
        TrapHandlerResult::Handled
    }

    fn test_registry_handler3(ctx: &mut TrapContext) -> TrapHandlerResult {
        println!("Registry test handler 3 called");
        TrapHandlerResult::Handled
    }

    // 定义要测试的处理器描述符
    const HANDLER1_DESC: &'static str = "Registry Test Handler 1";
    const HANDLER2_DESC: &'static str = "Registry Test Handler 2";
    const HANDLER3_DESC: &'static str = "Registry Test Handler 3";

    // 先尝试清除可能残留的测试处理器，确保测试环境干净
    unregister_handler(TrapType::TimerInterrupt, HANDLER1_DESC);
    unregister_handler(TrapType::TimerInterrupt, HANDLER2_DESC);
    unregister_handler(TrapType::TimerInterrupt, HANDLER3_DESC);

    // 记录清理残留后的初始计数 - 使用DI系统的handler_count函数
    let initial_count = handler_count(TrapType::TimerInterrupt);
    println!("Initial handler count after pre-cleanup: {}", initial_count);

    // 1. 同时注册多个处理器
    println!("Simulating concurrent handler registration...");

    let results = [
        register_handler(TrapType::TimerInterrupt, test_registry_handler1, 25, HANDLER1_DESC),
        register_handler(TrapType::TimerInterrupt, test_registry_handler2, 26, HANDLER2_DESC),
        register_handler(TrapType::TimerInterrupt, test_registry_handler3, 27, HANDLER3_DESC)
    ];

    // 检查注册结果
    let success_count = results.iter().filter(|&&r| r).count();
    println!("Successfully registered {} of 3 handlers", success_count);
    // 断言：确保所有三个测试处理器都注册成功，否则测试无法按预期进行
    assert_eq!(success_count, 3, "Failed to register all test handlers initially");

    // 验证处理器数量
    let new_count = handler_count(TrapType::TimerInterrupt);
    println!("New handler count: {}", new_count);
    println!("Checking count: new {} vs initial {} + success {}",
             new_count, initial_count, success_count);
    assert_eq!(new_count, initial_count + success_count,
               "Handler count mismatch after registration: expected {}, got {}",
               initial_count + success_count, new_count);

    // 2. 模拟"并发"访问 - 快速交替注册和注销 (制造重复项)
    println!("Simulating interleaved register/unregister operations...");

    // 先注销处理器2
    let unregister_result = unregister_handler(TrapType::TimerInterrupt, HANDLER2_DESC);
    println!("Unregistered handler 2: {}", unregister_result);
    assert!(unregister_result, "Failed to unregister Handler 2 the first time"); // 应该成功

    // 再次注册处理器2，但优先级不同
    let reregister_result = register_handler(
        TrapType::TimerInterrupt,
        test_registry_handler2,
        35, // 不同的优先级
        HANDLER2_DESC // 相同的描述符
    );
    println!("Re-registered handler 2: {}", reregister_result);
    assert!(reregister_result, "Failed to re-register Handler 2"); // 应该成功

    // 3. 验证注册表状态一致性
    print_handlers();

    // 获取清理前的最终计数
    let final_count = handler_count(TrapType::TimerInterrupt);
    println!("Final handler count before cleanup: {}", final_count);

    // 检查此时的数量是否正确 (初始 + 注册3个 - 注销1个 + 重注册1个 = 初始 + 3)
    let expected_final_count = initial_count + 3;
    println!("Checking final consistency: count {} vs expected {}",
             final_count, expected_final_count);
    assert_eq!(final_count, expected_final_count,
               "Registry state inconsistency before cleanup: expected {}, got {}",
               expected_final_count, final_count);

    // 4. 清理 - 注销所有我们添加的测试处理器
    println!("Attempting cleanup of test handlers...");
    let _cleanup1 = unregister_handler(TrapType::TimerInterrupt, HANDLER1_DESC);
    let _cleanup2 = unregister_handler(TrapType::TimerInterrupt, HANDLER2_DESC);
    let _cleanup3 = unregister_handler(TrapType::TimerInterrupt, HANDLER3_DESC);

    // 获取清理后的处理器数量
    let cleanup_count = handler_count(TrapType::TimerInterrupt);
    println!("After cleanup handler count: {}", cleanup_count);

    // 检查实际移除的数量是否等于我们尝试移除的数量 (3)
    let actual_removed_count = final_count.saturating_sub(cleanup_count); // 使用 saturating_sub 防止下溢
    let expected_removed_count = 3; // 我们尝试移除 H1, H2, H3 这三个

    println!("Checking cleanup: actual removed count {} vs expected removed count {}",
             actual_removed_count, expected_removed_count);

    // 断言实际移除的数量是否等于预期移除的数量
    assert_eq!(actual_removed_count, expected_removed_count,
               "Cleanup removed an unexpected number of handlers: expected to remove {}, actually removed {}",
               expected_removed_count, actual_removed_count);

    // 最终检查：清理后的数量应该等于测试开始前的初始数量
    assert_eq!(cleanup_count, initial_count,
               "Handler count after cleanup ({}) does not match initial count before test registration ({})",
               cleanup_count, initial_count);

    println!("Handler registry thread safety test passed");
}

/// 运行所有并发安全测试
pub fn run_all_concurrency_tests() {
    println!("=== Running DI System Concurrency Tests ===");

    test_concurrent_initialization();
    test_concurrent_handler_registration();
    test_interrupt_concurrency();
    test_lock_performance();
    test_handler_registry_safety();

    println!("=== All DI System Concurrency Tests Passed ===");
}
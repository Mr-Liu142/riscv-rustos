//! Trap API 测试模块
//!
//! 测试 trap::api 模块的功能

use crate::trap::api;
use crate::trap::ds::{
    TrapType, TrapContext, TrapHandlerResult, Interrupt, 
    SystemError, ErrorResult, ErrorSource, ErrorLevel, TrapError
};
use crate::trap::ds::handler::RegistrarId;
use crate::println;

// 全局测试模块注册者ID
static mut TEST_REGISTRAR_ID: Option<RegistrarId> = None;

// 获取测试模块的注册者ID
fn get_test_registrar_id() -> RegistrarId {
    unsafe {
        if TEST_REGISTRAR_ID.is_none() {
            TEST_REGISTRAR_ID = Some(api::get_registrar_id());
        }
        TEST_REGISTRAR_ID.unwrap()
    }
}

// 测试用的中断处理函数
fn test_trap_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Test trap handler called");
    TrapHandlerResult::Handled
}

// 测试用的错误处理函数
fn test_error_handler(error: &SystemError) -> ErrorResult {
    println!("Test error handler called for error: {:?}", error.code());
    ErrorResult::Handled
}

// 测试处理器管理函数
fn test_handler_management() -> bool {
    println!("Testing trap handler management...");
    
    // 获取测试模块的注册者ID
    let registrar_id = get_test_registrar_id();
    println!("Test module registrar ID: {}", registrar_id);
    
    // 生成唯一上下文ID用于测试
    let context_id = api::generate_context_id();
    println!("Generated context ID: {}", context_id);
    
    // 测试注册处理器
    let register_result = api::register_trap_handler_secure(
        TrapType::SoftwareInterrupt,
        test_trap_handler,
        50,
        "Test Software Interrupt Handler",
        Some(context_id),
        registrar_id // 使用正确的注册者ID
    );
    
    if register_result.is_err() {
        println!("Failed to register first handler: {:?}", register_result.err().unwrap());
        return false;
    }
    
    println!("Successfully registered first test handler");
    
    // 尝试注册第二个处理器
    let register_result2 = api::register_trap_handler_secure(
        TrapType::SystemCall,
        test_trap_handler,
        50,
        "Test System Call Handler",
        Some(context_id),
        registrar_id // 使用正确的注册者ID
    );
    
    if register_result2.is_err() {
        println!("Failed to register second handler: {:?}", register_result2.err().unwrap());
        return false;
    }
    
    println!("Successfully registered second test handler");
    
    // 测试使用错误的注册者ID注销处理器
    let invalid_registrar_id = registrar_id + 1;
    let unregister_result = api::unregister_trap_handler_secure(
        TrapType::SoftwareInterrupt,
        "Test Software Interrupt Handler",
        invalid_registrar_id // 使用错误的注册者ID
    );
    
    if unregister_result.is_ok() {
        println!("Unexpectedly succeeded in unregistering with wrong registrar ID");
        return false;
    } else {
        println!("As expected, failed to unregister with wrong registrar ID: {:?}", 
                 unregister_result.err().unwrap());
    }
    
    // 测试注销单个处理器
    let unregister_result2 = api::unregister_trap_handler_secure(
        TrapType::SoftwareInterrupt,
        "Test Software Interrupt Handler",
        registrar_id // 使用正确的注册者ID
    );
    
    if unregister_result2.is_err() {
        println!("Failed to unregister handler: {:?}", unregister_result2.err().unwrap());
        return false;
    }
    
    println!("Successfully unregistered first test handler");
    
    // 测试注销指定上下文的所有处理器
    let unregister_count = api::unregister_trap_handlers_for_context_secure(
        context_id,
        registrar_id // 使用正确的注册者ID
    );
    
    println!("Unregistered {} handlers for context ID: {}", unregister_count, context_id);
    
    if unregister_count != 1 { // 应该注销1个处理器，因为之前已经手动注销了一个
        println!("Expected to unregister 1 handler, but got {}", unregister_count);
        return false;
    }
    
    // 测试尝试注销系统级处理器
    let system_handler_unregister = api::unregister_trap_handler_secure(
        TrapType::TimerInterrupt, // 系统默认处理器
        "Default Timer Handler", 
        registrar_id // 非系统注册者ID
    );
    
    if system_handler_unregister.is_ok() {
        println!("Unexpectedly succeeded in unregistering system handler");
        return false;
    } else {
        println!("As expected, failed to unregister system handler: {:?}", 
                 system_handler_unregister.err().unwrap());
    }
    
    println!("Trap handler security tests passed");
    true
}

// 测试中断控制函数
fn test_interrupt_control() -> bool {
    println!("Testing interrupt control...");
    
    // 测试全局中断控制
    let was_enabled = api::disable_interrupts();
    
    // 检查是否成功禁用
    if api::enable_interrupts() {
        println!("Interrupts were still enabled after disable_interrupts()");
        return false;
    }
    
    // 恢复中断状态
    api::restore_interrupts(was_enabled);
    
    // 测试特定中断类型控制
    api::disable_specific_interrupt(Interrupt::SupervisorTimer);
    
    // 验证中断被禁用
    if api::is_interrupt_enabled(Interrupt::SupervisorTimer) {
        println!("Timer interrupt still enabled after disabling");
        return false;
    }
    
    // 重新启用
    api::enable_specific_interrupt(Interrupt::SupervisorTimer);
    
    // 验证中断被启用
    if !api::is_interrupt_enabled(Interrupt::SupervisorTimer) {
        println!("Timer interrupt not enabled after enabling");
        return false;
    }
    
    println!("Interrupt control tests passed");
    true
}

// 测试状态查询函数
fn test_status_queries() -> bool {
    println!("Testing status query functions...");
    
    // 测试中断上下文检测（在正常代码中应该返回false）
    if api::is_in_trap_context() {
        println!("Incorrectly detected being in trap context");
        return false;
    }
    
    // 测试嵌套级别（在正常代码中应该为0）
    if api::current_trap_nest_level() != 0 {
        println!("Incorrect trap nesting level: expected 0, got {}", 
                 api::current_trap_nest_level());
        return false;
    }
    
    // 测试中断挂起状态
    let is_pending = api::is_interrupt_pending(Interrupt::SupervisorSoft);
    println!("Soft interrupt pending status: {}", is_pending);
    
    println!("Status query tests passed");
    true
}

// 测试上下文ID管理
fn test_context_id_management() -> bool {
    println!("Testing context ID management...");
    
    // 生成多个上下文ID并确保它们唯一
    let id1 = api::generate_context_id();
    let id2 = api::generate_context_id();
    let id3 = api::generate_context_id();
    
    println!("Generated context IDs: {}, {}, {}", id1, id2, id3);
    
    if id1 == id2 || id1 == id3 || id2 == id3 {
        println!("Generated duplicate context IDs: {}, {}, {}", id1, id2, id3);
        return false;
    }
    
    println!("Context ID management tests passed");
    true
}

// 测试错误处理系统
fn test_error_handling() -> bool {
    println!("Testing error handling system...");
    
    // 使用唯一描述符避免冲突
    let handler_desc = "Test Error Handler For API Test";
    
    // 注册错误处理器
    let register_result = api::register_error_handler(
        test_error_handler,
        50,
        handler_desc,
        Some(ErrorSource::Process),
        Some(ErrorLevel::Error)
    );
    
    if register_result.is_err() {
        println!("Failed to register error handler: {:?}", register_result.err().unwrap());
        return false;
    }
    
    println!("Successfully registered test error handler");
    
    // 创建并处理一个错误
    let error = api::create_system_error(
        ErrorSource::Process,
        ErrorLevel::Error,
        100,
        None,
        0x1000
    );
    
    let result = api::handle_system_error(error);
    
    if result != ErrorResult::Handled {
        println!("Error was not handled correctly: {:?}", result);
        return false;
    }
    
    println!("Error was correctly handled");
    
    // 测试错误日志功能
    api::print_error_log(5);
    api::clear_error_log();
    println!("Error log cleared");
    
    // 测试注销错误处理器
    let unregister_result = api::unregister_error_handler(handler_desc);
    
    if unregister_result.is_err() {
        println!("Failed to unregister error handler: {:?}", unregister_result.err().unwrap());
        return false;
    }
    
    println!("Successfully unregistered test error handler");
    
    // 测试恐慌模式（正常情况下应该为false）
    if api::is_panic_mode() {
        println!("System incorrectly in panic mode");
        return false;
    }
    
    println!("Panic mode correctly shows as not active");
    println!("Error handling system tests passed");
    true
}

// 运行所有测试
pub fn run_tests() -> bool {
    println!("=== Running Trap API tests ===");
    
    // 添加更详细的输出
    println!("Starting handler management tests...");
    let handler_test = test_handler_management();
    println!("Handler management tests completed with result: {}", handler_test);
    
    println!("Starting interrupt control tests...");
    let interrupt_test = test_interrupt_control();
    println!("Interrupt control tests completed with result: {}", interrupt_test);
    
    println!("Starting status query tests...");
    let status_test = test_status_queries();
    println!("Status query tests completed with result: {}", status_test);
    
    println!("Starting context ID management tests...");
    let context_test = test_context_id_management();
    println!("Context ID management tests completed with result: {}", context_test);
    
    println!("Starting error handling tests...");
    let error_test = test_error_handling();
    println!("Error handling tests completed with result: {}", error_test);
    
    let all_passed = handler_test && interrupt_test && status_test && 
                     context_test && error_test;
    
    println!("=== Trap API test results ===");
    println!("Handler management: {}", if handler_test { "PASSED" } else { "FAILED" });
    println!("Interrupt control: {}", if interrupt_test { "PASSED" } else { "FAILED" });
    println!("Status queries: {}", if status_test { "PASSED" } else { "FAILED" });
    println!("Context ID management: {}", if context_test { "PASSED" } else { "FAILED" });
    println!("Error handling: {}", if error_test { "PASSED" } else { "FAILED" });
    println!("Overall Trap API tests: {}", if all_passed { "PASSED" } else { "FAILED" });
    
    all_passed
}
//! 错误处理系统测试模块

use crate::println;
use crate::trap::ds::{
    SystemError, ErrorResult, ErrorSource, ErrorLevel, ErrorCode
};
use crate::trap::infrastructure::di;

/// 测试自定义错误处理器
fn test_error_handler(error: &SystemError) -> ErrorResult {
    println!("Test error handler called for: {}", error);
    ErrorResult::Handled
}

/// 测试警告处理器
fn test_warning_handler(error: &SystemError) -> ErrorResult {
    println!("Test warning handler: {}", error);
    ErrorResult::Handled
}

/// 测试错误处理注册
pub fn test_error_handler_registration() {
    println!("Testing error handler registration...");
    
    // 注册测试处理器
    let result1 = di::register_error_handler(
        test_error_handler,
        50, // 高优先级
        "Test Error Handler",
        Some(ErrorSource::Memory),
        Some(ErrorLevel::Error)
    );
    
    let result2 = di::register_error_handler(
        test_warning_handler,
        60,
        "Test Warning Handler",
        None,
        Some(ErrorLevel::Warning)
    );
    
    println!("Registration results: {}, {}", result1, result2);
    
    // 打印已注册的处理器
    di::print_error_handlers();
    
    // 注销一个处理器
    let unregister_result = di::unregister_error_handler("Test Error Handler");
    println!("Unregistration result: {}", unregister_result);
    
    // 再次打印处理器
    di::print_error_handlers();
}

/// 测试错误处理
pub fn test_error_handling() {
    println!("Testing error handling...");
    
    // 创建并处理不同类型的错误
    
    // 内存警告
    let mem_warning = di::create_system_error(
        ErrorSource::Memory,
        ErrorLevel::Warning,
        101,
        Some(0x80001000),
        0x80200500
    );
    
    let result1 = di::handle_system_error(mem_warning);
    println!("Memory warning handling result: {:?}", result1);
    
    // 进程错误
    let proc_error = di::create_system_error(
        ErrorSource::Process,
        ErrorLevel::Error,
        202,
        None,
        0x80200600
    );
    
    let result2 = di::handle_system_error(proc_error);
    println!("Process error handling result: {:?}", result2);
    
    // 系统调用错误
    let syscall_error = di::create_system_error(
        ErrorSource::Syscall,
        ErrorLevel::Error,
        303,
        Some(0x80002000),
        0x80200700
    );
    
    let result3 = di::handle_system_error(syscall_error);
    println!("Syscall error handling result: {:?}", result3);
    
    // 打印错误日志
    println!("\nError log:");
    di::print_error_log(10);
}

/// 测试恐慌模式 - 注意：此测试不触发实际的致命错误
pub fn test_panic_mode() {
    println!("Testing panic mode (simulation)...");
    
    // 检查初始状态
    let initial_panic = di::is_in_panic_mode();
    println!("Initial panic mode: {}", initial_panic);
    
    // 创建非致命错误
    let non_fatal = di::create_system_error(
        ErrorSource::Device,
        ErrorLevel::Error,
        404,
        None,
        0x80200800
    );
    
    di::handle_system_error(non_fatal);
    
    // 再次检查恐慌状态
    let mid_panic = di::is_in_panic_mode();
    println!("After non-fatal error, panic mode: {}", mid_panic);
    
    // 重置恐慌模式
    di::reset_panic_mode();
    let after_reset = di::is_in_panic_mode();
    println!("After reset, panic mode: {}", after_reset);
}

/// 运行所有错误处理系统测试
pub fn run_all_tests() {
    println!("\n=== Running Error Handling System Tests ===");
    
    // 确保系统已初始化（DI系统中已经完成）
    
    // 执行测试
    test_error_handler_registration();
    test_error_handling();
    test_panic_mode();
    
    // 清空日志
    di::clear_error_log();
    
    println!("=== Error Handling System Tests Completed ===\n");
}
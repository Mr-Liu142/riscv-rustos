//! 错误处理基础设施
//!
//! 提供全局错误处理器和默认错误处理实现。
//! 设计为不依赖堆内存分配器。

use crate::println;
use crate::trap::ds::{
    SystemError, ErrorResult, ErrorHandler, ErrorHandlerEntry,
    ErrorLog, ErrorSource, ErrorLevel, ErrorCode
};
use crate::trap::infrastructure::di;

/// 初始化标志
static mut INITIALIZED: bool = false;

/// 初始化错误处理系统
pub fn init() {
    unsafe {
        if INITIALIZED {
            println!("Error handling system already initialized");
            return;
        }
        
        // 注册默认处理器
        register_default_handlers();
        
        INITIALIZED = true;
    }
    
    println!("Error handling system initialized");
}

/// 注册默认错误处理器
fn register_default_handlers() {
    // 内存错误处理器
    register_handler(
        memory_error_handler,
        100,
        "Default Memory Error Handler",
        Some(ErrorSource::Memory),
        None
    );
    
    // 中断错误处理器
    register_handler(
        interrupt_error_handler,
        100,
        "Default Interrupt Error Handler",
        Some(ErrorSource::Interrupt),
        None
    );
    
    // 进程错误处理器
    register_handler(
        process_error_handler,
        100,
        "Default Process Error Handler",
        Some(ErrorSource::Process),
        None
    );
    
    // 系统调用错误处理器
    register_handler(
        syscall_error_handler,
        100,
        "Default Syscall Error Handler",
        Some(ErrorSource::Syscall),
        None
    );
    
    // 致命错误处理器
    register_handler(
        fatal_error_handler,
        10, // 高优先级
        "Fatal Error Handler",
        None,
        Some(ErrorLevel::Fatal)
    );
}


/// 注册自定义错误处理器
pub fn register_handler(
    handler: ErrorHandler,
    priority: u8,
    description: &'static str,
    source: Option<ErrorSource>,
    level: Option<ErrorLevel>
) -> bool {
    di::register_error_handler(handler, priority, description, source, level)
}

/// 注销错误处理器
pub fn unregister_handler(description: &str) -> bool {
    di::unregister_error_handler(description)
}

/// 处理系统错误
pub fn handle_error(error: SystemError) -> ErrorResult {
    di::handle_system_error(error)
}

/// 创建新的系统错误
pub fn create_error(
    source: ErrorSource,
    level: ErrorLevel,
    code: u16,
    address: Option<usize>,
    ip: usize
) -> SystemError {
    di::create_system_error(source, level, code, address, ip)
}

/// 打印错误日志
pub fn print_error_log(count: usize) {
    di::print_error_log(count)
}

/// 清空错误日志
pub fn clear_error_log() {
    di::clear_error_log()
}

/// 打印所有注册的错误处理器
pub fn print_handlers() {
    di::print_error_handlers()
}

/// 检查是否处于恐慌模式
pub fn is_panic_mode() -> bool {
    di::is_in_panic_mode()
}

/// 重置恐慌模式
pub fn reset_panic_mode() {
    di::reset_panic_mode()
}


// 默认错误处理器实现

/// 内存错误处理器
fn memory_error_handler(error: &SystemError) -> ErrorResult {
    println!("Memory error detected: {}", error);
    
    // 获取错误编码
    let code = error.code().code();
    
    match code {
        1 => {
            println!("Page fault - attempting to recover");
            // 这里可以添加页错误恢复逻辑
            ErrorResult::Partial
        },
        2 => {
            println!("Out of memory error");
            ErrorResult::Unhandled
        },
        3 => {
            println!("Invalid memory access at {:#x}", 
                    error.address().unwrap_or(0));
            ErrorResult::Handled
        },
        _ => {
            println!("Unknown memory error");
            ErrorResult::Unhandled
        }
    }
}

/// 中断错误处理器
fn interrupt_error_handler(error: &SystemError) -> ErrorResult {
    println!("Interrupt error detected: {}", error);
    
    // 简单处理
    ErrorResult::Handled
}

/// 进程错误处理器
fn process_error_handler(error: &SystemError) -> ErrorResult {
    println!("Process error detected: {}", error);
    
    // 简单处理
    ErrorResult::Handled
}

/// 系统调用错误处理器
fn syscall_error_handler(error: &SystemError) -> ErrorResult {
    println!("Syscall error detected: {}", error);
    
    // 简单处理
    ErrorResult::Handled
}

/// 致命错误处理器
fn fatal_error_handler(error: &SystemError) -> ErrorResult {
    println!("FATAL ERROR: {}", error);
    println!("System will be halted");
    
    // 输出最近错误日志
    print_error_log(5);
    
    // 可以尝试保存状态或执行紧急恢复措施
    ErrorResult::Partial // 返回Partial以允许其他处理器也处理
}
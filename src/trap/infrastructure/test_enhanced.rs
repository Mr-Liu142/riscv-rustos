//! 增强型异常处理器测试模块
//!
//! 提供一系列测试函数，用于验证增强型异常处理器的功能。

use crate::println;
use crate::trap::ds::{TrapContext, TrapType};
use super::enhanced_handlers;
use core::arch::asm;

/// 测试增强型异常处理器的注册
pub fn test_enhanced_handlers_registration() {
    println!("Testing enhanced exception handlers registration...");
    
    // 注册增强型异常处理器
    enhanced_handlers::register_enhanced_handlers();
    
    // 打印所有已注册的处理器
    super::di::print_handlers();
    
    println!("Enhanced handlers registration test completed");
}

/// 触发一个断点异常（用于测试断点处理器）
/// 
/// 注意：此函数会导致实际的断点异常，正常情况下会被增强型处理器捕获
pub fn trigger_breakpoint() {
    println!("Triggering breakpoint exception...");
    
    unsafe {
        // 使用RISC-V的ebreak指令触发断点异常
        asm!(
            "ebreak",
            "nop",
            options(nostack)
        );
    }
    
    println!("Returned from breakpoint handler");
}

/// 触发一个非法指令异常（用于测试非法指令处理器）
/// 
/// 注意：此函数会导致实际的非法指令异常，正常情况下会使系统停机
pub fn trigger_illegal_instruction() {
    println!("Triggering illegal instruction exception...");
    
    unsafe {
        // 使用一个无效的RISC-V指令触发异常
        // 这里使用0xFFFFFFFF，它不是有效的RISC-V指令
        asm!(".word 0xFFFFFFFF");
    }
    
    // 这一行应该不会被执行到，因为异常处理器会使系统停机
    println!("This line should not be reached");
}

/// 触发一个页错误（访问无效地址）
/// 
/// 注意：此函数会导致实际的页错误，正常情况下会使系统停机
pub fn trigger_page_fault() {
    println!("Triggering page fault...");
    
    unsafe {
        // 尝试访问一个可能不存在的内存地址
        let invalid_addr: *mut u32 = 0xFFFF_FFFF_FFFF_0000 as *mut u32;
        // 尝试读取该地址
        let _value = *invalid_addr;
    }
    
    // 这一行应该不会被执行到，因为异常处理器会使系统停机
    println!("This line should not be reached");
}

/// 运行所有增强型异常处理器测试
///
/// 注意：此函数会按序执行测试，某些测试会导致系统停机，
/// 因此在实际使用中应该有选择地单独执行每个测试。
pub fn run_all_tests() {
    println!("=== Starting Enhanced Exception Handlers Tests ===");
    
    // 测试注册
    test_enhanced_handlers_registration();
    
    // 测试断点异常（这个不会导致系统停机）
     trigger_breakpoint();
    
    // 以下测试会导致系统停机，取消注释以手动测试
    // trigger_page_fault();
    // trigger_illegal_instruction();
    
    println!("=== Enhanced Exception Handlers Tests Completed ===");
}
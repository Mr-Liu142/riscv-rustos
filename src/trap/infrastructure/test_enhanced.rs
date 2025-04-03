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

/// 触发一个地址未对齐异常（用于测试未对齐地址处理器）
/// 
/// 注意：此函数会导致实际的未对齐地址异常，正常情况下会使系统停机
/// 触发一个地址未对齐异常（用于测试未对齐地址处理器）
pub fn trigger_true_misaligned_access() {
    println!("Triggering true misaligned access exception...");
    
    // 创建一个对齐的缓冲区
    static mut ALIGNED_BUFFER: [u32; 4] = [0x12345678, 0x9ABCDEF0, 0x13579BDF, 0x2468ACE0];
    
    println!("RISC-V exception codes reference:");
    println!("  4: Load address misaligned");
    println!("  6: Store address misaligned");
    
    unsafe {
        // 获取对齐缓冲区的地址
        let base_addr = ALIGNED_BUFFER.as_ptr();
        println!("Buffer base address (aligned): {:p}", base_addr);
        
        // 创建一个未对齐的指针，指向缓冲区的有效区域
        // 故意将地址+1，使其未对齐但仍在有效区域内
        let misaligned_addr = (base_addr as *const u8).add(1) as *const u32;
        println!("Misaligned address to access: {:p}", misaligned_addr);
        println!("Address alignment: 2-byte={}, 4-byte={}, 8-byte={}", 
                 (misaligned_addr as usize & 0x1) == 0, 
                 (misaligned_addr as usize & 0x3) == 0, 
                 (misaligned_addr as usize & 0x7) == 0);
        
        // 尝试从未对齐地址读取，这应该触发代码4的未对齐异常
        println!("Reading u32 from misaligned address...");
        let _value = core::ptr::read_volatile(misaligned_addr);
        
        // 如果处理器自动处理了未对齐访问，这里会打印值
        println!("Value read: 0x{:08x} (Note: If you see this, your processor handles misaligned accesses automatically)", _value);
    }
    
    println!("This line should not be reached if misaligned exceptions are enabled");
}

pub fn trigger_load_access_fault() {
    println!("Triggering load access fault...");
    
    println!("RISC-V exception codes reference:");
    println!("  0: Instruction address misaligned");
    println!("  4: Load address misaligned");
    println!("  5: Load access fault");
    println!("  6: Store address misaligned");
    println!("  7: Store access fault");
    
    unsafe {
        // 尝试从一个很可能无效的地址加载数据
        // 这里故意使用未对齐地址来同时测试对齐和访问问题
        let bad_addr: *const u32 = 0x80001003 as *const u32;
        println!("Attempting to access address: {:#x}", bad_addr as usize);
        println!("Address alignment: 2-byte={}, 4-byte={}, 8-byte={}", 
                 (bad_addr as usize & 0x1) == 0, 
                 (bad_addr as usize & 0x3) == 0, 
                 (bad_addr as usize & 0x7) == 0);
        
        // 使用volatile读取，确保编译器不优化掉这个操作
        println!("Reading value from address...");
        let _value = core::ptr::read_volatile(bad_addr);
    }
    
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
    //trigger_load_access_fault();  // 更准确的名称
    //trigger_true_misaligned_access();  // 新增的测试函数
    
    println!("=== Enhanced Exception Handlers Tests Completed ===");
}
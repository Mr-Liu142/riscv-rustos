//! 中断基础设施模块

mod vector;
//mod context;  // 将在后续实现
//mod handler;  // 将在后续实现 
//mod csr;      // 将在后续实现
pub mod test; // 测试功能公开

use crate::println;

// 对外导出API
pub use vector::{init, TrapMode, enable_interrupts, disable_interrupts, restore_interrupts};

/// 初始化中断系统
pub fn init_trap_system() {
    // 初始化中断向量表，使用直接模式
    vector::init(TrapMode::Direct);
    
    println!("Trap infrastructure initialized");
}

/// 中断处理函数
#[no_mangle]
pub extern "C" fn handle_trap() {
    let cause = vector::get_trap_cause();
    
    // 使用正确的方式处理中断/异常
    if cause.is_interrupt() {
        // 处理中断
        match cause.code() {
            5 => {
                println!("Timer interrupt occurred");
                // 处理时钟中断
            },
            _ => {
                println!("Unhandled interrupt: {}", cause.code());
            }
        }
    } else {
        // 处理异常
        match cause.code() {
            8 => {
                println!("System call occurred");
                // 处理系统调用
            },
            _ => {
                println!("Unhandled exception: {}", cause.code());
            }
        }
    }
}
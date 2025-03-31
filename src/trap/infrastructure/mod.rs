//! 中断基础设施模块
//!
//! 提供中断系统基础功能和API

mod vector;
//mod context; // 将在后续实现
//mod handler; // 将在后续实现 
//mod csr;     // 将在后续实现
pub mod test; // 测试功能公开

use crate::println;

// 对外导出API
pub use vector::{
    init, 
    TrapMode, 
    Interrupt, 
    Exception,
    TrapContext,
    enable_interrupts, 
    disable_interrupts, 
    restore_interrupts,
    enable_interrupt,
    disable_interrupt,
    is_interrupt_enabled,
    is_interrupt_pending,
    set_soft_interrupt,
    clear_soft_interrupt,
};

/// 初始化中断系统
///
/// 这个函数完成基础中断系统的初始化工作
pub fn init_trap_system() {
    // 初始化中断向量表，使用直接模式
    vector::init(TrapMode::Direct);
    
    println!("Trap infrastructure initialized");
}

/// 中断处理函数
/// 
/// 参数为指向上下文结构的指针
#[no_mangle]
pub extern "C" fn handle_trap(context: *mut TrapContext) {
    let ctx = unsafe { &mut *context };
    let cause = ctx.get_cause();
    
    // 处理中断/异常
    if cause.is_interrupt() {
        match cause.code() {
            5 => {
                println!("Timer interrupt occurred");
                // 处理时钟中断
                // TODO: 具体的时钟中断处理逻辑
            },
            1 => {
                println!("Software interrupt occurred");
                // 处理软件中断
                clear_soft_interrupt();
            },
            9 => {
                println!("External interrupt occurred");
                // 处理外部中断
                // TODO: 处理外部中断，通常需要与PLIC交互
            },
            _ => {
                println!("Unhandled interrupt: {}", cause.code());
            }
        }
    } else {
        match cause.code() {
            8 => {
                println!("System call occurred");
                // 处理系统调用
                // TODO: 具体的系统调用处理逻辑
                
                // 系统调用返回时，PC需要加4跳过ecall指令
                ctx.set_return_addr(ctx.sepc + 4);
            },
            _ => {
                println!("Unhandled exception: {}, addr: {:#x}", cause.code(), ctx.stval);
                // 对于未处理的异常，可能需要终止当前进程
                // TODO: 进程终止或异常处理逻辑
            }
        }
    }
}
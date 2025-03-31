//! 上下文管理模块
//!
//! 本模块实现了处理器上下文的保存、恢复和切换功能，
//! 为操作系统的多任务和中断处理提供支持。

use core::fmt;
use core::arch::asm;
use crate::println;
use crate::trap::ds::{TaskContext, TrapContext};
use riscv::register::{sstatus, scause, stval, sepc};

/// 保存当前上下文到目标位置并切换到新上下文
/// 
/// # 参数
/// 
/// * `current_ctx` - 当前上下文的指针
/// * `next_ctx` - 下一个上下文的指针
///
/// # 安全性
///
/// 这个函数是不安全的，因为它：
/// 1. 直接操作栈和寄存器状态
/// 2. 假设两个上下文指针都有效
/// 3. 可能导致不一致状态，如果调用不当
pub unsafe extern "C" fn task_switch(
    _current_ctx: *mut TaskContext,
    _next_ctx: *const TaskContext,
) {
    // 这个函数无法用Rust代码实现，因为它需要直接控制
    // 寄存器保存和恢复。因此使用内联汇编实现。
    asm!(
        // 保存当前上下文的callee-saved寄存器
        "sd ra, 0(a0)",
        "sd sp, 8(a0)",
        "sd s0, 16(a0)",
        "sd s1, 24(a0)",
        "sd s2, 32(a0)",
        "sd s3, 40(a0)",
        "sd s4, 48(a0)",
        "sd s5, 56(a0)",
        "sd s6, 64(a0)",
        "sd s7, 72(a0)",
        "sd s8, 80(a0)",
        "sd s9, 88(a0)",
        "sd s10, 96(a0)",
        "sd s11, 104(a0)",
        
        // 恢复下一个上下文的callee-saved寄存器
        "ld ra, 0(a1)",
        "ld sp, 8(a1)",
        "ld s0, 16(a1)",
        "ld s1, 24(a1)",
        "ld s2, 32(a1)",
        "ld s3, 40(a1)",
        "ld s4, 48(a1)",
        "ld s5, 56(a1)",
        "ld s6, 64(a1)",
        "ld s7, 72(a1)",
        "ld s8, 80(a1)",
        "ld s9, 88(a1)",
        "ld s10, 96(a1)",
        "ld s11, 104(a1)",
        
        // 返回，实际上是跳转到新上下文的ra寄存器指向的地址
        "ret",
        
        options(noreturn)
    );
}

/// 在指定地址上创建一个新的任务上下文以准备启动
/// 
/// # 参数
/// 
/// * `entry` - 任务入口点函数
/// * `stack_top` - 任务栈顶
/// * `kstack_top` - 内核栈顶(用于特权级切换)
/// * `satp` - 页表基址寄存器值
/// 
/// # 返回值
/// 
/// 返回一个完整的任务上下文
pub fn prepare_task_context(
    entry: usize,
    stack_top: usize,
    kstack_top: usize,
    satp: usize,
) -> TrapContext {
    // 创建一个新的陷阱上下文
    let mut ctx = TrapContext::new();
    
    // 设置用户栈指针(sp)寄存器
    ctx.x[2] = stack_top;
    
    // 设置返回地址寄存器(ra)
    ctx.x[1] = entry;
    
    // 设置特权级寄存器
    // 设置SPP=0表示从U模式到S模式
    // 设置SPIE=1表示中断使能
    // 设置SUM=1允许S模式访问U模式页面
    let mut status = sstatus::read();
    status.set_spp(sstatus::SPP::User); // 用户模式
    status.set_spie(true); // 开启中断
    ctx.sstatus = status.bits();
    
    // 设置程序计数器为入口点
    ctx.sepc = entry;
    
    // 设置一个空的异常原因
    ctx.scause = 0;
    ctx.stval = 0;
    
    ctx
}

/// 将陷阱上下文从内核栈恢复到用户空间
/// 
/// 此函数不会返回，而是直接切换到用户空间
///
/// # 参数
///
/// * `ctx` - 陷阱上下文指针
///
/// # 安全性
///
/// 这个函数是不安全的，因为它会导致特权级切换
pub unsafe extern "C" fn trap_return() -> ! {
    // 恢复陷阱上下文并返回
    asm!(
        // 从栈上加载特权级寄存器
        "ld t0, 256(sp)",
        "ld t1, 264(sp)",
        "csrw sstatus, t0",
        "csrw sepc, t1",
        
        // 恢复通用寄存器
        "ld x1, 8(sp)",
        "ld x3, 24(sp)",
        "ld x4, 32(sp)",
        "ld x5, 40(sp)",
        "ld x6, 48(sp)",
        "ld x7, 56(sp)",
        "ld x8, 64(sp)",
        "ld x9, 72(sp)",
        "ld x10, 80(sp)",
        "ld x11, 88(sp)",
        "ld x12, 96(sp)",
        "ld x13, 104(sp)",
        "ld x14, 112(sp)",
        "ld x15, 120(sp)",
        "ld x16, 128(sp)",
        "ld x17, 136(sp)",
        "ld x18, 144(sp)",
        "ld x19, 152(sp)",
        "ld x20, 160(sp)",
        "ld x21, 168(sp)",
        "ld x22, 176(sp)",
        "ld x23, 184(sp)",
        "ld x24, 192(sp)",
        "ld x25, 200(sp)",
        "ld x26, 208(sp)",
        "ld x27, 216(sp)",
        "ld x28, 224(sp)",
        "ld x29, 232(sp)",
        "ld x30, 240(sp)",
        "ld x31, 248(sp)",
        
        // 最后恢复sp
        "ld x2, 16(sp)",
        "addi sp, sp, 288",  // 释放栈空间
        
        // 返回到用户空间
        "sret",
        
        options(noreturn)
    );
}

/// 保存完整上下文
/// 
/// 从当前寄存器状态创建一个完整的TrapContext
/// 
/// # 返回值
/// 
/// 返回填充了当前状态的TrapContext
pub fn save_full_context() -> TrapContext {
    let mut ctx = TrapContext::new();
    
    unsafe {
        // 读取通用寄存器
        asm!(
            "sd x1, 8({0})",
            "sd x2, 16({0})",
            "sd x3, 24({0})",
            "sd x4, 32({0})",
            "sd x5, 40({0})",
            "sd x6, 48({0})",
            "sd x7, 56({0})",
            "sd x8, 64({0})",
            "sd x9, 72({0})",
            "sd x10, 80({0})",
            "sd x11, 88({0})",
            "sd x12, 96({0})",
            "sd x13, 104({0})",
            "sd x14, 112({0})",
            "sd x15, 120({0})",
            "sd x16, 128({0})",
            "sd x17, 136({0})",
            "sd x18, 144({0})",
            "sd x19, 152({0})",
            "sd x20, 160({0})",
            "sd x21, 168({0})",
            "sd x22, 176({0})",
            "sd x23, 184({0})",
            "sd x24, 192({0})",
            "sd x25, 200({0})",
            "sd x26, 208({0})",
            "sd x27, 216({0})",
            "sd x28, 224({0})",
            "sd x29, 232({0})",
            "sd x30, 240({0})",
            "sd x31, 248({0})",
            in(reg) &mut ctx.x
        );
        
        // 读取特权级寄存器
        ctx.sstatus = sstatus::read().bits();
        ctx.sepc = sepc::read();
        ctx.scause = scause::read().bits();
        ctx.stval = stval::read();
    }
    
    ctx
}

/// 恢复完整上下文
/// 
/// 将提供的TrapContext恢复到处理器状态
/// 
/// # 参数
/// 
/// * `ctx` - 要恢复的上下文
/// 
/// # 安全性
/// 
/// 这个函数是不安全的，因为它直接改变处理器状态
pub unsafe fn restore_full_context(ctx: &TrapContext) {
    // 恢复特权级寄存器
    // 直接写入sepc寄存器
    sepc::write(ctx.sepc);
    
    // 使用内联汇编直接写入sstatus寄存器
    asm!(
        "csrw sstatus, {0}",
        in(reg) ctx.sstatus,
        options(nostack)
    );
    
    // 恢复通用寄存器
    asm!(
        "ld x1, 8({0})",
        "ld x3, 24({0})",
        "ld x4, 32({0})",
        "ld x5, 40({0})",
        "ld x6, 48({0})",
        "ld x7, 56({0})",
        "ld x8, 64({0})",
        "ld x9, 72({0})",
        "ld x10, 80({0})",
        "ld x11, 88({0})",
        "ld x12, 96({0})",
        "ld x13, 104({0})",
        "ld x14, 112({0})",
        "ld x15, 120({0})",
        "ld x16, 128({0})",
        "ld x17, 136({0})",
        "ld x18, 144({0})",
        "ld x19, 152({0})",
        "ld x20, 160({0})",
        "ld x21, 168({0})",
        "ld x22, 176({0})",
        "ld x23, 184({0})",
        "ld x24, 192({0})",
        "ld x25, 200({0})",
        "ld x26, 208({0})",
        "ld x27, 216({0})",
        "ld x28, 224({0})",
        "ld x29, 232({0})",
        "ld x30, 240({0})",
        "ld x31, 248({0})",
        "ld x2, 16({0})",  // 最后恢复sp
        in(reg) &ctx.x
    );
}

/// 创建一个用于测试的上下文
pub fn create_test_context(pc: usize, sp: usize) -> TrapContext {
    let mut ctx = TrapContext::new();
    ctx.sepc = pc;
    ctx.x[2] = sp; // sp
    
    // 设置特权级寄存器
    let mut status = sstatus::read();
    status.set_spp(sstatus::SPP::Supervisor); // 管理员模式
    status.set_spie(true); // 开启中断
    ctx.sstatus = status.bits();
    
    ctx
}

/// 上下文测试函数
pub fn test_context_switch() {
    println!("Testing context switching...");
    
    // 创建两个任务上下文
    let mut ctx1 = TaskContext::new();
    let mut ctx2 = TaskContext::new();
    
    // 在实际使用场景中，这些上下文会指向不同的任务入口点
    ctx1.set_ra(test_context_func1 as usize);
    ctx2.set_ra(test_context_func2 as usize);
    
    // 分配栈空间
    static mut STACK1: [u8; 4096] = [0; 4096];
    static mut STACK2: [u8; 4096] = [0; 4096];
    
    unsafe {
        ctx1.set_sp(STACK1.as_ptr().add(4096) as usize);
        ctx2.set_sp(STACK2.as_ptr().add(4096) as usize);
    }
    
    println!("Context 1: {:?}", ctx1);
    println!("Context 2: {:?}", ctx2);
    
    println!("Context switching test completed");
}

/// 测试函数1
fn test_context_func1() {
    println!("This is test function 1");
}

/// 测试函数2
fn test_context_func2() {
    println!("This is test function 2");
}
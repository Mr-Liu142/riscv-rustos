//! 中断系统测试模块

use crate::println;
use super::vector;
use super::context;
use super::TrapMode;
use super::TaskContext;

/// 测试中断向量初始化
pub fn test_vector_init() {
    println!("测试陷阱向量初始化...");
    vector::init(TrapMode::Direct);
    println!("陷阱向量初始化成功");
}

/// 测试中断开关功能
pub fn test_interrupt_control() {
    println!("测试中断控制...");
    
    // 保存当前中断状态
    let was_enabled = vector::disable_interrupts();
    println!("中断已禁用，之前状态: {}", was_enabled);
    
    // 启用中断
    vector::enable_interrupts();
    println!("中断已启用");
    
    // 再次禁用中断
    let new_state = vector::disable_interrupts();
    println!("中断再次禁用，状态为: {}", new_state);
    assert!(new_state, "中断应该已启用");
    
    // 恢复原始状态
    vector::restore_interrupts(was_enabled);
    println!("中断状态已恢复到原始状态");
}

/// 测试上下文管理功能
pub fn test_context_management() {
    println!("测试上下文管理...");
    
    // 创建两个任务上下文
    let mut ctx1 = TaskContext::new();
    let mut ctx2 = TaskContext::new();
    
    // 模拟两个不同的任务入口点函数
    extern "C" fn test_task1() {
        println!("任务1正在运行");
    }
    
    extern "C" fn test_task2() {
        println!("任务2正在运行");
    }
    
    // 设置入口点和栈
    static mut STACK1: [u8; 4096] = [0; 4096];
    static mut STACK2: [u8; 4096] = [0; 4096];
    
    unsafe {
        ctx1.set_ra(test_task1 as usize);
        ctx1.set_sp(STACK1.as_ptr().add(4096) as usize);
        
        ctx2.set_ra(test_task2 as usize);
        ctx2.set_sp(STACK2.as_ptr().add(4096) as usize);
    }
    
    println!("上下文1已准备: ra=0x{:x}, sp=0x{:x}", 
             ctx1.get_ra(), ctx1.get_sp());
    println!("上下文2已准备: ra=0x{:x}, sp=0x{:x}", 
             ctx2.get_ra(), ctx2.get_sp());
    
    // 由于实际的上下文切换会改变执行流，我们只测试结构是否正确
    println!("上下文管理测试完成");
}

/// 测试陷阱上下文创建和操作
pub fn test_trap_context() {
    println!("测试陷阱上下文功能...");
    
    // 创建一个测试用的陷阱上下文
    let test_pc = 0x80200000;
    let test_sp = 0x81000000;
    let ctx = context::create_test_context(test_pc, test_sp);
    
    println!("创建测试陷阱上下文 PC=0x{:x}, SP=0x{:x}", 
             ctx.sepc, ctx.x[2]);
    
    // 测试陷阱上下文基本操作
    let cause = ctx.get_cause();
    println!("陷阱原因: 是否中断={}, 代码={}", 
             cause.is_interrupt(), cause.code());
    
    println!("陷阱上下文测试完成");
}

/// 运行所有测试
pub fn run_all_tests() {
    println!("=== 开始陷阱基础设施测试 ===");
    test_vector_init();
    test_interrupt_control();
    test_context_management();
    test_trap_context();
    
    // 运行上下文切换测试
    context::test_context_switch();
    
    println!("=== 所有陷阱测试成功完成 ===");
}
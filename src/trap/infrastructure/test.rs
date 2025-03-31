//! 中断系统测试模块

use crate::println;
use super::vector;
use super::TrapMode;

/// 测试中断向量初始化
pub fn test_vector_init() {
    println!("Testing trap vector initialization...");
    vector::init(TrapMode::Direct);
    println!("Trap vector initialized successfully");
}

/// 测试中断开关功能
pub fn test_interrupt_control() {
    println!("Testing interrupt control...");
    
    // 保存当前中断状态
    let was_enabled = vector::disable_interrupts();
    println!("Interrupts disabled, previous state: {}", was_enabled);
    
    // 启用中断
    vector::enable_interrupts();
    println!("Interrupts enabled");
    
    // 再次禁用中断
    let new_state = vector::disable_interrupts();
    println!("Interrupts disabled again, state was: {}", new_state);
    assert!(new_state, "Interrupt should have been enabled");
    
    // 恢复原始状态
    vector::restore_interrupts(was_enabled);
    println!("Interrupt state restored to original");
}

/// 运行所有测试
pub fn run_all_tests() {
    println!("=== Starting trap infrastructure tests ===");
    test_vector_init();
    test_interrupt_control();
    println!("=== All trap tests completed successfully ===");
}
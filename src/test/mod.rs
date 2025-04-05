//! 内核测试模块
//!
//! 包含各种内核组件的单元测试。

use crate::println;

// 导出子模块
pub mod trap_api_test;

// 测试系统初始化函数
pub fn init_test_system() {
    // 初始化核心系统以便能够运行测试
    // 注意：这里假设trap系统已经在调用前初始化
    println!("Test system initialized");
}

// 测试运行器
pub fn run_all_tests() -> bool {
    println!("=== Running all kernel tests ===");
    
    // 运行各测试模块的测试
    let trap_api_success = trap_api_test::run_tests();
    
    // 汇总结果
    let all_success = trap_api_success;
    
    println!("=== Test summary ===");
    println!("Trap API tests: {}", if trap_api_success { "PASSED" } else { "FAILED" });
    println!("Overall result: {}", if all_success { "PASSED" } else { "FAILED" });
    
    all_success
}
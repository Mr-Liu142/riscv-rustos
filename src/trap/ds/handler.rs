//! 中断处理器类型定义
//!
//! 定义中断处理器函数的类型和相关数据结构

use super::context::TrapContext;
use super::types::TrapType;

/// 中断处理器保护级别
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProtectionLevel {
    /// 系统级处理器 - 只能由内核核心注册和注销
    System,
    /// 用户级处理器 - 可由各模块注册和注销，但必须验证所有权
    User,
}

/// 注册者ID类型
pub type RegistrarId = u64;

/// 系统注册者ID常量 - 使用特殊值表示内核核心
pub const SYSTEM_REGISTRAR_ID: RegistrarId = 0;

/// 生成新的注册者ID
pub fn generate_registrar_id() -> RegistrarId {
    use core::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1); // 从1开始，0保留给系统
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

/// 中断处理结果
#[derive(Debug, Clone, Copy)]
pub enum TrapHandlerResult {
    /// 已处理
    Handled,
    /// 需要传递给下一个处理器
    Pass,
    /// 中断处理失败
    Failed(TrapError),
}

/// 中断处理错误
#[derive(Debug, Clone, Copy)]
pub enum TrapError {
    /// 没有处理器可处理该中断
    NoHandler,
    /// 处理器执行失败
    HandlerFailed,
    /// 未知错误
    Unknown,
}

/// 中断处理器函数类型
pub type TrapHandler = fn(&mut TrapContext) -> TrapHandlerResult;

/// 中断处理器注册信息
#[derive(Copy, Clone)]
pub struct HandlerEntry {
    /// 处理器函数
    pub handler: TrapHandler,
    /// 处理器优先级，数字越小优先级越高
    pub priority: u8,
    /// 处理器描述，用于调试
    pub description: &'static str,
    /// 处理器保护级别
    pub protection_level: ProtectionLevel,
    /// 注册者ID
    pub registrar_id: RegistrarId,
}

impl HandlerEntry {
    /// 创建新的处理器入口 (兼容原有代码)
    pub const fn new(
        handler: TrapHandler, 
        priority: u8, 
        description: &'static str
    ) -> Self {
        Self {
            handler,
            priority,
            description,
            protection_level: ProtectionLevel::System, // 默认为系统级
            registrar_id: SYSTEM_REGISTRAR_ID,
        }
    }
    
    /// 创建新的处理器入口 (完整版)
    pub const fn new_with_protection(
        handler: TrapHandler, 
        priority: u8, 
        description: &'static str,
        protection_level: ProtectionLevel,
        registrar_id: RegistrarId
    ) -> Self {
        Self {
            handler,
            priority,
            description,
            protection_level,
            registrar_id,
        }
    }

    /// 检查是否为系统级处理器
    pub fn is_system(&self) -> bool {
        self.protection_level == ProtectionLevel::System
    }

    /// 验证注册者身份
    pub fn verify_registrar(&self, id: RegistrarId) -> bool {
        // 系统级处理器需要特殊系统ID
        if self.is_system() {
            return id == SYSTEM_REGISTRAR_ID;
        }
        
        // 用户级处理器验证注册者ID匹配
        self.registrar_id == id
    }
}
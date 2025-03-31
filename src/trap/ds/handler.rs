//! 中断处理器类型定义
//!
//! 定义中断处理器函数的类型和相关数据结构

use super::context::TrapContext;
use super::types::TrapType;

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
}

impl HandlerEntry {
    /// 创建新的处理器入口
    pub const fn new(handler: TrapHandler, priority: u8, description: &'static str) -> Self {
        Self {
            handler,
            priority,
            description,
        }
    }
}
//! 上下文类型定义模块
//!
//! 定义了中断处理器上下文关联所需的类型和常量

use core::sync::atomic::{AtomicUsize, Ordering};

/// 上下文ID类型，用于唯一标识一个系统上下文
pub type ContextId = usize;

/// 内核上下文ID，表示不属于特定上下文的处理器
pub const KERNEL_CONTEXT_ID: Option<ContextId> = None;

/// 生成全局唯一的上下文ID
pub fn generate_context_id() -> ContextId {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}
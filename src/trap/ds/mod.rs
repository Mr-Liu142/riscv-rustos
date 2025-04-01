//! 中断系统数据结构模块
//!
//! 本模块定义了中断系统所需的基本数据结构和类型，
//! 作为其他中断相关模块的共享基础。

pub mod context;
pub mod types;
pub mod handler;
pub mod context_manager;  // 新增上下文管理器模块
pub mod error;  // 添加错误处理数据结构模块

// 从子模块重新导出所有公共类型，方便使用
pub use context::{TrapContext, TaskContext};
pub use types::{TrapMode, Interrupt, Exception, TrapType, TrapCause};
pub use handler::{TrapHandler, TrapHandlerResult, TrapError, HandlerEntry};
pub use context_manager::{
    ContextManager, ContextError, ContextType, ContextState,
    InterruptContextGuard, is_in_interrupt_context, get_interrupt_nest_level,
    init_global_context_manager, get_context_manager,
};
pub use error::{  // 导出错误处理类型
    SystemError, ErrorResult, ErrorHandler, ErrorHandlerEntry,
    ErrorSource, ErrorLevel, ErrorCode, ErrorLog, ErrorManager
};
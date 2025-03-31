//! 中断系统数据结构模块
//!
//! 本模块定义了中断系统所需的基本数据结构和类型，
//! 作为其他中断相关模块的共享基础。

pub mod context;
pub mod types;
pub mod handler;

// 从子模块重新导出所有公共类型，方便使用
pub use context::{TrapContext, TaskContext};
pub use types::{TrapMode, Interrupt, Exception, TrapType, TrapCause};
pub use handler::{TrapHandler, TrapHandlerResult, TrapError, HandlerEntry};
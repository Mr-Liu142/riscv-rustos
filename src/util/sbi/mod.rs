//! SBI接口封装模块
//! 
//! 本模块封装了RISC-V SBI调用，提供了更友好的接口给操作系统使用。
//! SBI (Supervisor Binary Interface) 是RISC-V架构中M模式和S模式之间的标准接口。

mod api;

pub use api::*;
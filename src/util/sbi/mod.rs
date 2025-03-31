//! SBI接口封装模块
//! 
//! 本模块封装了RISC-V SBI调用，提供了更友好的接口给操作系统使用。
//! SBI (Supervisor Binary Interface) 是RISC-V架构中M模式和S模式之间的标准接口。

mod api;
mod ext;

// 导出基础API函数
pub use api::*;

// 导出扩展模块
pub use ext::system;
pub use ext::console;
pub use ext::timer;
pub use ext::hart;
pub use ext::tlb;
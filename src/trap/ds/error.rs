//! Error handling data structures module
//!
//! 定义操作系统错误处理所需的各种类型和数据结构。
//! 设计为不依赖堆内存分配器。

use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering, AtomicBool}; // 添加AtomicBool的导入


/// 错误级别枚举
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ErrorLevel {
    /// 致命错误 - 无法恢复，需要系统重启
    Fatal = 0,
    /// 严重错误 - 可能需要终止当前进程
    Critical = 1,
    /// 错误 - 通常可以被处理
    Error = 2,
    /// 警告 - 不会导致程序失败但需要注意
    Warning = 3,
    /// 信息 - 只是通知而非错误
    Info = 4,
}

/// 错误源枚举
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorSource {
    /// 未知错误源
    Unknown = 0,
    /// 中断处理相关
    Interrupt = 1,
    /// 内存管理相关
    Memory = 2,
    /// 进程管理相关
    Process = 3,
    /// 文件系统相关
    FileSystem = 4,
    /// 设备驱动相关
    Device = 5,
    /// 网络相关
    Network = 6,
    /// 系统调用相关
    Syscall = 7,
    /// 电源管理相关
    Power = 8,
    /// 同步原语相关
    Synchronization = 9,
    /// 调度器相关
    Scheduler = 10,
}

/// 记录错误码
/// 
/// 采用32位整数:
/// - 高8位: 错误源
/// - 中8位: 错误级别
/// - 低16位: 错误编号
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ErrorCode(u32);

impl ErrorCode {
    /// 创建新的错误码
    pub const fn new(source: ErrorSource, level: ErrorLevel, code: u16) -> Self {
        let value = ((source as u32) << 24) | ((level as u32) << 16) | (code as u32);
        Self(value)
    }
    
    /// 获取原始值
    pub fn value(&self) -> u32 {
        self.0
    }
    
    /// 获取错误源
    pub fn source(&self) -> ErrorSource {
        let src = (self.0 >> 24) as u8;
        // 安全地转换回枚举
        match src {
            1 => ErrorSource::Interrupt,
            2 => ErrorSource::Memory,
            3 => ErrorSource::Process,
            4 => ErrorSource::FileSystem,
            5 => ErrorSource::Device,
            6 => ErrorSource::Network,
            7 => ErrorSource::Syscall,
            8 => ErrorSource::Power,
            9 => ErrorSource::Synchronization,
            10 => ErrorSource::Scheduler,
            _ => ErrorSource::Unknown,
        }
    }
    
    /// 获取错误级别
    pub fn level(&self) -> ErrorLevel {
        let lvl = ((self.0 >> 16) & 0xFF) as u8;
        // 安全地转换回枚举
        match lvl {
            0 => ErrorLevel::Fatal,
            1 => ErrorLevel::Critical,
            2 => ErrorLevel::Error,
            3 => ErrorLevel::Warning,
            4 => ErrorLevel::Info,
            _ => ErrorLevel::Error, // 默认为一般错误
        }
    }
    
    /// 获取错误编号
    pub fn code(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
    
    /// 检查是否为致命错误
    pub fn is_fatal(&self) -> bool {
        self.level() == ErrorLevel::Fatal
    }
    
    /// 检查是否为警告
    pub fn is_warning(&self) -> bool {
        self.level() == ErrorLevel::Warning || self.level() == ErrorLevel::Info
    }
}

impl fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ErrorCode({:#010x}: {:?}/{:?}/{})", 
               self.0, self.source(), self.level(), self.code())
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}:{:?}:{}]", self.source(), self.level(), self.code())
    }
}

/// 系统错误类型
/// 
/// 包含错误码和附加信息
#[derive(Debug, Copy, Clone)]
pub struct SystemError {
    /// 错误码
    code: ErrorCode,
    /// 错误相关地址 (如果有)
    address: Option<usize>,
    /// 错误产生时的指令地址
    instruction_pointer: usize,
    /// 时间戳
    timestamp: u64,
}

impl SystemError {
    /// 创建新的系统错误
    pub fn new(code: ErrorCode, address: Option<usize>, instruction_pointer: usize, timestamp: u64) -> Self {
        Self {
            code,
            address,
            instruction_pointer,
            timestamp,
        }
    }
    
    /// 获取错误码
    pub fn code(&self) -> ErrorCode {
        self.code
    }
    
    /// 获取相关地址
    pub fn address(&self) -> Option<usize> {
        self.address
    }
    
    /// 获取指令指针
    pub fn instruction_pointer(&self) -> usize {
        self.instruction_pointer
    }
    
    /// 获取时间戳
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {} at IP={:#x}", self.code, self.instruction_pointer)?;
        if let Some(addr) = self.address {
            write!(f, ", address={:#x}", addr)?;
        }
        write!(f, ", time={}", self.timestamp)
    }
}

/// 错误处理结果枚举
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorResult {
    /// 错误已完全处理
    Handled,
    /// 错误部分处理，可继续
    Partial,
    /// 错误无法处理
    Unhandled,
    /// 错误处理被忽略
    Ignored,
}

/// 错误处理器函数类型
pub type ErrorHandler = fn(&SystemError) -> ErrorResult;

/// 错误处理器注册信息
#[derive(Copy, Clone)]
pub struct ErrorHandlerEntry {
    /// 处理器函数
    pub handler: ErrorHandler,
    /// 处理器优先级，数字越小优先级越高
    pub priority: u8,
    /// 处理器描述
    pub description: &'static str,
    /// 适用的错误源
    pub source: Option<ErrorSource>,
    /// 适用的错误级别
    pub level: Option<ErrorLevel>,
}

impl ErrorHandlerEntry {
    /// 创建新的错误处理器入口
    pub const fn new(
        handler: ErrorHandler, 
        priority: u8, 
        description: &'static str,
        source: Option<ErrorSource>,
        level: Option<ErrorLevel>,
    ) -> Self {
        Self {
            handler,
            priority,
            description,
            source,
            level,
        }
    }
    
    /// 检查处理器是否适用于指定错误
    pub fn matches(&self, error: &SystemError) -> bool {
        // 检查错误源是否匹配
        if let Some(src) = self.source {
            if error.code().source() != src {
                return false;
            }
        }
        
        // 检查错误级别是否匹配
        if let Some(lvl) = self.level {
            if error.code().level() != lvl {
                return false;
            }
        }
        
        true
    }
}

impl fmt::Debug for ErrorHandlerEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ErrorHandlerEntry")
            .field("description", &self.description)
            .field("priority", &self.priority)
            .field("source", &self.source)
            .field("level", &self.level)
            .finish()
    }
}

/// 错误记录项
#[derive(Copy, Clone)]
pub struct ErrorLogEntry {
    /// 错误信息
    pub error: SystemError,
    /// 是否已处理
    pub handled: bool,
    /// 处理结果
    pub result: ErrorResult,
}

/// 固定大小的错误日志
pub struct ErrorLog {
    /// 错误记录数组
    entries: [Option<ErrorLogEntry>; Self::MAX_ENTRIES],
    /// 当前索引
    current: usize,
    /// 记录总数
    count: AtomicUsize,
}

impl ErrorLog {
    /// 最大记录数
    pub const MAX_ENTRIES: usize = 32;
    
    /// 创建新的错误日志
    pub const fn new() -> Self {
        const NONE_ENTRY: Option<ErrorLogEntry> = None;
        Self {
            entries: [NONE_ENTRY; Self::MAX_ENTRIES],
            current: 0,
            count: AtomicUsize::new(0),
        }
    }
    
    /// 记录一个新错误
    pub fn log(&mut self, error: SystemError, handled: bool, result: ErrorResult) {
        // 创建记录
        let entry = ErrorLogEntry {
            error,
            handled,
            result,
        };
        
        // 更新索引，采用循环缓冲方式
        let index = self.current;
        self.current = (self.current + 1) % Self::MAX_ENTRIES;
        
        // 保存记录
        self.entries[index] = Some(entry);
        
        // 更新计数
        self.count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// 获取记录总数
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
    
    /// 获取指定索引的记录
    pub fn get(&self, index: usize) -> Option<ErrorLogEntry> {
        if index >= Self::MAX_ENTRIES {
            return None;
        }
        
        // 计算实际索引，考虑循环缓冲
        let count = self.count();
        if count <= Self::MAX_ENTRIES {
            // 未填满，直接使用索引
            if index < count {
                return self.entries[index];
            }
        } else {
            // 已填满，需要考虑当前位置
            let actual_index = (self.current + index) % Self::MAX_ENTRIES;
            return self.entries[actual_index];
        }
        
        None
    }
    
    /// 清空日志
    pub fn clear(&mut self) {
        for i in 0..Self::MAX_ENTRIES {
            self.entries[i] = None;
        }
        self.current = 0;
        self.count.store(0, Ordering::Relaxed);
    }
    
    /// 打印最近的n条记录
    pub fn print_recent(&self, n: usize) {
        let total = self.count();
        let to_print = if total < n { total } else { n };
        
        if to_print == 0 {
            crate::println!("No error records found.");
            return;
        }
        
        crate::println!("Recent {} error(s) of total {}:", to_print, total);
        
        // 打印最近的n条记录
        let start_idx = if total <= Self::MAX_ENTRIES {
            // 未填满，从0开始
            if to_print > total {
                0
            } else {
                total - to_print
            }
        } else {
            // 已填满，需要考虑循环
            let current = self.current;
            if to_print >= Self::MAX_ENTRIES {
                // 打印所有可见记录
                0
            } else {
                // 计算起始索引，确保打印最近的n条
                (current + Self::MAX_ENTRIES - to_print) % Self::MAX_ENTRIES
            }
        };
        
        for i in 0..to_print {
            let idx = (start_idx + i) % Self::MAX_ENTRIES;
            if let Some(entry) = self.entries[idx] {
                let status = if entry.handled { "Handled" } else { "Unhandled" };
                crate::println!("[{}] {}: {} - {:?}", 
                    total - to_print + i + 1,
                    entry.error,
                    status,
                    entry.result
                );
            }
        }
    }
}

/// 最大错误处理器数量
const MAX_ERROR_HANDLERS: usize = 16;

/// 错误处理管理器
pub struct ErrorManager {
    /// 注册的错误处理器
    handlers: [Option<ErrorHandlerEntry>; MAX_ERROR_HANDLERS],
    /// 处理器数量
    handler_count: usize,
    /// 错误日志
    log: ErrorLog,
    /// 恐慌模式标志
    panic_mode: AtomicBool,
}

impl ErrorManager {
    /// 创建新的错误处理管理器
    pub const fn new() -> Self {
        const NONE_HANDLER: Option<ErrorHandlerEntry> = None;
        Self {
            handlers: [NONE_HANDLER; MAX_ERROR_HANDLERS],
            handler_count: 0,
            log: ErrorLog::new(),
            panic_mode: AtomicBool::new(false),
        }
    }
    
    /// 注册错误处理器
    pub fn register_handler(&mut self, handler: ErrorHandlerEntry) -> bool {
        if self.handler_count >= MAX_ERROR_HANDLERS {
            // 处理器已满
            return false;
        }
        
        // 查找插入位置，按优先级排序
        let mut insert_idx = self.handler_count;
        for i in 0..self.handler_count {
            if let Some(h) = &self.handlers[i] {
                if h.priority > handler.priority {
                    insert_idx = i;
                    break;
                }
            }
        }
        
        // 移动元素
        if insert_idx < self.handler_count {
            for i in (insert_idx..self.handler_count).rev() {
                self.handlers[i + 1] = self.handlers[i];
            }
        }
        
        // 插入新处理器
        self.handlers[insert_idx] = Some(handler);
        self.handler_count += 1;
        
        crate::println!("Registered error handler: {} with priority {}", 
                        handler.description, handler.priority);
        true
    }
    
    /// 注销指定的错误处理器
    pub fn unregister_handler(&mut self, description: &str) -> bool {
        let mut found = false;
        let mut found_idx = 0;
        
        // 查找处理器
        for i in 0..self.handler_count {
            if let Some(h) = &self.handlers[i] {
                if h.description == description {
                    found = true;
                    found_idx = i;
                    break;
                }
            }
        }
        
        if !found {
            return false;
        }
        
        // 移除处理器
        for i in found_idx..self.handler_count-1 {
            self.handlers[i] = self.handlers[i + 1];
        }
        self.handlers[self.handler_count - 1] = None;
        self.handler_count -= 1;
        
        crate::println!("Unregistered error handler: {}", description);
        true
    }
    
    /// 处理错误
    pub fn handle_error(&mut self, error: SystemError) -> ErrorResult {
        // 如果在恐慌模式，直接返回
        if self.panic_mode.load(Ordering::Relaxed) {
            // 仍然记录，但不尝试处理
            self.log.log(error, false, ErrorResult::Ignored);
            return ErrorResult::Ignored;
        }
        
        // 如果是致命错误，进入恐慌模式
        if error.code().is_fatal() {
            self.panic_mode.store(true, Ordering::Relaxed);
            crate::println!("FATAL ERROR: {}", error);
        }
        
        // 尝试所有匹配的处理器
        let mut final_result = ErrorResult::Unhandled;
        let mut handled = false;
        
        for i in 0..self.handler_count {
            if let Some(h) = &self.handlers[i] {
                if h.matches(&error) {
                    match (h.handler)(&error) {
                        ErrorResult::Handled => {
                            // 已处理，可以停止
                            handled = true;
                            final_result = ErrorResult::Handled;
                            break;
                        }
                        ErrorResult::Partial => {
                            // 部分处理，继续尝试其他处理器
                            handled = true;
                            final_result = ErrorResult::Partial;
                        }
                        ErrorResult::Ignored => {
                            // 忽略，继续尝试
                            final_result = ErrorResult::Ignored;
                        }
                        ErrorResult::Unhandled => {
                            // 未处理，继续尝试
                        }
                    }
                }
            }
        }
        
        // 记录错误
        self.log.log(error, handled, final_result);
        
        // 如果是致命错误且未处理，必须终止系统
        if error.code().is_fatal() && !handled {
            // 输出最后信息
            crate::println!("FATAL ERROR UNHANDLED, SYSTEM HALTING");
            crate::println!("Error details: {}", error);
            
            // 调用SBI关机函数或进入无限循环
            #[cfg(feature = "sbi_shutdown")]
            crate::util::sbi::system::shutdown(crate::util::sbi::system::ShutdownReason::SystemFailure);
            
            // 如果没有SBI支持，进入死循环
            loop {
                core::hint::spin_loop();
            }
        }
        
        final_result
    }
    
    /// 检查是否处于恐慌模式
    pub fn is_panic_mode(&self) -> bool {
        self.panic_mode.load(Ordering::Relaxed)
    }
    
    /// 重置恐慌模式
    pub fn reset_panic_mode(&self) {
        self.panic_mode.store(false, Ordering::Relaxed);
    }
    
    /// 获取错误日志引用
    pub fn get_log(&self) -> &ErrorLog {
        &self.log
    }
    
    /// 获取错误日志可变引用
    pub fn get_log_mut(&mut self) -> &mut ErrorLog {
        &mut self.log
    }
    
    /// 打印所有注册的处理器
    pub fn print_handlers(&self) {
        crate::println!("=== Registered Error Handlers ({}) ===", self.handler_count);
        for i in 0..self.handler_count {
            if let Some(h) = &self.handlers[i] {
                // 不使用format!宏，直接打印
                crate::println!("{}. {} (Priority: {}, Source: {:?}, Level: {:?})",
                    i + 1, h.description, h.priority, 
                    h.source.unwrap_or(ErrorSource::Unknown), 
                    h.level.unwrap_or(ErrorLevel::Error));
            }
        }
        crate::println!("===================================");
    }
}
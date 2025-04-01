//! 错误处理管理器数据结构
//!
//! 提供错误处理器的注册、分发和管理功能。

use core::sync::atomic::{AtomicBool, Ordering};
use super::error::{
    SystemError, ErrorResult, ErrorHandler, ErrorHandlerEntry,
    ErrorLog, ErrorSource, ErrorLevel, ErrorCode
};

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
                let source_str = match h.source {
                    Some(src) => format!("{:?}", src),
                    None => "Any".into(),
                };
                
                let level_str = match h.level {
                    Some(lvl) => format!("{:?}", lvl),
                    None => "Any".into(),
                };
                
                crate::println!("{}. {} (Priority: {}, Source: {}, Level: {})",
                    i + 1, h.description, h.priority, source_str, level_str);
            }
        }
        crate::println!("===================================");
    }
}
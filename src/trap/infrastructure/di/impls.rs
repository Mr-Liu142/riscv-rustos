//! Trap System Component Implementations
//!
//! This module provides concrete implementations of the trap system interfaces.

use core::sync::atomic::{AtomicUsize, Ordering};
use crate::println;
use crate::trap::ds::{
    TrapContext, TaskContext, TrapType, TrapHandlerResult, TrapError,
    TrapMode, Interrupt, ContextError, ContextType, ContextState
};
use super::traits::{
    TrapHandlerInterface, ContextManagerInterface, 
    HardwareControlInterface, TrapSystemConfig, ErrorManagerInterface
};

/// Standard Trap Handler Implementation
#[derive(Debug, Copy, Clone)]
pub struct StandardTrapHandler {
    /// Function pointer to the handler implementation
    handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
    
    /// Handler priority (lower = higher priority)
    priority: u8,
    
    /// Description for debugging
    description: &'static str,
    
    /// Type of trap this handler manages
    trap_type: TrapType,
}

impl StandardTrapHandler {
    /// Create a new standard trap handler
    pub const fn new(
        handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
        trap_type: TrapType,
        priority: u8,
        description: &'static str
    ) -> Self {
        Self {
            handler_fn,
            priority,
            description,
            trap_type,
        }
    }
}

impl TrapHandlerInterface for StandardTrapHandler {
    fn handle_trap(&self, context: &mut TrapContext) -> TrapHandlerResult {
        (self.handler_fn)(context)
    }
    
    fn get_trap_type(&self) -> TrapType {
        self.trap_type
    }
    
    fn get_priority(&self) -> u8 {
        self.priority
    }
    
    fn get_description(&self) -> &'static str {
        self.description
    }
}

/// RISC-V Hardware Control Implementation
#[derive(Copy, Clone)]
pub struct RiscvHardwareControl;

impl RiscvHardwareControl {
    /// Create a new RISC-V hardware control
    pub const fn new() -> Self {
        Self {}
    }
}

impl HardwareControlInterface for RiscvHardwareControl {
    fn init_trap_vector(&self, mode: TrapMode) {
        // Implementation from the original vector.rs
        unsafe {
            // Declare the external assembly entry point
            extern "C" {
                fn __trap_entry();
            }
            
            // Prepare value: address needs to be 4-byte aligned, mode in the lowest 2 bits
            let addr = (__trap_entry as usize) & !0x3;
            let mode_val = mode as usize;
            let value = addr | mode_val;
            
            // Use inline assembly to directly write to stvec
            core::arch::asm!(
                "csrw stvec, {0}",
                in(reg) value,
                options(nostack)
            );
        }
        
        println!("Trap vector initialized with {:?} mode", mode);
    }
    
    fn enable_interrupts(&self) -> bool {
        let was_enabled = riscv::register::sstatus::read().sie();
        unsafe {
            riscv::register::sstatus::set_sie();
        }
        was_enabled
    }
    
    fn disable_interrupts(&self) -> bool {
        let was_enabled = riscv::register::sstatus::read().sie();
        unsafe {
            riscv::register::sstatus::clear_sie();
        }
        was_enabled
    }
    
    fn restore_interrupts(&self, was_enabled: bool) {
        if was_enabled {
            unsafe {
                riscv::register::sstatus::set_sie();
            }
        }
    }
    
    fn enable_interrupt(&self, interrupt: Interrupt) {
        unsafe {
            match interrupt {
                Interrupt::SupervisorSoft => riscv::register::sie::set_ssoft(),
                Interrupt::SupervisorTimer => riscv::register::sie::set_stimer(),
                Interrupt::SupervisorExternal => riscv::register::sie::set_sext(),
            }
        }
    }
    
    fn disable_interrupt(&self, interrupt: Interrupt) {
        unsafe {
            match interrupt {
                Interrupt::SupervisorSoft => riscv::register::sie::clear_ssoft(),
                Interrupt::SupervisorTimer => riscv::register::sie::clear_stimer(),
                Interrupt::SupervisorExternal => riscv::register::sie::clear_sext(),
            }
        }
    }
    
    fn is_interrupt_enabled(&self, interrupt: Interrupt) -> bool {
        match interrupt {
            Interrupt::SupervisorSoft => riscv::register::sie::read().ssoft(),
            Interrupt::SupervisorTimer => riscv::register::sie::read().stimer(),
            Interrupt::SupervisorExternal => riscv::register::sie::read().sext(),
        }
    }
    
    fn is_interrupt_pending(&self, interrupt: Interrupt) -> bool {
        match interrupt {
            Interrupt::SupervisorSoft => riscv::register::sip::read().ssoft(),
            Interrupt::SupervisorTimer => riscv::register::sip::read().stimer(),
            Interrupt::SupervisorExternal => riscv::register::sip::read().sext(),
        }
    }
    
    fn set_soft_interrupt(&self) {
        unsafe {
            riscv::register::sip::set_ssoft();
        }
    }
    
    fn clear_soft_interrupt(&self) {
        unsafe {
            riscv::register::sip::clear_ssoft();
        }
    }
}

/// Interrupt nesting counter, stored as atomic to be thread-safe
static INTERRUPT_NEST_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Standard Context Manager Implementation
/// 
/// Note: This can't derive Copy because it contains a large array,
/// but we use raw pointers to static instances, so we don't need Copy.
pub struct StandardContextManager {
    /// Interrupt stack
    interrupt_stack: [u8; Self::INTERRUPT_STACK_SIZE],
    
    /// Current interrupt stack top pointer
    interrupt_stack_top: usize,
    
    /// Maximum allowed interrupt nesting level
    max_nest_level: usize,
}

impl StandardContextManager {
    /// Interrupt stack size (16KB)
    pub const INTERRUPT_STACK_SIZE: usize = 16 * 1024;
    
    /// Default maximum nesting level
    pub const DEFAULT_MAX_NEST_LEVEL: usize = 8;
    
    /// Create a new standard context manager
    pub const fn new() -> Self {
        Self {
            interrupt_stack: [0; Self::INTERRUPT_STACK_SIZE],
            interrupt_stack_top: 0,
            max_nest_level: Self::DEFAULT_MAX_NEST_LEVEL,
        }
    }
    
    /// Internal function to increase interrupt nesting level
    fn enter_interrupt(&mut self) -> Result<usize, ContextError> {
        let current = INTERRUPT_NEST_COUNT.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_nest_level {
            // Roll back counter
            INTERRUPT_NEST_COUNT.fetch_sub(1, Ordering::SeqCst);
            return Err(ContextError::StackOverflow);
        }
        Ok(current + 1)
    }
    
    /// Internal function to decrease interrupt nesting level
    fn exit_interrupt(&mut self) -> Result<usize, ContextError> {
        let current = INTERRUPT_NEST_COUNT.load(Ordering::Relaxed);
        if current == 0 {
            return Err(ContextError::StackUnderflow);
        }
        
        Ok(INTERRUPT_NEST_COUNT.fetch_sub(1, Ordering::SeqCst) - 1)
    }
}

impl ContextManagerInterface for StandardContextManager {
    fn save_context_for_interrupt(&mut self) -> Result<(*mut TrapContext, usize), ContextError> {
        // Increase nesting level
        let level = self.enter_interrupt()?;
        
        // Calculate stack position
        let stack_offset = level * core::mem::size_of::<TrapContext>();
        if stack_offset + core::mem::size_of::<TrapContext>() > Self::INTERRUPT_STACK_SIZE {
            self.exit_interrupt().ok(); // Decrease nesting level
            return Err(ContextError::StackOverflow);
        }
        
        // Use correct position on the interrupt stack to save context
        let ctx_ptr = unsafe {
            self.interrupt_stack.as_mut_ptr().add(stack_offset) as *mut TrapContext
        };
        
        // Create new context
        unsafe {
            *ctx_ptr = TrapContext::new();
        }
        
        // Return context pointer and nesting level
        Ok((ctx_ptr, level))
    }
    
    fn restore_context_from_interrupt(&mut self, ctx: &TrapContext) -> Result<(), ContextError> {
        // Decrease nesting level
        self.exit_interrupt()?;
        
        // Call low-level restore function
        unsafe {
            crate::trap::infrastructure::restore_full_context(ctx);
        }
        
        Ok(())
    }
    
    fn save_full_context(&mut self) -> TrapContext {
        crate::trap::infrastructure::save_full_context()
    }
    
    fn switch_task_context(&mut self, current: &mut TaskContext, next: &TaskContext) {
        // Use low-level task switch function
        unsafe {
            crate::trap::infrastructure::task_switch(current, next);
        }
    }
    
    fn create_task_context(&self, entry: usize, user_stack: usize, kernel_stack: usize, privilege_level: u8) -> TrapContext {
        let satp = 0; // Page table base address, could be passed from outside
        
        // Call the infrastructure context creation function
        crate::trap::infrastructure::prepare_task_context(
            entry, user_stack, kernel_stack, satp
        )
    }
    
    fn get_context_size(&self, context_type: ContextType) -> usize {
        match context_type {
            ContextType::Task => core::mem::size_of::<TaskContext>(),
            ContextType::Trap => core::mem::size_of::<TrapContext>(),
        }
    }
    
    fn get_interrupt_stack_usage(&self) -> (usize, usize) {
        let used = self.get_nest_level() * core::mem::size_of::<TrapContext>();
        (used, Self::INTERRUPT_STACK_SIZE)
    }
    
    fn is_in_interrupt_context(&self) -> bool {
        self.get_nest_level() > 0
    }
    
    fn get_nest_level(&self) -> usize {
        INTERRUPT_NEST_COUNT.load(Ordering::Relaxed)
    }
    
    fn set_max_nest_level(&mut self, level: usize) {
        self.max_nest_level = level;
    }
}

use crate::trap::ds::{
    SystemError, ErrorResult, ErrorHandler, ErrorHandlerEntry,
    ErrorSource, ErrorLevel, ErrorCode, ErrorManager
};
use crate::util::sbi::timer;

/// 标准错误管理器实现
pub struct StandardErrorManager {
    /// 内部错误管理器
    manager: ErrorManager,
}

impl StandardErrorManager {
    /// 创建新的标准错误管理器
    pub const fn new() -> Self {
        Self {
            manager: ErrorManager::new(),
        }
    }
    
    /// 紧急错误处理 - 在错误管理器未完全初始化时使用
    fn emergency_error_handler(&self, error: &SystemError) -> ErrorResult {
        println!("EMERGENCY ERROR HANDLER: {}", error);
        
        if error.code().is_fatal() {
            println!("FATAL ERROR in emergency mode, halting system");
            // 无限循环
            loop {
                core::hint::spin_loop();
            }
        }
        
        ErrorResult::Partial
    }
}

impl ErrorManagerInterface for StandardErrorManager {
    fn register_handler(
        &mut self,
        handler: ErrorHandler,
        priority: u8,
        description: &'static str,
        source: Option<ErrorSource>,
        level: Option<ErrorLevel>
    ) -> bool {
        let entry = ErrorHandlerEntry::new(handler, priority, description, source, level);
        self.manager.register_handler(entry)
    }
    
    fn unregister_handler(&mut self, description: &str) -> bool {
        self.manager.unregister_handler(description)
    }
    
    fn handle_error(&mut self, error: SystemError) -> ErrorResult {
        self.manager.handle_error(error)
    }
    
    fn print_error_log(&self, count: usize) {
        self.manager.get_log().print_recent(count)
    }
    
    fn clear_error_log(&mut self) {
        self.manager.get_log_mut().clear();
        println!("Error log cleared");
    }
    
    fn print_handlers(&self) {
        self.manager.print_handlers()
    }
    
    fn is_panic_mode(&self) -> bool {
        self.manager.is_panic_mode()
    }
    
    fn reset_panic_mode(&self) {
        self.manager.reset_panic_mode()
    }
    
    fn create_error(
        &self,
        source: ErrorSource,
        level: ErrorLevel,
        code: u16,
        address: Option<usize>,
        ip: usize
    ) -> SystemError {
        let error_code = ErrorCode::new(source, level, code);
        SystemError::new(error_code, address, ip, timer::get_time())
    }
}
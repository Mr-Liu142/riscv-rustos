//! Trap System Dependency Injection Traits
//!
//! This module defines the core traits for dependency injection in the trap system.
//! These traits provide a modular interface for different components of the trap system.

use crate::trap::ds::{
    TrapContext, TaskContext, TrapType, TrapHandlerResult, 
    ContextError, ContextType, ContextState
};

/// Trait for trap handler implementations
pub trait TrapHandlerInterface: Send + Sync {
    /// Handle a trap event
    fn handle_trap(&self, context: &mut TrapContext) -> TrapHandlerResult;
    
    /// Get the trap type this handler manages
    fn get_trap_type(&self) -> TrapType;
    
    /// Get handler priority (lower values = higher priority)
    fn get_priority(&self) -> u8;
    
    /// Get handler description
    fn get_description(&self) -> &'static str;
}

/// Trait for context management implementations
pub trait ContextManagerInterface: Send + Sync {
    /// Save current context for interrupt handling
    fn save_context_for_interrupt(&mut self) -> Result<(*mut TrapContext, usize), ContextError>;
    
    /// Restore context after interrupt handling
    fn restore_context_from_interrupt(&mut self, ctx: &TrapContext) -> Result<(), ContextError>;
    
    /// Save full processor context
    fn save_full_context(&mut self) -> TrapContext;
    
    /// Switch between task contexts
    fn switch_task_context(&mut self, current: &mut TaskContext, next: &TaskContext);
    
    /// Create a new task context
    fn create_task_context(&self, entry: usize, user_stack: usize, kernel_stack: usize, privilege_level: u8) -> TrapContext;
    
    /// Get the size of a context structure
    fn get_context_size(&self, context_type: ContextType) -> usize;
    
    /// Get current interrupt stack usage
    fn get_interrupt_stack_usage(&self) -> (usize, usize);
    
    /// Check if currently in interrupt context
    fn is_in_interrupt_context(&self) -> bool;
    
    /// Get current interrupt nesting level
    fn get_nest_level(&self) -> usize;
    
    /// Set maximum allowed interrupt nesting level
    fn set_max_nest_level(&mut self, level: usize);
}

/// Trait for hardware control implementations
pub trait HardwareControlInterface: Send + Sync {
    /// Initialize trap vector with specified mode
    fn init_trap_vector(&self, mode: crate::trap::ds::TrapMode);
    
    /// Enable all interrupts
    fn enable_interrupts(&self) -> bool;
    
    /// Disable all interrupts
    fn disable_interrupts(&self) -> bool;
    
    /// Restore previous interrupt state
    fn restore_interrupts(&self, was_enabled: bool);
    
    /// Enable specific interrupt
    fn enable_interrupt(&self, interrupt: crate::trap::ds::Interrupt);
    
    /// Disable specific interrupt
    fn disable_interrupt(&self, interrupt: crate::trap::ds::Interrupt);
    
    /// Check if specific interrupt is enabled
    fn is_interrupt_enabled(&self, interrupt: crate::trap::ds::Interrupt) -> bool;
    
    /// Check if specific interrupt is pending
    fn is_interrupt_pending(&self, interrupt: crate::trap::ds::Interrupt) -> bool;
    
    /// Set software interrupt
    fn set_soft_interrupt(&self);
    
    /// Clear software interrupt
    fn clear_soft_interrupt(&self);
}

/// Trait for trap system configuration
pub trait TrapSystemConfig: Send + Sync {
    /// Get maximum number of handlers per trap type
    fn max_handlers_per_type(&self) -> usize;
    
    /// Get maximum interrupt nesting level
    fn max_interrupt_nesting_level(&self) -> usize;
    
    /// Get interrupt stack size
    fn interrupt_stack_size(&self) -> usize;
}

/// Default implementation of TrapSystemConfig
pub struct DefaultTrapSystemConfig;

impl TrapSystemConfig for DefaultTrapSystemConfig {
    fn max_handlers_per_type(&self) -> usize {
        8 // Same as the original implementation
    }
    
    fn max_interrupt_nesting_level(&self) -> usize {
        8 // Same as the default in ContextManager
    }
    
    fn interrupt_stack_size(&self) -> usize {
        16 * 1024 // 16KB, same as the original implementation
    }
}
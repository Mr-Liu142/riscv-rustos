//! Trap System Public API
//!
//! This module provides a stable, clear, and unified public interface for
//! interacting with the trap system. It serves as the only entry point for other
//! kernel modules (such as Memory Management, Scheduler, etc.) to interact with
//! the trap system.
//!
//! The API hides the internal complexity of the trap system, particularly the
//! infrastructure and dependency injection components, providing a simplified
//! interface that reduces coupling and enhances maintainability.

use crate::trap::ds::{
    TrapType, TrapContext, TrapHandler, TrapHandlerResult, Interrupt, 
    SystemError, ErrorResult, ErrorSource, ErrorLevel, ErrorCode,
};
use crate::trap::infrastructure::di::context::ContextId;
use crate::println;

/// Errors that can occur when interacting with the trap API
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapApiError {
    /// The trap system has not been initialized
    SystemNotInitialized,
    /// Failed to register a handler
    RegistrationFailed,
    /// The specified handler could not be found
    HandlerNotFound,
    /// Too many handlers have been registered
    TooManyHandlers,
    /// The storage for handlers is locked
    StorageLocked,
    /// The given context ID is invalid
    InvalidContextId,
    /// The operation is not allowed in the current state
    OperationNotAllowed,
    /// The underlying trap system encountered an error
    InternalError,
}

impl core::fmt::Display for TrapApiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SystemNotInitialized => write!(f, "Trap system not initialized"),
            Self::RegistrationFailed => write!(f, "Failed to register handler"),
            Self::HandlerNotFound => write!(f, "Handler not found"),
            Self::TooManyHandlers => write!(f, "Too many handlers registered"),
            Self::StorageLocked => write!(f, "Handler storage is locked"),
            Self::InvalidContextId => write!(f, "Invalid context ID"),
            Self::OperationNotAllowed => write!(f, "Operation not allowed in current state"),
            Self::InternalError => write!(f, "Internal trap system error"),
        }
    }
}

//
// Trap Handler Management Functions
//

/// Register a trap handler for a specific trap type
///
/// # Parameters
///
/// * `trap_type` - The type of trap to handle
/// * `handler` - The handler function
/// * `priority` - Priority level (lower values mean higher priority)
/// * `description` - A static description of the handler (for debugging)
/// * `context_id` - Optional context ID to associate the handler with
///
/// # Returns
///
/// * `Ok(())` if registration was successful
/// * `Err(TrapApiError)` if registration failed
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads or in interrupt context.
pub fn register_trap_handler(
    trap_type: TrapType,
    handler: TrapHandler,
    priority: u8,
    description: &'static str,
    context_id: Option<ContextId>
) -> Result<(), TrapApiError> {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return Err(TrapApiError::SystemNotInitialized);
    }

    // Call the internal function to register the handler
    let result = crate::trap::infrastructure::di::register_handler(
        trap_type,
        handler,
        priority,
        description,
        context_id
    );

    if result {
        Ok(())
    } else {
        Err(TrapApiError::RegistrationFailed)
    }
}

/// Unregister a trap handler
///
/// # Parameters
///
/// * `trap_type` - The type of trap the handler was registered for
/// * `description` - The description used when registering the handler
///
/// # Returns
///
/// * `Ok(())` if unregistration was successful
/// * `Err(TrapApiError)` if the handler was not found or could not be unregistered
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads or in interrupt context.
pub fn unregister_trap_handler(
    trap_type: TrapType,
    description: &'static str
) -> Result<(), TrapApiError> {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return Err(TrapApiError::SystemNotInitialized);
    }

    // Call the internal function to unregister the handler
    let result = crate::trap::infrastructure::di::unregister_handler(trap_type, description);

    if result {
        Ok(())
    } else {
        Err(TrapApiError::HandlerNotFound)
    }
}

/// Unregister all trap handlers associated with a specific context ID
///
/// # Parameters
///
/// * `context_id` - The context ID to unregister handlers for
///
/// # Returns
///
/// The number of handlers that were unregistered
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads or in interrupt context.
pub fn unregister_trap_handlers_for_context(context_id: ContextId) -> usize {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return 0;
    }

    // Call the internal function to unregister handlers
    crate::trap::infrastructure::di::unregister_handlers_for_context(context_id)
}

//
// Interrupt Control Functions
//

/// Enable all interrupts
///
/// # Returns
///
/// * `true` if interrupts were previously enabled
/// * `false` if interrupts were previously disabled
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads, but can have system-wide
/// effects since it enables interrupts globally.
pub fn enable_interrupts() -> bool {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Return false as a safe default
        return false;
    }

    // Call the internal function to enable interrupts
    crate::trap::infrastructure::di::enable_interrupts()
}

/// Disable all interrupts
///
/// # Returns
///
/// * `true` if interrupts were previously enabled
/// * `false` if interrupts were previously disabled
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads, but can have system-wide
/// effects since it disables interrupts globally.
pub fn disable_interrupts() -> bool {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Return false as a safe default
        return false;
    }

    // Call the internal function to disable interrupts
    crate::trap::infrastructure::di::disable_interrupts()
}

/// Restore interrupts to their previous state
///
/// # Parameters
///
/// * `was_enabled` - The previous interrupt state (returned by disable_interrupts)
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads, but can have system-wide
/// effects since it may enable interrupts globally.
pub fn restore_interrupts(was_enabled: bool) {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return;
    }

    // Call the internal function to restore interrupts
    crate::trap::infrastructure::di::restore_interrupts(was_enabled)
}

/// Enable a specific type of interrupt
///
/// # Parameters
///
/// * `interrupt` - The specific interrupt type to enable
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads, but affects
/// system-wide interrupt handling.
pub fn enable_specific_interrupt(interrupt: Interrupt) {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return;
    }

    // Call the internal function to enable the specific interrupt
    crate::trap::infrastructure::di::enable_interrupt(interrupt)
}

/// Disable a specific type of interrupt
///
/// # Parameters
///
/// * `interrupt` - The specific interrupt type to disable
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads, but affects
/// system-wide interrupt handling.
pub fn disable_specific_interrupt(interrupt: Interrupt) {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return;
    }

    // Call the internal function to disable the specific interrupt
    crate::trap::infrastructure::di::disable_interrupt(interrupt)
}

//
// Status Query Functions
//

/// Check if the current execution context is an interrupt/trap context
///
/// # Returns
///
/// * `true` if currently executing in an interrupt/trap context
/// * `false` if executing in normal (thread) context
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn is_in_trap_context() -> bool {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Conservatively assume not in trap context
        return false;
    }

    // Call the internal function to check the context
    crate::trap::infrastructure::di::is_in_interrupt_context()
}

/// Get the current interrupt nesting level
///
/// # Returns
///
/// The number of nested interrupts currently active (0 = not in interrupt context)
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn current_trap_nest_level() -> usize {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Return 0 if not initialized
        return 0;
    }

    // Call the internal function to get the nesting level
    crate::trap::infrastructure::di::get_interrupt_nest_level()
}

/// Check if a specific interrupt is enabled
///
/// # Parameters
///
/// * `interrupt` - The specific interrupt type to check
///
/// # Returns
///
/// * `true` if the specified interrupt is enabled
/// * `false` if the specified interrupt is disabled
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn is_interrupt_enabled(interrupt: Interrupt) -> bool {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Conservatively assume disabled
        return false;
    }

    // Call the internal function to check if the interrupt is enabled
    crate::trap::infrastructure::di::is_interrupt_enabled(interrupt)
}

/// Check if a specific interrupt is pending
///
/// # Parameters
///
/// * `interrupt` - The specific interrupt type to check
///
/// # Returns
///
/// * `true` if the specified interrupt is pending
/// * `false` if the specified interrupt is not pending
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn is_interrupt_pending(interrupt: Interrupt) -> bool {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Conservatively assume not pending
        return false;
    }

    // Call the internal function to check if the interrupt is pending
    crate::trap::infrastructure::di::is_interrupt_pending(interrupt)
}

//
// Context ID Management
//

/// Generate a new unique context ID
///
/// Context IDs are used to associate trap handlers with specific contexts,
/// such as processes or threads.
///
/// # Returns
///
/// A new unique context ID
///
/// # Thread Safety
///
/// This function is safe to call from any context, including multiple threads
/// and interrupt handlers.
pub fn generate_context_id() -> ContextId {
    // Call the internal function to generate a context ID
    crate::trap::infrastructure::di::context::generate_context_id()
}

//
// Error Handling System
//

/// Register an error handler
///
/// # Parameters
///
/// * `handler` - The error handler function
/// * `priority` - The priority of the handler (lower values mean higher priority)
/// * `description` - A static description of the handler
/// * `source` - Optional error source to handle (if None, handles all sources)
/// * `level` - Optional error level to handle (if None, handles all levels)
///
/// # Returns
///
/// * `Ok(())` if registration was successful
/// * `Err(TrapApiError)` if registration failed
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads.
pub fn register_error_handler(
    handler: crate::trap::ds::ErrorHandler,
    priority: u8,
    description: &'static str,
    source: Option<ErrorSource>,
    level: Option<ErrorLevel>
) -> Result<(), TrapApiError> {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return Err(TrapApiError::SystemNotInitialized);
    }

    // Call the internal function to register the error handler
    let result = crate::trap::infrastructure::di::register_error_handler(
        handler, priority, description, source, level
    );

    if result {
        Ok(())
    } else {
        Err(TrapApiError::RegistrationFailed)
    }
}

/// Unregister an error handler
///
/// # Parameters
///
/// * `description` - The description used when registering the handler
///
/// # Returns
///
/// * `Ok(())` if unregistration was successful
/// * `Err(TrapApiError)` if the handler was not found
///
/// # Thread Safety
///
/// This function is safe to call from multiple threads.
pub fn unregister_error_handler(description: &str) -> Result<(), TrapApiError> {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return Err(TrapApiError::SystemNotInitialized);
    }

    // Call the internal function to unregister the error handler
    let result = crate::trap::infrastructure::di::unregister_error_handler(description);

    if result {
        Ok(())
    } else {
        Err(TrapApiError::HandlerNotFound)
    }
}

/// Handle a system error
///
/// This function delegates error handling to the registered error handlers.
///
/// # Parameters
///
/// * `error` - The system error to handle
///
/// # Returns
///
/// The result of error handling
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn handle_system_error(error: SystemError) -> ErrorResult {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Return unhandled if not initialized
        return ErrorResult::Unhandled;
    }

    // Call the internal function to handle the system error
    crate::trap::infrastructure::di::handle_system_error(error)
}

/// Create a new system error
///
/// # Parameters
///
/// * `source` - The source of the error
/// * `level` - The severity level of the error
/// * `code` - The error code
/// * `address` - Optional address related to the error
/// * `ip` - Instruction pointer at the time of the error
///
/// # Returns
///
/// A new system error instance
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn create_system_error(
    source: ErrorSource,
    level: ErrorLevel,
    code: u16,
    address: Option<usize>,
    ip: usize
) -> SystemError {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Create error directly if system not initialized
        let error_code = ErrorCode::new(source, level, code);
        // Use current time or zero if not available
        let time = crate::util::sbi::timer::get_time();
        return SystemError::new(error_code, address, ip, time);
    }

    // Call the internal function to create a system error
    crate::trap::infrastructure::di::create_system_error(source, level, code, address, ip)
}

/// Print the error log
///
/// # Parameters
///
/// * `count` - Number of recent errors to print
///
/// # Thread Safety
///
/// This function is safe to call from any context but may produce interleaved
/// output if called concurrently.
pub fn print_error_log(count: usize) {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        println!("Error log not available: trap system not initialized");
        return;
    }

    // Call the internal function to print the error log
    crate::trap::infrastructure::di::print_error_log(count)
}

/// Clear the error log
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn clear_error_log() {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return;
    }

    // Call the internal function to clear the error log
    crate::trap::infrastructure::di::clear_error_log()
}

/// Print the registered error handlers
///
/// # Thread Safety
///
/// This function is safe to call from any context but may produce interleaved
/// output if called concurrently.
pub fn print_error_handlers() {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        println!("Error handlers not available: trap system not initialized");
        return;
    }

    // Call the internal function to print error handlers
    crate::trap::infrastructure::di::print_error_handlers()
}

/// Check if the system is in panic mode
///
/// # Returns
///
/// * `true` if the system is in panic mode
/// * `false` if the system is not in panic mode
///
/// # Thread Safety
///
/// This function is safe to call from any context.
pub fn is_panic_mode() -> bool {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        // Conservatively assume not in panic mode
        return false;
    }

    // Call the internal function to check panic mode
    crate::trap::infrastructure::di::is_in_panic_mode()
}

/// Reset panic mode
///
/// # Thread Safety
///
/// This function is safe to call from any context but should be used with
/// extreme caution, as it affects system-wide error handling.
pub fn reset_panic_mode() {
    // Check if trap system is initialized
    if !crate::trap::infrastructure::di::get_trap_system_initialized() {
        return;
    }

    // Call the internal function to reset panic mode
    crate::trap::infrastructure::di::reset_panic_mode()
}
//! Trap System Dependency Injection Container
//!
//! This module provides the container for dependency injection in the trap system.
//! It manages component registration and lifecycle.

use crate::println;
use crate::trap::ds::{
    TrapContext, TaskContext, TrapType, TrapHandlerResult, TrapError,
    ContextType, TrapCause
};
use super::traits::{
    TrapHandlerInterface, ContextManagerInterface, 
    HardwareControlInterface, TrapSystemConfig
};

/// Static reference pointer implementation without heap allocation
///
/// This is a simple implementation that provides a way to reference static data
/// without moving ownership.
pub struct StaticRef<T> {
    data: *mut T,
}

impl<T> StaticRef<T> {
    /// Create a new static reference from a mutable pointer to static data
    ///
    /// # Safety
    ///
    /// This is unsafe because it creates a reference from a raw pointer.
    /// The pointer must be valid for the entire lifetime of the program.
    pub const unsafe fn from_static(ptr: *mut T) -> Self {
        Self {
            data: ptr,
        }
    }
    
    /// Get a mutable reference to the data
    /// 
    /// # Safety
    /// 
    /// This is unsafe because it bypasses Rust's borrowing rules.
    /// The caller must ensure exclusive access.
    pub unsafe fn get_mut(&self) -> &mut T {
        &mut *self.data
    }
    
    /// Get a shared reference to the data
    /// 
    /// # Safety
    /// 
    /// This is unsafe because it may violate Rust's borrowing rules
    /// if a mutable reference exists elsewhere.
    pub unsafe fn get(&self) -> &T {
        &*self.data
    }
}

// Safety: StaticRef<T> is Send if T is Send
unsafe impl<T: Send> Send for StaticRef<T> {}

// Safety: StaticRef<T> is Sync if T is Sync
unsafe impl<T: Sync> Sync for StaticRef<T> {}

/// Maximum number of trap handlers that can be registered
const MAX_TRAP_HANDLERS: usize = 32;

/// Trap system container
///
/// This is the main container for the trap system,
/// managing dependencies and their lifecycle.
pub struct TrapSystem<C: ContextManagerInterface, H: HardwareControlInterface> {
    /// Context manager implementation
    context_manager: StaticRef<C>,
    
    /// Hardware control implementation
    hardware_control: StaticRef<H>,
    
    /// Registered trap handlers
    ///
    /// Using fixed-size array instead of Vec to avoid heap allocation
    handlers: [Option<&'static dyn TrapHandlerInterface>; MAX_TRAP_HANDLERS],
    
    /// Number of registered handlers
    handler_count: usize,
    
    /// System configuration
    config: &'static dyn TrapSystemConfig,
}

impl<C: ContextManagerInterface, H: HardwareControlInterface> TrapSystem<C, H> {
    /// Create a new trap system with the given components
    pub const fn new(
        context_manager: StaticRef<C>,
        hardware_control: StaticRef<H>,
        config: &'static dyn TrapSystemConfig,
    ) -> Self {
        // Initialize with empty handlers
        const NONE_HANDLER: Option<&'static dyn TrapHandlerInterface> = None;
        
        Self {
            context_manager,
            hardware_control,
            handlers: [NONE_HANDLER; MAX_TRAP_HANDLERS],
            handler_count: 0,
            config,
        }
    }
    
    /// Initialize the trap system
    pub fn initialize(&mut self, mode: crate::trap::ds::TrapMode) {
        // Initialize hardware components
        unsafe {
            self.hardware_control.get().init_trap_vector(mode);
        }
        
        // Configure context manager
        unsafe {
            self.context_manager.get_mut().set_max_nest_level(
                self.config.max_interrupt_nesting_level()
            );
        }
        
        println!("Trap system initialized with {:?} mode", mode);
    }
    
    /// Register a trap handler
    ///
    /// Returns true if registration was successful, false otherwise
    pub fn register_handler(&mut self, handler: &'static dyn TrapHandlerInterface) -> bool {
        if self.handler_count >= MAX_TRAP_HANDLERS {
            println!("Cannot register handler: maximum number of handlers reached");
            return false;
        }
        
        // Find the right position based on priority and trap type
        let mut insert_index = self.handler_count;
        
        for i in 0..self.handler_count {
            if let Some(existing) = self.handlers[i] {
                // For comparing trap types, we use the fact that TrapType implements PartialEq
                if existing.get_trap_type() == handler.get_trap_type() && 
                   existing.get_priority() > handler.get_priority() {
                    // Found a handler with lower priority (higher number)
                    // Insert before this one
                    insert_index = i;
                    break;
                }
            }
        }
        
        // Shift handlers to make room
        if insert_index < self.handler_count {
            for i in (insert_index..self.handler_count).rev() {
                self.handlers[i + 1] = self.handlers[i];
            }
        }
        
        // Insert the new handler
        self.handlers[insert_index] = Some(handler);
        self.handler_count += 1;
        
        println!("Registered trap handler: {} for {:?} with priority {}", 
                 handler.get_description(), 
                 handler.get_trap_type(), 
                 handler.get_priority());
                 
        true
    }
    
    /// Unregister a trap handler by description
    ///
    /// Returns true if unregistration was successful, false otherwise
    pub fn unregister_handler(&mut self, trap_type: TrapType, description: &'static str) -> bool {
        let mut found = false;
        let mut found_index = 0;
        
        // Find the handler
        for i in 0..self.handler_count {
            if let Some(handler) = self.handlers[i] {
                if handler.get_trap_type() == trap_type && 
                   handler.get_description() == description {
                    found = true;
                    found_index = i;
                    break;
                }
            }
        }
        
        if !found {
            return false;
        }
        
        // Shift handlers to fill the gap
        for i in found_index..self.handler_count-1 {
            self.handlers[i] = self.handlers[i + 1];
        }
        
        // Clear the last slot
        self.handlers[self.handler_count - 1] = None;
        self.handler_count -= 1;
        
        println!("Unregistered trap handler: {} for {:?}", description, trap_type);
        true
    }
    
    /// Dispatch a trap to the appropriate handler
    pub fn dispatch_trap(&self, trap_type: TrapType, context: &mut TrapContext) -> TrapHandlerResult {
        // Find and call handlers for this trap type
        for i in 0..self.handler_count {
            if let Some(handler) = self.handlers[i] {
                if handler.get_trap_type() == trap_type {
                    match handler.handle_trap(context) {
                        result @ TrapHandlerResult::Handled => {
                            // Handler processed the trap successfully
                            return result;
                        }
                        TrapHandlerResult::Pass => {
                            // Handler passed, try the next one
                            continue;
                        }
                        result @ TrapHandlerResult::Failed(_) => {
                            // Handler failed, but try the next one
                            println!("Handler '{}' failed", handler.get_description());
                            continue;
                        }
                    }
                }
            }
        }
        
        // No handler processed the trap
        TrapHandlerResult::Failed(TrapError::NoHandler)
    }
    
    /// Handle a trap event
    ///
    /// This is the main entry point for trap handling
    pub fn handle_trap(&self, context: *mut TrapContext) {
        let ctx = unsafe { &mut *context };
        let cause = ctx.get_cause();
        let trap_type = cause.to_trap_type();
        
        // Record interrupt occurrence
        if cause.is_interrupt() {
            println!("Interrupt occurred: {:?}, code: {}", 
                     trap_type, cause.code());
        } else {
            println!("Exception occurred: {:?}, code: {}, addr: {:#x}", 
                     trap_type, cause.code(), ctx.stval);
        }
        
        // Dispatch to registered handlers
        match self.dispatch_trap(trap_type, ctx) {
            TrapHandlerResult::Handled => {
                println!("Interrupt handled successfully by registered handler");
            },
            TrapHandlerResult::Pass => {
                // All handlers passed this interrupt
                println!("All handlers passed the interrupt: {:?}", trap_type);
                
                // Default handling logic
                self.handle_unhandled_trap(trap_type, cause, ctx);
            },
            TrapHandlerResult::Failed(err) => {
                // Handling failed
                println!("Failed to handle interrupt: {:?}, error: {:?}", trap_type, err);
                
                // Default handling logic
                self.handle_unhandled_trap(trap_type, cause, ctx);
            }
        }
    }
    
    /// Handle an unhandled trap with default behavior
    fn handle_unhandled_trap(&self, trap_type: TrapType, cause: TrapCause, ctx: &mut TrapContext) {
        // Default handling logic
        if cause.is_interrupt() {
            match trap_type {
                TrapType::TimerInterrupt => {
                    println!("Default handling for timer interrupt");
                },
                TrapType::SoftwareInterrupt => {
                    unsafe {
                        self.hardware_control.get().clear_soft_interrupt();
                    }
                },
                TrapType::ExternalInterrupt => {
                    println!("Default handling for external interrupt");
                },
                _ => {
                    println!("No default handler for interrupt type: {:?}", trap_type);
                }
            }
        } else {
            // Exception handling
            match trap_type {
                TrapType::SystemCall => {
                    println!("Default handling for system call");
                    // Advance PC past the ecall instruction
                    ctx.set_return_addr(ctx.sepc + 4);
                },
                TrapType::InstructionPageFault | 
                TrapType::LoadPageFault | 
                TrapType::StorePageFault => {
                    println!("Unhandled page fault at address {:#x}", ctx.stval);
                },
                _ => {
                    println!("Unhandled exception: {:?} at {:#x}", trap_type, ctx.sepc);
                }
            }
        }
    }
    
    /// Get context manager implementation
    pub fn get_context_manager(&self) -> &C {
        unsafe { self.context_manager.get() }
    }
    
    /// Get mutable context manager implementation
    pub fn get_context_manager_mut(&self) -> &mut C {
        unsafe { self.context_manager.get_mut() }
    }
    
    /// Get hardware control implementation
    pub fn get_hardware_control(&self) -> &H {
        unsafe { self.hardware_control.get() }
    }
    
    /// Count handlers registered for a specific trap type
    pub fn handler_count_for_type(&self, trap_type: TrapType) -> usize {
        let mut count = 0;
        
        for i in 0..self.handler_count {
            if let Some(handler) = self.handlers[i] {
                if handler.get_trap_type() == trap_type {
                    count += 1;
                }
            }
        }
        
        count
    }
    
    /// Print all registered handlers (for debugging)
    pub fn print_handlers(&self) {
        println!("=== Registered Trap Handlers ===");
        
        // Create a map of trap types to handlers
        // Since we don't have a map data structure without heap allocation,
        // we'll iterate through all possible trap types
        for i in 0..TrapType::COUNT {
            let trap_type = TrapType::from_index(i);
            let mut handlers_found = false;
            
            // Find handlers for this trap type
            for j in 0..self.handler_count {
                if let Some(handler) = self.handlers[j] {
                    if handler.get_trap_type() == trap_type {
                        if !handlers_found {
                            println!("{:?} Handlers:", trap_type);
                            handlers_found = true;
                        }
                        
                        println!("  {}. {} (Priority: {})", 
                                 j + 1, 
                                 handler.get_description(), 
                                 handler.get_priority());
                    }
                }
            }
        }
        
        println!("===============================");
    }
}
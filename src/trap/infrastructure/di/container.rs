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
    HardwareControlInterface, TrapSystemConfig, ErrorManagerInterface
};
use super::impls::StandardTrapHandler;

/// Static reference pointer implementation without heap allocation
///
/// This is a simple implementation that provides a way to reference static data
/// without moving ownership.
pub struct StaticRef<T> {
    data: *mut T,
}

impl<T> StaticRef<T> {
    /// Create a new static reference from a mutable pointer
    ///
    /// # Safety
    ///
    /// This is safe because we ensure exclusive access through the Mutex
    pub fn new(ptr: *mut T) -> Self {
        Self {
            data: ptr,
        }
    }

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

/// Handler information structure
#[derive(Copy, Clone)]
pub struct HandlerInfo {
    /// 指向 HANDLER_STORAGE 的索引
    pub index: usize,
    /// 处理器优先级
    pub priority: u8,
    /// 处理器类型
    pub trap_type: TrapType,
}

impl HandlerInfo {
    /// 创建新的处理器信息
    pub const fn new(index: usize, priority: u8, trap_type: TrapType) -> Self {
        Self {
            index,
            priority,
            trap_type,
        }
    }
}

/// Trap system container
///
/// This is the main container for the trap system,
/// managing dependencies and their lifecycle.
pub struct TrapSystem<C: ContextManagerInterface, H: HardwareControlInterface, E: ErrorManagerInterface> {
    /// Context manager implementation
    context_manager: StaticRef<C>,

    /// Hardware control implementation
    hardware_control: StaticRef<H>,

    /// Error manager implementation
    error_manager: StaticRef<E>,

    /// Registered trap handlers - 改为使用 HandlerInfo
    handlers: [Option<HandlerInfo>; MAX_TRAP_HANDLERS],

    /// Number of registered handlers
    handler_count: usize,

    /// System configuration
    config: &'static dyn TrapSystemConfig,
}

impl<C: ContextManagerInterface, H: HardwareControlInterface, E: ErrorManagerInterface> TrapSystem<C, H, E> {
    /// Create a new trap system with the given components
    pub const fn new(
        context_manager: StaticRef<C>,
        hardware_control: StaticRef<H>,
        error_manager: StaticRef<E>,
        config: &'static dyn TrapSystemConfig,
    ) -> Self {
        // 修改为使用 HandlerInfo
        const NONE_HANDLER_INFO: Option<HandlerInfo> = None;

        Self {
            context_manager,
            hardware_control,
            error_manager,
            handlers: [NONE_HANDLER_INFO; MAX_TRAP_HANDLERS],
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
    /// 修改接口以接收索引而非直接引用
    pub fn register_handler(
        &mut self,
        index: usize,
        priority: u8,
        trap_type: TrapType,
        description: &'static str
    ) -> bool {
        if self.handler_count >= MAX_TRAP_HANDLERS {
            println!("Cannot register handler: maximum number of handlers reached");
            return false;
        }

        // 检查索引是否已注册，防止逻辑错误
        for i in 0..self.handler_count {
            if let Some(handler_info) = self.handlers[i] {
                if handler_info.index == index {
                    println!("Cannot register handler: index {} already registered", index);
                    return false;
                }
            }
        }

        // 创建 HandlerInfo 实例
        let handler_info = HandlerInfo::new(index, priority, trap_type);

        // 查找插入位置，基于trap_type和priority
        let mut insert_idx = self.handler_count;

        for i in 0..self.handler_count {
            if let Some(existing) = self.handlers[i] {
                if existing.trap_type == trap_type && existing.priority > priority {
                    // 找到优先级较低的处理器
                    insert_idx = i;
                    break;
                }
            }
        }

        // 移动现有元素
        if insert_idx < self.handler_count {
            for i in (insert_idx..self.handler_count).rev() {
                self.handlers[i + 1] = self.handlers[i];
            }
        }

        // 插入新的处理器信息
        self.handlers[insert_idx] = Some(handler_info);
        self.handler_count += 1;

        println!("Registered trap handler: {} for {:?} with priority {} (index: {})",
                 description, trap_type, priority, index);

        true
    }

    /// Unregister a trap handler by index
    pub fn unregister_handler(&mut self, index: usize) -> bool {
        let mut found = false;
        let mut found_idx = 0;

        // 查找匹配索引的处理器
        for i in 0..self.handler_count {
            if let Some(handler_info) = self.handlers[i] {
                if handler_info.index == index {
                    found = true;
                    found_idx = i;
                    break;
                }
            }
        }

        if !found {
            return false;
        }

        // 移动元素填补空位
        for i in found_idx..self.handler_count-1 {
            self.handlers[i] = self.handlers[i + 1];
        }

        // 清空最后一个位置
        self.handlers[self.handler_count - 1] = None;
        self.handler_count -= 1;

        println!("Unregistered trap handler (index: {})", index);
        true
    }

    /// Dispatch a trap to the appropriate handler
    /// 修改以接收外部存储
    pub fn dispatch_trap(
        &self,
        trap_type: TrapType,
        context: &mut TrapContext,
        storage: &[Option<StandardTrapHandler>]
    ) -> TrapHandlerResult {
        // 查找匹配的处理器
        for i in 0..self.handler_count {
            if let Some(handler_info) = self.handlers[i] {
                if handler_info.trap_type == trap_type {
                    // 从传入的存储中获取实际处理器实例
                    if let Some(handler) = &storage[handler_info.index] {
                        match handler.handle_trap(context) {
                            result @ TrapHandlerResult::Handled => {
                                // 处理成功
                                return result;
                            }
                            TrapHandlerResult::Pass => {
                                // 传递给下一个处理器
                                continue;
                            }
                            result @ TrapHandlerResult::Failed(_) => {
                                // 处理失败
                                println!("Handler failed (index: {})", handler_info.index);
                                continue;
                            }
                        }
                    } else {
                        // 索引无效或槽位为空
                        println!("Warning: Handler instance not found at index {}", handler_info.index);
                        continue;
                    }
                }
            }
        }

        // 没有处理器处理该中断
        TrapHandlerResult::Failed(TrapError::NoHandler)
    }

    /// Handle a trap event
    /// 修改以接收外部存储
    pub fn handle_trap(
        &self,
        context: *mut TrapContext,
        storage: &[Option<StandardTrapHandler>]
    ) {
        let ctx = unsafe { &mut *context };
        let cause = ctx.get_cause();
        let trap_type = cause.to_trap_type();

        // 记录中断发生
        if cause.is_interrupt() {
            println!("Interrupt occurred: {:?}, code: {}",
                     trap_type, cause.code());
        } else {
            println!("Exception occurred: {:?}, code: {}, addr: {:#x}",
                     trap_type, cause.code(), ctx.stval);
        }

        // 分发给注册的处理器
        match self.dispatch_trap(trap_type, ctx, storage) {
            TrapHandlerResult::Handled => {
                println!("Interrupt handled successfully by registered handler");
            },
            TrapHandlerResult::Pass => {
                // 所有处理器都传递了该中断
                println!("All handlers passed the interrupt: {:?}", trap_type);

                // 默认处理逻辑
                self.handle_unhandled_trap(trap_type, cause, ctx);
            },
            TrapHandlerResult::Failed(err) => {
                // 处理失败
                println!("Failed to handle interrupt: {:?}, error: {:?}", trap_type, err);

                // 默认处理逻辑
                self.handle_unhandled_trap(trap_type, cause, ctx);
            }
        }
    }

    /// Handle an unhandled trap with default behavior
    fn handle_unhandled_trap(&self, trap_type: TrapType, cause: TrapCause, ctx: &mut TrapContext) {
        // 默认处理逻辑
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
            // 异常处理
            match trap_type {
                TrapType::SystemCall => {
                    println!("Default handling for system call");
                    // 系统调用需要跳过 ecall 指令
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

    /// Get error manager implementation
    pub fn get_error_manager(&self) -> &E {
        unsafe { self.error_manager.get() }
    }

    /// Get mutable error manager implementation
    pub fn get_error_manager_mut(&self) -> &mut E {
        unsafe { self.error_manager.get_mut() }
    }

    /// Count handlers registered for a specific trap type
    pub fn handler_count_for_type(&self, trap_type: TrapType) -> usize {
        let mut count = 0;

        for i in 0..self.handler_count {
            if let Some(handler_info) = self.handlers[i] {
                if handler_info.trap_type == trap_type {
                    count += 1;
                }
            }
        }

        count
    }

    /// Print all registered handlers (for debugging)
    /// 修改以接收外部存储
    pub fn print_handlers(&self, storage: &[Option<StandardTrapHandler>]) {
        println!("=== Registered Trap Handlers ===");

        // 按中断类型分类打印
        for i in 0..TrapType::COUNT {
            let trap_type = TrapType::from_index(i);
            let mut handlers_found = false;

            // 查找该类型的所有处理器
            for j in 0..self.handler_count {
                if let Some(handler_info) = self.handlers[j] {
                    if handler_info.trap_type == trap_type {
                        if !handlers_found {
                            println!("{:?} Handlers:", trap_type);
                            handlers_found = true;
                        }

                        // 获取描述符信息
                        let description = if let Some(handler) = &storage[handler_info.index] {
                            handler.get_description()
                        } else {
                            "<missing handler>"
                        };

                        println!("  {}. {} (Priority: {}, Index: {})",
                                 j + 1, description, handler_info.priority, handler_info.index);
                    }
                }
            }
        }

        println!("===============================");
    }
}
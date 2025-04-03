//! Trap System Dependency Injection Module
//!
//! This module provides a dependency injection framework for the trap system,
//! allowing for modular and testable components.

pub mod traits;
pub mod container;
pub mod impls;
pub mod test;  // Export test module
pub mod concurrency_test;  // Export concurrency test module

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use spin::Mutex;
use crate::println;
use self::impls::StandardErrorManager;
use crate::trap::ds::{
    TrapContext, TaskContext, TrapType, TrapHandlerResult, TrapError,
    SystemError, ErrorResult, ErrorHandler, ErrorSource, ErrorLevel,
    TrapMode, Interrupt, ContextError
};
use self::impls::{StandardContextManager, RiscvHardwareControl, StandardTrapHandler};
use self::traits::DefaultTrapSystemConfig;

/// Global trap system instance flag - atomic for thread safety
static TRAP_SYSTEM_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Static storage for context manager - protected by Mutex
static CONTEXT_MANAGER: Mutex<StandardContextManager> = Mutex::new(StandardContextManager::new());

/// Static storage for hardware control - protected by Mutex
static HARDWARE_CONTROL: Mutex<RiscvHardwareControl> = Mutex::new(RiscvHardwareControl::new());

/// Static storage for trap system configuration
static TRAP_SYSTEM_CONFIG: DefaultTrapSystemConfig = DefaultTrapSystemConfig {};

/// Static storage for trap system - protected by Mutex
static TRAP_SYSTEM: Mutex<Option<TrapSystem<StandardContextManager, RiscvHardwareControl, StandardErrorManager>>> = Mutex::new(None);

/// Static storage for error manager - protected by Mutex
static ERROR_MANAGER: Mutex<StandardErrorManager> = Mutex::new(StandardErrorManager::new());

/// Default handler implementations

/// Timer interrupt handler
fn default_timer_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Timer interrupt occurred");
    TrapHandlerResult::Handled
}

/// Software interrupt handler
fn default_software_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Software interrupt occurred");
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().clear_soft_interrupt();
    });
    TrapHandlerResult::Handled
}

/// External interrupt handler
fn default_external_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("External interrupt occurred");
    TrapHandlerResult::Handled
}

/// System call handler
fn default_syscall_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("System call occurred");
    // Advance PC past the ecall instruction
    ctx.set_return_addr(ctx.sepc + 4);
    TrapHandlerResult::Handled
}

/// Page fault handler
fn default_page_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Page fault occurred, address: {:#x}", ctx.stval);
    TrapHandlerResult::Handled
}

/// Illegal instruction handler
fn default_illegal_instruction_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Illegal instruction: {:#x}", ctx.stval);
    TrapHandlerResult::Handled
}

/// Unknown trap handler
fn default_unknown_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Unknown trap: cause={:#x}, addr={:#x}", ctx.scause, ctx.stval);
    TrapHandlerResult::Handled
}

/// Static default handlers
static TIMER_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_timer_handler,
    TrapType::TimerInterrupt,
    100,
    "Default Timer Handler"
);

static SOFTWARE_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_software_handler,
    TrapType::SoftwareInterrupt,
    100,
    "Default Software Handler"
);

static EXTERNAL_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_external_handler,
    TrapType::ExternalInterrupt,
    100,
    "Default External Handler"
);

static SYSCALL_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_syscall_handler,
    TrapType::SystemCall,
    100,
    "Default System Call Handler"
);

static PAGE_FAULT_INSTRUCTION_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_page_fault_handler,
    TrapType::InstructionPageFault,
    100,
    "Default Instruction Page Fault Handler"
);

static PAGE_FAULT_LOAD_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_page_fault_handler,
    TrapType::LoadPageFault,
    100,
    "Default Load Page Fault Handler"
);

static PAGE_FAULT_STORE_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_page_fault_handler,
    TrapType::StorePageFault,
    100,
    "Default Store Page Fault Handler"
);

static ILLEGAL_INSTRUCTION_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_illegal_instruction_handler,
    TrapType::IllegalInstruction,
    100,
    "Default Illegal Instruction Handler"
);

static UNKNOWN_HANDLER: StandardTrapHandler = StandardTrapHandler::new(
    default_unknown_handler,
    TrapType::Unknown,
    100,
    "Default Unknown Handler"
);

/// Initialize the trap system with dependency injection
///
/// # 并发安全性
///
/// 此函数使用原子变量确保只初始化一次，即使多个核心并发调用也安全。
pub fn initialize_trap_system(mode: TrapMode) {
    // Use CAS operation to safely check and set initialization flag
    if TRAP_SYSTEM_INITIALIZED.compare_exchange(
        false, true, Ordering::SeqCst, Ordering::SeqCst
    ).is_err() {
        println!("Trap system already initialized");
        return;
    }
    
    // Create static references using raw pointers to static data with lock protection
    let context_manager = {
        let mut cm = CONTEXT_MANAGER.lock();
        container::StaticRef::new(&mut *cm as *mut StandardContextManager)
    };
    
    let hardware_control = {
        let mut hc = HARDWARE_CONTROL.lock();
        container::StaticRef::new(&mut *hc as *mut RiscvHardwareControl)
    };
    
    let error_manager = {
        let mut em = ERROR_MANAGER.lock();
        container::StaticRef::new(&mut *em as *mut StandardErrorManager)
    };
    
    // Create trap system
    let mut trap_system = container::TrapSystem::new(
        context_manager,
        hardware_control,
        error_manager,
        &TRAP_SYSTEM_CONFIG,
    );
    
    // Initialize the system
    trap_system.initialize(mode);
    
    // Register default handlers
    trap_system.register_handler(&TIMER_HANDLER);
    trap_system.register_handler(&SOFTWARE_HANDLER);
    trap_system.register_handler(&EXTERNAL_HANDLER);
    trap_system.register_handler(&SYSCALL_HANDLER);
    trap_system.register_handler(&PAGE_FAULT_INSTRUCTION_HANDLER);
    trap_system.register_handler(&PAGE_FAULT_LOAD_HANDLER);
    trap_system.register_handler(&PAGE_FAULT_STORE_HANDLER);
    trap_system.register_handler(&ILLEGAL_INSTRUCTION_HANDLER);
    trap_system.register_handler(&UNKNOWN_HANDLER);
    
    // Store the trap system
    {
        let mut ts = TRAP_SYSTEM.lock();
        *ts = Some(trap_system);
    }
    
    println!("Trap system initialized with dependency injection");
}

/// Execute a function with a reference to the trap system
/// 
/// # 并发安全性
///
/// 此函数使用Mutex确保在中断上下文和多核环境中的安全访问。
/// 不要在持有锁时禁用中断，否则可能导致死锁。
///
/// # Panics
///
/// Panics if the trap system is not initialized
pub fn with_trap_system<F, R>(f: F) -> R
where
    F: FnOnce(&TrapSystem<StandardContextManager, RiscvHardwareControl, StandardErrorManager>) -> R,
{
    if !TRAP_SYSTEM_INITIALIZED.load(Ordering::SeqCst) {
        panic!("Trap system not initialized");
    }
    
    let guard = TRAP_SYSTEM.lock();
    let trap_system = guard.as_ref().expect("Trap system is None but initialized flag is true");
    f(trap_system)
}

/// Execute a function with a mutable reference to the trap system
/// 
/// # 并发安全性
///
/// 此函数使用Mutex确保在中断上下文和多核环境中的安全访问。
/// 不要在持有锁时禁用中断，否则可能导致死锁。
///
/// # Panics
///
/// Panics if the trap system is not initialized
pub fn with_trap_system_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut TrapSystem<StandardContextManager, RiscvHardwareControl, StandardErrorManager>) -> R,
{
    if !TRAP_SYSTEM_INITIALIZED.load(Ordering::SeqCst) {
        panic!("Trap system not initialized");
    }
    
    let mut guard = TRAP_SYSTEM.lock();
    let trap_system = guard.as_mut().expect("Trap system is None but initialized flag is true");
    f(trap_system)
}

/// Check if the trap system is initialized
pub fn get_trap_system_initialized() -> bool {
    TRAP_SYSTEM_INITIALIZED.load(Ordering::SeqCst)
}

// Maximum number of custom handlers we can support
const MAX_CUSTOM_HANDLERS: usize = 32;

/// 中断处理器注册表状态
/// 
/// 封装所有与处理器注册相关的状态，便于通过单一锁保护
pub struct DiHandlerRegistryState {
    /// 处理器存储 - 保存所有注册的处理器实例
    storage: [Option<StandardTrapHandler>; MAX_CUSTOM_HANDLERS],
    /// 处理器引用 - 指向存储中处理器的静态引用
    handlers: [Option<&'static StandardTrapHandler>; MAX_CUSTOM_HANDLERS],
    /// 当前注册的处理器数量
    count: usize,
}

impl DiHandlerRegistryState {
    /// 创建新的处理器注册表状态
    pub const fn new() -> Self {
        const NONE_HANDLER: Option<StandardTrapHandler> = None;
        const NONE_REF: Option<&'static StandardTrapHandler> = None;
        
        Self {
            storage: [NONE_HANDLER; MAX_CUSTOM_HANDLERS],
            handlers: [NONE_REF; MAX_CUSTOM_HANDLERS],
            count: 0,
        }
    }
}

// Static storage for custom handlers - thread-safe using Mutex and atomic counter
//static CUSTOM_HANDLERS: Mutex<[Option<&'static StandardTrapHandler>; MAX_CUSTOM_HANDLERS]> = Mutex::new([None; MAX_CUSTOM_HANDLERS]);
//static HANDLER_STORAGE: Mutex<[Option<StandardTrapHandler>; MAX_CUSTOM_HANDLERS]> = Mutex::new([None; MAX_CUSTOM_HANDLERS]);
//static CUSTOM_HANDLER_COUNT: AtomicUsize = AtomicUsize::new(0);

// 替换为单一的受保护状态
static HANDLER_REGISTRY: Mutex<DiHandlerRegistryState> = Mutex::new(DiHandlerRegistryState::new());

/// Register a custom trap handler
///
/// # 并发安全性
///
/// 此函数使用锁和原子操作保护共享数据，在中断上下文或多核环境中安全。
pub fn register_handler(
    trap_type: TrapType,
    handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
    priority: u8,
    description: &'static str
) -> bool {
    // 获取注册表锁
    let mut registry = HANDLER_REGISTRY.lock();
    
    // 检查是否已达到最大处理器数量
    if registry.count >= MAX_CUSTOM_HANDLERS {
        println!("Cannot register handler: maximum number of custom handlers reached");
        return false;
    }
    
    // 创建处理器
    let handler = StandardTrapHandler::new(
        handler_fn,
        trap_type,
        priority,
        description
    );
    
    // 获取当前索引并准备存储 - 使用局部变量避免借用冲突
    let current_idx = registry.count;
    
    // 存储在我们的数组中
    registry.storage[current_idx] = Some(handler);
    
    // 创建静态引用并存储 - 这仍需要unsafe，但在锁保护下进行
    let handler_ref: &'static StandardTrapHandler;
    unsafe {
        if let Some(ref h) = registry.storage[current_idx] {
            // 这是安全的，因为storage数组在整个程序生命周期内存在
            handler_ref = core::mem::transmute(h);
            registry.handlers[current_idx] = Some(handler_ref);
        } else {
            return false;
        }
    }
    
    // 通过trap系统注册
    let result = with_trap_system_mut(|trap_system| {
        trap_system.register_handler(handler_ref)
    });
    
    // 如果成功，更新计数
    if result {
        registry.count += 1;
    }
    
    result
}

/// Unregister a trap handler
///
/// # 并发安全性
///
/// 此函数同时更新trap系统和本地注册表状态，
/// 确保在多核环境中的一致性
pub fn unregister_handler(trap_type: TrapType, description: &'static str) -> bool {
    // 首先通过trap系统注销
    let result = with_trap_system_mut(|trap_system| {
        trap_system.unregister_handler(trap_type, description)
    });
    
    // 如果成功，也从我们的注册表中移除
    if result {
        let mut registry = HANDLER_REGISTRY.lock();
        
        // 查找匹配的处理器
        let mut found_idx = None;
        for i in 0..registry.count {
            if let Some(handler) = registry.handlers[i] {
                if handler.get_trap_type() == trap_type && 
                   handler.get_description() == description {
                    // 找到匹配项
                    found_idx = Some(i);
                    break;
                }
            }
        }
        
        // 如果找到处理器，移除它
        if let Some(idx) = found_idx {
            // 获取当前计数值
            let current_count = registry.count;
            
            // 将所有后续元素前移
            for j in idx..current_count-1 {
                // 解决借用问题 - 先取出下一个元素
                let next_storage = registry.storage[j+1].take();
                let next_handler = registry.handlers[j+1].take();
                
                // 然后赋值给当前元素
                registry.storage[j] = next_storage;
                registry.handlers[j] = next_handler;
            }
            
            // 清除最后一个位置 - 使用局部变量避免借用冲突
            let last_idx = current_count - 1;
            registry.storage[last_idx] = None;
            registry.handlers[last_idx] = None;
            
            // 减少计数
            registry.count -= 1;
        }
    }
    
    result
}

/// Get the number of handlers registered for a trap type
pub fn handler_count(trap_type: TrapType) -> usize {
    with_trap_system(|trap_system| {
        trap_system.handler_count_for_type(trap_type)
    })
}

/// Print all registered handlers
pub fn print_handlers() {
    with_trap_system(|trap_system| {
        trap_system.print_handlers()
    })
}

/// Enable interrupts
pub fn enable_interrupts() -> bool {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().enable_interrupts()
    })
}

/// Disable interrupts
pub fn disable_interrupts() -> bool {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().disable_interrupts()
    })
}

/// Restore interrupts
pub fn restore_interrupts(was_enabled: bool) {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().restore_interrupts(was_enabled)
    })
}

/// Enable a specific interrupt
pub fn enable_interrupt(interrupt: Interrupt) {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().enable_interrupt(interrupt)
    })
}

/// Disable a specific interrupt
pub fn disable_interrupt(interrupt: Interrupt) {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().disable_interrupt(interrupt)
    })
}

/// Check if an interrupt is enabled
pub fn is_interrupt_enabled(interrupt: Interrupt) -> bool {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().is_interrupt_enabled(interrupt)
    })
}

/// Check if an interrupt is pending
pub fn is_interrupt_pending(interrupt: Interrupt) -> bool {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().is_interrupt_pending(interrupt)
    })
}

/// Set a software interrupt
pub fn set_soft_interrupt() {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().set_soft_interrupt()
    })
}

/// Clear a software interrupt
pub fn clear_soft_interrupt() {
    with_trap_system(|trap_system| {
        trap_system.get_hardware_control().clear_soft_interrupt()
    })
}

/// Create a task context
pub fn create_task_context(entry: usize, user_stack: usize, kernel_stack: usize) -> TrapContext {
    with_trap_system(|trap_system| {
        trap_system.get_context_manager().create_task_context(
            entry, user_stack, kernel_stack, 0
        )
    })
}

/// Switch task context
pub fn switch_task_context(current: &mut TaskContext, next: &TaskContext) {
    with_trap_system_mut(|trap_system| {
        trap_system.get_context_manager_mut().switch_task_context(
            current, next
        )
    })
}

/// Check if in interrupt context
pub fn is_in_interrupt_context() -> bool {
    with_trap_system(|trap_system| {
        trap_system.get_context_manager().is_in_interrupt_context()
    })
}

/// Get the current interrupt nesting level
pub fn get_interrupt_nest_level() -> usize {
    with_trap_system(|trap_system| {
        trap_system.get_context_manager().get_nest_level()
    })
}

/// Internal function to handle trap events without conflicting with the main handler
pub fn internal_handle_trap(context: *mut TrapContext) {
    with_trap_system(|trap_system| {
        trap_system.handle_trap(context);
    })
}

/// 获取自定义处理器数量
///
/// 返回通过DI系统注册的自定义处理器总数
pub fn custom_handler_count() -> usize {
    let registry = HANDLER_REGISTRY.lock();
    registry.count
}

/// 打印所有注册的自定义处理器
///
/// 显示通过DI系统注册的所有处理器信息
pub fn print_custom_handlers() {
    let registry = HANDLER_REGISTRY.lock();
    println!("=== Registered Custom Handlers ({}) ===", registry.count);
    
    for i in 0..registry.count {
        if let Some(handler) = registry.handlers[i] {
            println!("{}. {} (Type: {:?}, Priority: {})",
                    i + 1,
                    handler.get_description(),
                    handler.get_trap_type(),
                    handler.get_priority());
        }
    }
    
    println!("=======================================");
}

/// Register an error handler
pub fn register_error_handler(
    handler: ErrorHandler,
    priority: u8,
    description: &'static str,
    source: Option<ErrorSource>,
    level: Option<ErrorLevel>
) -> bool {
    with_trap_system_mut(|trap_system| {
        trap_system.get_error_manager_mut().register_handler(
            handler, priority, description, source, level
        )
    })
}

/// Unregister an error handler
pub fn unregister_error_handler(description: &str) -> bool {
    with_trap_system_mut(|trap_system| {
        trap_system.get_error_manager_mut().unregister_handler(description)
    })
}

/// Handle a system error
pub fn handle_system_error(error: SystemError) -> ErrorResult {
    with_trap_system_mut(|trap_system| {
        trap_system.get_error_manager_mut().handle_error(error)
    })
}

/// Create a new system error
pub fn create_system_error(
    source: ErrorSource,
    level: ErrorLevel,
    code: u16,
    address: Option<usize>,
    ip: usize
) -> SystemError {
    with_trap_system(|trap_system| {
        trap_system.get_error_manager().create_error(
            source, level, code, address, ip
        )
    })
}

/// Print error log
pub fn print_error_log(count: usize) {
    with_trap_system(|trap_system| {
        trap_system.get_error_manager().print_error_log(count)
    })
}

/// Clear error log
pub fn clear_error_log() {
    with_trap_system_mut(|trap_system| {
        trap_system.get_error_manager_mut().clear_error_log()
    })
}

/// Print registered error handlers
pub fn print_error_handlers() {
    with_trap_system(|trap_system| {
        trap_system.get_error_manager().print_handlers()
    })
}

/// Check if in panic mode
pub fn is_in_panic_mode() -> bool {
    with_trap_system(|trap_system| {
        trap_system.get_error_manager().is_panic_mode()
    })
}

/// Reset panic mode
pub fn reset_panic_mode() {
    with_trap_system(|trap_system| {
        trap_system.get_error_manager().reset_panic_mode()
    })
}

// 导出公共函数和接口
pub use self::container::{TrapSystem, StaticRef};
pub use self::traits::{
    TrapHandlerInterface, ContextManagerInterface, 
    HardwareControlInterface, TrapSystemConfig, ErrorManagerInterface
};
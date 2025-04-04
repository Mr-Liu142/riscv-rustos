//! Trap System Dependency Injection Module
//!
//! This module provides a dependency injection framework for the trap system,
//! allowing for modular and testable components.

pub mod traits;
pub mod container;
pub mod impls;
pub mod test;  // Export test module
pub mod concurrency_test;  // Export concurrency test module
pub mod context;
pub mod context_pool;

use self::context::{ContextId, KERNEL_CONTEXT_ID};

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
use self::container::MAX_TRAP_HANDLERS;

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

/// Maximum number of custom handlers
const MAX_CUSTOM_HANDLERS: usize = 64;

/// Static storage for handler instances
static HANDLER_STORAGE: Mutex<[Option<StandardTrapHandler>; MAX_CUSTOM_HANDLERS]> = {
    const NONE_HANDLER: Option<StandardTrapHandler> = None;
    Mutex::new([NONE_HANDLER; MAX_CUSTOM_HANDLERS])
};

/// 为默认处理器预留的存储槽位范围
const DEFAULT_HANDLER_START_IDX: usize = 0;
const DEFAULT_HANDLER_END_IDX: usize = 9; // 预留10个槽位给默认处理器

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

/// Breakpoint handler
fn default_breakpoint_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Breakpoint occurred at: {:#x}", ctx.sepc);
    // 断点处理需要手动前进PC
    ctx.set_return_addr(ctx.sepc + 4);
    TrapHandlerResult::Handled
}

/// Unknown trap handler
fn default_unknown_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    println!("Unknown trap: cause={:#x}, addr={:#x}", ctx.scause, ctx.stval);
    TrapHandlerResult::Handled
}

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

    // Store the trap system
    {
        let mut ts = TRAP_SYSTEM.lock();
        *ts = Some(trap_system);
    }

    println!("Trap system initialized with dependency injection");

    // 注册默认处理器
    println!("Registering default trap handlers...");

    let default_handlers_registered = register_default_handlers();
    println!("Registered {} default trap handlers", default_handlers_registered);
}

/// 内部函数：注册默认处理器
fn register_default_handler(
    trap_type: TrapType,
    handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
    priority: u8,
    description: &'static str
) -> bool {
    // 加锁 HANDLER_STORAGE
    let storage_result = HANDLER_STORAGE.try_lock();
    let mut storage = match storage_result {
        Some(guard) => guard,
        None => {
            println!("Cannot register default handler: storage lock busy");
            return false;
        }
    };

    // 为默认处理器查找槽位 - 仅在预留范围内
    let mut idx = DEFAULT_HANDLER_END_IDX;
    for i in DEFAULT_HANDLER_START_IDX..=DEFAULT_HANDLER_END_IDX {
        if storage[i].is_none() {
            idx = i;
            break;
        }
    }

    if idx == DEFAULT_HANDLER_END_IDX && storage[idx].is_some() {
        println!("Cannot register default handler: no empty slots in reserved range");
        return false;
    }

    // 创建并存储处理器实例
    let handler = StandardTrapHandler::new(
        handler_fn,
        trap_type,
        priority,
        description
    );

    storage[idx] = Some(handler);

    // 释放锁，防止死锁
    drop(storage);

    // 调用 trap_system 注册处理器 - 使用内核上下文ID
    let result = with_trap_system_mut(|trap_system| {
        trap_system.register_handler(idx, priority, trap_type, description, KERNEL_CONTEXT_ID)
    });

    // 如果注册失败，回滚
    if !result {
        if let Some(mut storage) = HANDLER_STORAGE.try_lock() {
            storage[idx] = None;
            println!("Failed to register default handler in trap system, rolling back storage");
        } else {
            println!("Warning: Failed to roll back handler registration, storage lock busy");
        }
    }

    result
}

/// 注册默认处理器的实现
fn register_default_handlers() -> usize {
    let mut registered_count = 0;

    // 注册定时器中断默认处理器
    if register_default_handler(
        TrapType::TimerInterrupt,
        default_timer_handler,
        100,
        "Default Timer Handler"
    ) {
        registered_count += 1;
    }

    // 注册软件中断默认处理器
    if register_default_handler(
        TrapType::SoftwareInterrupt,
        default_software_handler,
        100,
        "Default Software Handler"
    ) {
        registered_count += 1;
    }

    // 注册外部中断默认处理器
    if register_default_handler(
        TrapType::ExternalInterrupt,
        default_external_handler,
        100,
        "Default External Handler"
    ) {
        registered_count += 1;
    }

    // 注册系统调用默认处理器
    if register_default_handler(
        TrapType::SystemCall,
        default_syscall_handler,
        100,
        "Default System Call Handler"
    ) {
        registered_count += 1;
    }

    // 注册指令页错误默认处理器
    if register_default_handler(
        TrapType::InstructionPageFault,
        default_page_fault_handler,
        100,
        "Default Instruction Page Fault Handler"
    ) {
        registered_count += 1;
    }

    // 注册加载页错误默认处理器
    if register_default_handler(
        TrapType::LoadPageFault,
        default_page_fault_handler,
        100,
        "Default Load Page Fault Handler"
    ) {
        registered_count += 1;
    }

    // 注册存储页错误默认处理器
    if register_default_handler(
        TrapType::StorePageFault,
        default_page_fault_handler,
        100,
        "Default Store Page Fault Handler"
    ) {
        registered_count += 1;
    }

    // 注册非法指令默认处理器
    if register_default_handler(
        TrapType::IllegalInstruction,
        default_illegal_instruction_handler,
        100,
        "Default Illegal Instruction Handler"
    ) {
        registered_count += 1;
    }

    // 注册未知中断默认处理器
    if register_default_handler(
        TrapType::Unknown,
        default_unknown_handler,
        100,
        "Default Unknown Handler"
    ) {
        registered_count += 1;
    }

    // 注册断点默认处理器
    if register_default_handler(
        TrapType::Breakpoint,
        default_breakpoint_handler,
        100,
        "Default Breakpoint Handler"
    ) {
        registered_count += 1;
    }

    registered_count
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

/// Register a custom trap handler
///
/// # 并发安全性
///
/// 此函数使用锁和原子操作保护共享数据，在中断上下文或多核环境中安全。
pub fn register_handler(
    trap_type: TrapType,
    handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
    priority: u8,
    description: &'static str,
    context_id: Option<ContextId>
) -> bool {
    // 检查trap系统是否初始化
    if !get_trap_system_initialized() {
        println!("Cannot register handler: trap system not initialized");
        return false;
    }

    // 加锁 HANDLER_STORAGE
    let storage_result = HANDLER_STORAGE.try_lock();
    let mut storage = match storage_result {
        Some(guard) => guard,
        None => {
            println!("Cannot register handler: handler storage lock busy");
            return false;
        }
    };

    // 检查传入的 description 在 HANDLER_STORAGE 中是否已存在
    for i in 0..MAX_CUSTOM_HANDLERS {
        if let Some(handler) = &storage[i] {
            if handler.get_description() == description &&
                handler.get_trap_type() == trap_type {
                println!("Cannot register handler: description '{}' already exists for trap type {:?}",
                         description, trap_type);
                return false;
            }
        }
    }

    // 查找第一个空槽位 - 从默认处理器范围之后开始
    let mut idx = MAX_CUSTOM_HANDLERS;
    for i in (DEFAULT_HANDLER_END_IDX + 1)..MAX_CUSTOM_HANDLERS {
        if storage[i].is_none() {
            idx = i;
            break;
        }
    }

    // 输出调试信息
    println!("Handler registration: found slot at index {}, type {:?}, desc '{}', context_id: {:?}",
             idx, trap_type, description, context_id);

    if idx == MAX_CUSTOM_HANDLERS {
        println!("Cannot register handler: no empty slots in storage (all {} slots are full)",
                 MAX_CUSTOM_HANDLERS);
        // 打印已占用的槽位
        println!("Occupied slots:");
        let mut count = 0;
        for i in 0..MAX_CUSTOM_HANDLERS {
            if let Some(handler) = &storage[i] {
                count += 1;
                println!("  Slot {}: {:?} - '{}'",
                         i, handler.get_trap_type(), handler.get_description());
            }
        }
        println!("Total occupied: {}/{}", count, MAX_CUSTOM_HANDLERS);
        return false;
    }

    // 创建并存储处理器实例
    let handler = StandardTrapHandler::new(
        handler_fn,
        trap_type,
        priority,
        description
    );

    storage[idx] = Some(handler);

    // 释放锁，防止死锁
    drop(storage);

    // 调用 trap_system 注册处理器
    let trap_result = with_trap_system_mut(|trap_system| {
        trap_system.register_handler(idx, priority, trap_type, description, context_id)
    });

    // 如果注册失败，回滚
    if !trap_result {
        if let Some(mut storage) = HANDLER_STORAGE.try_lock() {
            storage[idx] = None;
            println!("Failed to register handler in trap system, rolling back storage");
        } else {
            println!("Warning: Failed to roll back handler registration, storage lock busy");
        }
        return false;
    }

    trap_result
}

// 添加一个便利函数，默认使用内核上下文
/// 使用内核上下文注册中断处理器（便利函数）
pub fn register_handler_with_kernel_context(
    trap_type: TrapType,
    handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
    priority: u8,
    description: &'static str
) -> bool {
    register_handler(trap_type, handler_fn, priority, description, KERNEL_CONTEXT_ID)
}

/// 注销指定上下文的所有中断处理器
///
/// # 参数
///
/// * `context_id` - 要注销处理器的上下文ID
///
/// # 返回值
///
/// 返回成功注销的处理器数量
///
/// # 并发安全性
///
/// 此函数使用锁和原子操作保护共享数据，在中断上下文或多核环境中安全。
pub fn unregister_handlers_for_context(context_id: ContextId) -> usize {
    // 如果trap系统未初始化，直接返回
    if !get_trap_system_initialized() {
        println!("Cannot unregister handlers: trap system not initialized");
        return 0;
    }
    
    // 使用TrapSystem的方法获取存储索引
    let storage_indices = with_trap_system_mut(|trap_system| {
        trap_system.unregister_handlers_for_context(context_id)
    });
    
    // 清理HANDLER_STORAGE
    let mut unregistered_count = 0;
    let storage_guard = HANDLER_STORAGE.try_lock();
    if let Some(mut storage) = storage_guard {
        for i in 0..MAX_TRAP_HANDLERS {
            if let Some(index) = storage_indices[i] {
                if storage[index].is_some() {
                    let handler_desc: &'static str = if let Some(ref handler) = storage[index] {
                        handler.get_description()
                    } else {
                        "unknown"
                    };
                    
                    storage[index] = None;
                    println!("Unregistered handler at storage index {}: {}", index, handler_desc);
                    unregistered_count += 1;
                }
            } else if i > 0 {
                // 如果遇到None且不是第一个元素，说明已经处理完所有有效索引
                break;
            }
        }
    } else {
        println!("Warning: Could not lock handler storage to clean up.");
    }
    
    println!("Successfully unregistered {} handlers for context ID: {}", unregistered_count, context_id);
    unregistered_count
}

/// Unregister a trap handler
///
/// # 并发安全性
///
/// 此函数同时更新trap系统和本地注册表状态，
/// 确保在多核环境中的一致性
pub fn unregister_handler(trap_type: TrapType, description: &'static str) -> bool {
    // 加锁 HANDLER_STORAGE 用于查找
    let storage = HANDLER_STORAGE.lock();

    // 根据 trap_type 和 description 查找索引
    let mut idx = MAX_CUSTOM_HANDLERS;
    for i in 0..MAX_CUSTOM_HANDLERS {
        if let Some(handler) = &storage[i] {
            if handler.get_description() == description &&
                handler.get_trap_type() == trap_type {
                idx = i;
                break;
            }
        }
    }

    if idx == MAX_CUSTOM_HANDLERS {
        println!("Cannot unregister handler: description '{}' not found for trap type {:?}",
                 description, trap_type);
        return false;
    }

    // 释放查找锁
    drop(storage);

    // 调用 trap_system 注销处理器
    let result = with_trap_system_mut(|trap_system| {
        trap_system.unregister_handler(idx)
    });

    // 如果注销成功，清除存储
    if result {
        let mut storage = HANDLER_STORAGE.lock();
        storage[idx] = None;
        println!("Unregistered trap handler: {} for {:?} (index: {})",
                 description, trap_type, idx);
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
    // 锁定 HANDLER_STORAGE
    let storage = HANDLER_STORAGE.lock();

    // 调用 trap_system 打印处理器 - 需要转换为切片
    with_trap_system(|trap_system| {
        trap_system.print_handlers(&storage[..]);
    });
}

/// Internal function to handle trap events without conflicting with the main handler
pub fn internal_handle_trap(context: *mut TrapContext) {
    // 锁定 HANDLER_STORAGE
    let storage = HANDLER_STORAGE.lock();

    // 调用 trap_system 处理中断 - 需要转换为切片
    with_trap_system(|trap_system| {
        trap_system.handle_trap(context, &storage[..]);
    });

    // 锁会在函数返回时自动释放
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

/// 获取自定义处理器数量
///
/// 返回通过DI系统注册的自定义处理器总数
pub fn custom_handler_count() -> usize {
    let storage = HANDLER_STORAGE.lock();
    let mut count = 0;
    for i in 0..MAX_CUSTOM_HANDLERS {
        if storage[i].is_some() {
            count += 1;
        }
    }
    count
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
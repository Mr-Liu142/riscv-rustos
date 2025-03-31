//! 上下文管理器抽象层
//!
//! 此模块提供了上下文操作的高级抽象，封装底层细节，
//! 并提供嵌套中断处理和上下文生命周期管理。

use core::marker::PhantomData;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::println;
use super::context::{TrapContext, TaskContext};

/// 上下文数据所有权标记，用于提供类型安全
pub struct ContextOwnership<T>(PhantomData<T>);

/// 上下文管理错误
#[derive(Debug, Clone, Copy)]
pub enum ContextError {
    /// 栈溢出
    StackOverflow,
    /// 栈下溢（尝试从空栈弹出）
    StackUnderflow,
    /// 上下文无效
    InvalidContext,
    /// 内存不足
    OutOfMemory,
    /// 操作不允许
    OperationNotAllowed,
}

/// 上下文类型枚举
#[derive(Debug, Clone, Copy)]
pub enum ContextType {
    /// 任务上下文
    Task,
    /// 中断上下文
    Trap,
}

/// 上下文状态枚举
#[derive(Debug, Clone, Copy)]
pub enum ContextState {
    /// 活动状态
    Active,
    /// 挂起状态
    Suspended,
    /// 等待状态
    Waiting,
    /// 已完成状态
    Terminated,
}

/// 中断嵌套计数器
static INTERRUPT_NEST_COUNT: AtomicUsize = AtomicUsize::new(0);

/// 上下文管理器
/// 
/// 提供上下文操作的高层抽象，管理上下文的生命周期。
pub struct ContextManager {
    /// 中断栈
    interrupt_stack: [u8; Self::INTERRUPT_STACK_SIZE],
    /// 当前中断栈顶指针
    interrupt_stack_top: usize,
    /// 最大允许的嵌套中断层级
    max_nest_level: usize,
}

impl ContextManager {
    /// 中断栈大小（16KB）
    pub const INTERRUPT_STACK_SIZE: usize = 16 * 1024;
    
    /// 默认最大嵌套层级
    pub const DEFAULT_MAX_NEST_LEVEL: usize = 8;
    
    /// 创建新的上下文管理器
    pub const fn new() -> Self {
        Self {
            interrupt_stack: [0; Self::INTERRUPT_STACK_SIZE],
            interrupt_stack_top: 0,
            max_nest_level: Self::DEFAULT_MAX_NEST_LEVEL,
        }
    }
    
    /// 获取当前中断嵌套层级
    pub fn get_nest_level() -> usize {
        INTERRUPT_NEST_COUNT.load(Ordering::Relaxed)
    }
    
    /// 增加中断嵌套层级
    fn enter_interrupt(&mut self) -> Result<usize, ContextError> {
        let current = INTERRUPT_NEST_COUNT.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_nest_level {
            // 回滚计数器
            INTERRUPT_NEST_COUNT.fetch_sub(1, Ordering::SeqCst);
            return Err(ContextError::StackOverflow);
        }
        Ok(current + 1)
    }
    
    /// 减少中断嵌套层级
    fn exit_interrupt(&mut self) -> Result<usize, ContextError> {
        let current = INTERRUPT_NEST_COUNT.load(Ordering::Relaxed);
        if current == 0 {
            return Err(ContextError::StackUnderflow);
        }
        
        Ok(INTERRUPT_NEST_COUNT.fetch_sub(1, Ordering::SeqCst) - 1)
    }
    
    /// 设置最大嵌套层级
    pub fn set_max_nest_level(&mut self, level: usize) {
        self.max_nest_level = level;
    }
    
    /// 为中断保存当前上下文
    /// 
    /// 返回上下文指针和嵌套层级
    pub fn save_context_for_interrupt(&mut self) -> Result<(*mut TrapContext, usize), ContextError> {
        // 增加嵌套层级
        let level = self.enter_interrupt()?;
        
        // 计算栈位置
        let stack_offset = level * core::mem::size_of::<TrapContext>();
        if stack_offset + core::mem::size_of::<TrapContext>() > Self::INTERRUPT_STACK_SIZE {
            self.exit_interrupt().ok(); // 减少嵌套层级
            return Err(ContextError::StackOverflow);
        }
        
        // 使用中断栈上正确的位置保存上下文
        let ctx_ptr = unsafe {
            self.interrupt_stack.as_mut_ptr().add(stack_offset) as *mut TrapContext
        };
        
        // 创建新的上下文
        unsafe {
            *ctx_ptr = TrapContext::new();
        }
        
        // 返回上下文指针和嵌套层级
        Ok((ctx_ptr, level))
    }
    
    /// 恢复中断上下文
    pub fn restore_context_from_interrupt(&mut self, ctx: &TrapContext) -> Result<(), ContextError> {
        // 减少嵌套层级
        self.exit_interrupt()?;
        
        // 调用底层恢复函数
        unsafe {
            crate::trap::infrastructure::restore_full_context(ctx);
        }
        
        Ok(())
    }
    
    /// 保存完整上下文
    pub fn save_full_context(&mut self) -> TrapContext {
        crate::trap::infrastructure::save_full_context()
    }
    
    /// 安全地切换任务上下文
    pub fn switch_task_context(&mut self, current: &mut TaskContext, next: &TaskContext) {
        // 使用底层任务切换函数
        unsafe {
            crate::trap::infrastructure::task_switch(current, next);
        }
    }
    
    /// 为任务创建上下文
    pub fn create_task_context(
        &self,
        entry: usize,
        user_stack: usize,
        kernel_stack: usize,
        priviledge_level: u8,
    ) -> TrapContext {
        let satp = 0; // 页表基址，可以从外部传入
        
        // 调用基础设施的上下文创建函数
        crate::trap::infrastructure::prepare_task_context(
            entry, user_stack, kernel_stack, satp
        )
    }
    
    /// 根据上下文类型获取底层上下文结构大小
    pub fn get_context_size(&self, context_type: ContextType) -> usize {
        match context_type {
            ContextType::Task => core::mem::size_of::<TaskContext>(),
            ContextType::Trap => core::mem::size_of::<TrapContext>(),
        }
    }
    
    /// 获取当前中断栈使用情况
    pub fn get_interrupt_stack_usage(&self) -> (usize, usize) {
        let used = Self::get_nest_level() * core::mem::size_of::<TrapContext>();
        (used, Self::INTERRUPT_STACK_SIZE)
    }
    
    /// 检查是否在中断上下文中
    pub fn is_in_interrupt_context() -> bool {
        Self::get_nest_level() > 0
    }
}

/// RAII风格的中断上下文守卫
/// 
/// 自动在作用域结束时恢复上下文
pub struct InterruptContextGuard<'a> {
    /// 引用上下文管理器
    manager: &'a mut ContextManager,
    /// 上下文数据
    context: *mut TrapContext,
    /// 中断嵌套层级
    nest_level: usize,
}

impl<'a> InterruptContextGuard<'a> {
    /// 创建新的中断上下文守卫
    pub fn new(manager: &'a mut ContextManager) -> Result<Self, ContextError> {
        let (context, nest_level) = manager.save_context_for_interrupt()?;
        
        Ok(Self {
            manager,
            context,
            nest_level,
        })
    }
    
    /// 获取上下文引用
    pub fn get_context(&self) -> &mut TrapContext {
        unsafe { &mut *self.context }
    }
    
    /// 获取嵌套层级
    pub fn get_nest_level(&self) -> usize {
        self.nest_level
    }
}

impl<'a> Drop for InterruptContextGuard<'a> {
    fn drop(&mut self) {
        // 在作用域结束时自动恢复上下文
        let context = unsafe { &*self.context };
        if let Err(err) = self.manager.restore_context_from_interrupt(context) {
            // 错误处理 - 实际情况可能需要更严格的措施
            println!("Error restoring context: {:?}", err);
        }
    }
}

/// 单例模式实现全局上下文管理器
static mut GLOBAL_CONTEXT_MANAGER: Option<ContextManager> = None;

/// 全局接口函数

/// 初始化全局上下文管理器
pub fn init_global_context_manager() {
    unsafe {
        GLOBAL_CONTEXT_MANAGER = Some(ContextManager::new());
    }
    
    println!("Global context manager initialized");
}

/// 获取全局上下文管理器引用
pub fn get_context_manager() -> &'static mut ContextManager {
    unsafe {
        GLOBAL_CONTEXT_MANAGER.as_mut().expect("Context manager not initialized")
    }
}

/// 是否在中断上下文中
pub fn is_in_interrupt_context() -> bool {
    ContextManager::is_in_interrupt_context()
}

/// 获取当前中断嵌套层级
pub fn get_interrupt_nest_level() -> usize {
    ContextManager::get_nest_level()
}
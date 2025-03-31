//! 中断处理器注册表
//!
//! 实现中断处理器的注册、查找和管理功能

use crate::trap::ds::{TrapType, TrapContext, TrapHandler, HandlerEntry, TrapHandlerResult, TrapError};
use crate::println;

// 每种中断类型的最大处理器数量
const MAX_HANDLERS_PER_TYPE: usize = 8;

/// 表示中断处理器注册表插槽的状态
#[derive(Copy, Clone)]
enum HandlerSlot {
    /// 空闲插槽
    Empty,
    /// 已注册处理器
    Occupied(HandlerEntry),
}

impl HandlerSlot {
    /// 创建一个空的插槽
    const fn empty() -> Self {
        HandlerSlot::Empty
    }
    
    /// 检查插槽是否为空
    fn is_empty(&self) -> bool {
        match self {
            HandlerSlot::Empty => true,
            _ => false,
        }
    }
    
    /// 获取处理器入口，如果插槽非空
    fn get_entry(&self) -> Option<HandlerEntry> {
        match self {
            HandlerSlot::Empty => None,
            HandlerSlot::Occupied(entry) => Some(*entry),
        }
    }
}

/// 中断处理器注册表
pub struct HandlerRegistry {
    /// 每种中断类型的处理器数组
    slots: [[HandlerSlot; MAX_HANDLERS_PER_TYPE]; TrapType::COUNT],
}

// 全局静态注册表
static mut REGISTRY: HandlerRegistry = HandlerRegistry::new();

impl HandlerRegistry {
    /// 创建新的处理器注册表
    const fn new() -> Self {
        // 使用空插槽填充数组
        const EMPTY_SLOT: HandlerSlot = HandlerSlot::empty();
        const EMPTY_ARRAY: [HandlerSlot; MAX_HANDLERS_PER_TYPE] = [EMPTY_SLOT; MAX_HANDLERS_PER_TYPE];
        
        Self {
            slots: [EMPTY_ARRAY; TrapType::COUNT],
        }
    }
    
    /// 注册处理器
    pub fn register(&mut self, trap_type: TrapType, handler: TrapHandler, priority: u8, description: &'static str) -> bool {
        let type_index = trap_type as usize;
        
        // 查找可用插槽和正确的插入位置
        let mut insert_index = MAX_HANDLERS_PER_TYPE;
        let mut occupied_count = 0;
        
        for i in 0..MAX_HANDLERS_PER_TYPE {
            if self.slots[type_index][i].is_empty() {
                // 找到第一个空插槽
                if insert_index == MAX_HANDLERS_PER_TYPE {
                    insert_index = i;
                }
            } else {
                occupied_count += 1;
                
                // 检查优先级，找到合适的插入位置
                if let Some(entry) = self.slots[type_index][i].get_entry() {
                    if entry.priority > priority && i < insert_index {
                        insert_index = i;
                    }
                }
            }
        }
        
        if insert_index == MAX_HANDLERS_PER_TYPE {
            // 没有可用插槽
            println!("Cannot register handler: registry full for {:?}", trap_type);
            return false;
        }
        
        // 如果需要腾出插入位置，向后移动其他处理器
        if !self.slots[type_index][insert_index].is_empty() {
            // 确保有足够的空间
            if occupied_count >= MAX_HANDLERS_PER_TYPE {
                println!("Cannot register handler: registry full for {:?}", trap_type);
                return false;
            }
            
            // 向后移动插槽
            for i in (insert_index..MAX_HANDLERS_PER_TYPE-1).rev() {
                self.slots[type_index][i + 1] = self.slots[type_index][i];
            }
        }
        
        // 插入新处理器
        let entry = HandlerEntry::new(handler, priority, description);
        self.slots[type_index][insert_index] = HandlerSlot::Occupied(entry);
        
        println!("Registered trap handler: {} for {:?} with priority {}", description, trap_type, priority);
        true
    }
    
    /// 注销处理器
    pub fn unregister(&mut self, trap_type: TrapType, description: &'static str) -> bool {
        let type_index = trap_type as usize;
        
        // 查找匹配的处理器
        for i in 0..MAX_HANDLERS_PER_TYPE {
            if let Some(entry) = self.slots[type_index][i].get_entry() {
                if entry.description == description {
                    // 找到匹配的处理器
                    
                    // 向前移动后面的处理器
                    for j in i..MAX_HANDLERS_PER_TYPE-1 {
                        self.slots[type_index][j] = self.slots[type_index][j + 1];
                    }
                    
                    // 清空最后一个插槽
                    self.slots[type_index][MAX_HANDLERS_PER_TYPE - 1] = HandlerSlot::Empty;
                    
                    println!("Unregistered trap handler: {} for {:?}", description, trap_type);
                    return true;
                }
            }
        }
        
        false
    }
    
    /// 分发中断到已注册的处理器
    pub fn dispatch(&self, trap_type: TrapType, ctx: &mut TrapContext) -> TrapHandlerResult {
        let type_index = trap_type as usize;
        
        // 按优先级依次尝试处理器
        for i in 0..MAX_HANDLERS_PER_TYPE {
            if let Some(entry) = self.slots[type_index][i].get_entry() {
                match (entry.handler)(ctx) {
                    TrapHandlerResult::Handled => {
                        // 已处理，直接返回
                        return TrapHandlerResult::Handled;
                    }
                    TrapHandlerResult::Pass => {
                        // 传递给下一个处理器
                        continue;
                    }
                    TrapHandlerResult::Failed(err) => {
                        // 处理失败，记录日志
                        println!("Handler '{}' failed with error: {:?}", entry.description, err);
                        // 继续尝试下一个处理器
                        continue;
                    }
                }
            } else {
                // 遇到空插槽，表示没有更多处理器
                break;
            }
        }
        
        // 所有处理器都无法处理或没有处理器
        TrapHandlerResult::Failed(TrapError::NoHandler)
    }
    
    /// 获取特定中断类型的处理器数量
    pub fn handler_count(&self, trap_type: TrapType) -> usize {
        let type_index = trap_type as usize;
        let mut count = 0;
        
        for i in 0..MAX_HANDLERS_PER_TYPE {
            if !self.slots[type_index][i].is_empty() {
                count += 1;
            } else {
                break;
            }
        }
        
        count
    }
    
    /// 打印所有注册的处理器信息（用于调试）
    pub fn print_handlers(&self) {
        println!("=== Registered Trap Handlers ===");
        
        for i in 0..TrapType::COUNT {
            let trap_type = TrapType::from_index(i);
            let mut handlers_found = false;
            
            for j in 0..MAX_HANDLERS_PER_TYPE {
                if let Some(entry) = self.slots[i][j].get_entry() {
                    if !handlers_found {
                        println!("{:?} 处理器:", trap_type);
                        handlers_found = true;
                    }
                    println!("  {}. {} (优先级: {})", j + 1, entry.description, entry.priority);
                } else if handlers_found {
                    // 遇到空插槽且已找到处理器，表示没有更多处理器
                    break;
                }
            }
        }
        
        println!("===============================");
    }
}

// 公共API函数

/// 注册中断处理器
pub fn register_handler(trap_type: TrapType, handler: TrapHandler, priority: u8, description: &'static str) -> bool {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let result = unsafe {
        REGISTRY.register(trap_type, handler, priority, description)
    };
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    result
}

/// 注销中断处理器
pub fn unregister_handler(trap_type: TrapType, description: &'static str) -> bool {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let result = unsafe {
        REGISTRY.unregister(trap_type, description)
    };
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    result
}

/// 分发中断到已注册的处理器
pub fn dispatch_trap(trap_type: TrapType, ctx: &mut TrapContext) -> TrapHandlerResult {
    // 注意：这个函数可能在已禁用中断的情况下调用，所以不要在此处禁用中断
    unsafe {
        REGISTRY.dispatch(trap_type, ctx)
    }
}

/// 获取特定中断类型的处理器数量
pub fn handler_count(trap_type: TrapType) -> usize {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let count = unsafe {
        REGISTRY.handler_count(trap_type)
    };
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    count
}

/// 打印所有注册的处理器信息（用于调试）
pub fn print_handlers() {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    unsafe {
        REGISTRY.print_handlers();
    }
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
}
//! 中断处理器注册表
//!
//! 实现中断处理器的注册、查找和管理功能

use crate::trap::ds::{TrapType, TrapContext, TrapHandler, HandlerEntry, TrapHandlerResult, TrapError};
use crate::trap::ds::handler::{ProtectionLevel, RegistrarId, SYSTEM_REGISTRAR_ID};
use crate::trap::infrastructure::di::context::ContextId;
use crate::println;
use spin::Mutex; 

// 添加安全错误枚举
#[derive(Debug)]
pub enum SecurityError {
    ProtectedHandler,
    InvalidRegistrar,
    SystemRequired,
    InternalError,
}

// 每种中断类型的最大处理器数量
const MAX_HANDLERS_PER_TYPE: usize = 8;

/// 增加注册器结构，支持保护级别和所有权
#[derive(Copy, Clone)]
struct HandlerRegistration {
    entry: HandlerEntry,
    context_id: Option<ContextId>,
}

/// 表示中断处理器注册表插槽的状态
#[derive(Copy, Clone)]
enum HandlerSlot {
    /// 空闲插槽
    Empty,
    /// 已注册处理器
    Occupied(HandlerRegistration),
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
            HandlerSlot::Occupied(reg) => Some(reg.entry),
        }
    }
    
    /// 获取完整的注册信息
    fn get_registration(&self) -> Option<HandlerRegistration> {
        match self {
            HandlerSlot::Empty => None,
            HandlerSlot::Occupied(reg) => Some(*reg),
        }
    }
}

/// 中断处理器注册表
pub struct HandlerRegistry {
    /// 每种中断类型的处理器数组
    slots: [[HandlerSlot; MAX_HANDLERS_PER_TYPE]; TrapType::COUNT],
}

// 全局静态注册表
static REGISTRY: Mutex<HandlerRegistry> = Mutex::new(HandlerRegistry::new());

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
        
        // 创建简单的处理器条目(兼容原代码)
        let entry = HandlerEntry::new(handler, priority, description);
        
        // 创建注册信息，不设置上下文ID
        let registration = HandlerRegistration {
            entry,
            context_id: None,
        };
        
        // 插入新处理器
        self.slots[type_index][insert_index] = HandlerSlot::Occupied(registration);
        
        println!("Registered trap handler: {} for {:?} with priority {}", description, trap_type, priority);
        true
    }
    
    /// 安全版注册内部方法
    fn register_internal(&mut self, trap_type: TrapType, registration: HandlerRegistration) -> bool {
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
                if let Some(reg) = self.slots[type_index][i].get_registration() {
                    if reg.entry.priority > registration.entry.priority && i < insert_index {
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
        self.slots[type_index][insert_index] = HandlerSlot::Occupied(registration);
        
        println!("Registered trap handler: {} for {:?} with priority {}, protection: {:?}, registrar: {}",
                 registration.entry.description, trap_type, registration.entry.priority,
                 registration.entry.protection_level, registration.entry.registrar_id);
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
        
        println!("Cannot unregister handler: description '{}' not found for trap type {:?}",
               description, trap_type);
        false
    }
    
    /// 安全版注销方法，验证所有权
    fn unregister_secure(
        &mut self,
        trap_type: TrapType,
        description: &'static str,
        registrar_id: RegistrarId
    ) -> Result<bool, SecurityError> {
        let type_index = trap_type as usize;
        
        // 查找匹配的处理器
        for i in 0..MAX_HANDLERS_PER_TYPE {
            if let Some(reg) = self.slots[type_index][i].get_registration() {
                if reg.entry.description == description {
                    // 找到匹配的处理器，检查权限
                    
                    // 系统级处理器只能由系统注销
                    if reg.entry.is_system() && registrar_id != SYSTEM_REGISTRAR_ID {
                        println!("Cannot unregister system handler: {} by non-system registrar: {}",
                                 description, registrar_id);
                        return Err(SecurityError::ProtectedHandler);
                    }
                    
                    // 用户级处理器需要匹配注册者ID
                    if !reg.entry.is_system() && reg.entry.registrar_id != registrar_id {
                        println!("Cannot unregister handler: {} - registrar mismatch: expected {}, got {}",
                                 description, reg.entry.registrar_id, registrar_id);
                        return Err(SecurityError::InvalidRegistrar);
                    }
                    
                    // 权限验证通过，可以注销
                    
                    // 向前移动后面的处理器
                    for j in i..MAX_HANDLERS_PER_TYPE-1 {
                        self.slots[type_index][j] = self.slots[type_index][j + 1];
                    }
                    
                    // 清空最后一个插槽
                    self.slots[type_index][MAX_HANDLERS_PER_TYPE - 1] = HandlerSlot::Empty;
                    
                    println!("Unregistered trap handler: {} for {:?} (owner: {})",
                             description, trap_type, registrar_id);
                    return Ok(true);
                }
            }
        }
        
        // 没有找到匹配的处理器
        println!("Cannot unregister handler: description '{}' not found for trap type {:?}",
                 description, trap_type);
        Ok(false)
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
    
    /// 安全版上下文关联处理器注销，验证所有权(无堆实现)
    fn unregister_context_secure(&mut self, context_id: ContextId, registrar_id: RegistrarId) -> usize {
        let mut total_count = 0;
        
        // 遍历所有trap类型
        for type_index in 0..TrapType::COUNT {
            // 使用固定大小数组存储待删除的索引
            let mut removed_indices = [0; MAX_HANDLERS_PER_TYPE];
            let mut removed_count = 0;
            
            // 先找出需要删除的处理器
            for i in 0..MAX_HANDLERS_PER_TYPE {
                if let Some(reg) = self.slots[type_index][i].get_registration() {
                    if let Some(ctx_id) = reg.context_id {
                        if ctx_id == context_id {
                            // 检查权限
                            let can_remove = if reg.entry.is_system() {
                                // 系统级处理器需要系统ID
                                registrar_id == SYSTEM_REGISTRAR_ID
                            } else {
                                // 用户级处理器需要匹配注册者ID
                                reg.entry.registrar_id == registrar_id
                            };
                            
                            if can_remove && removed_count < MAX_HANDLERS_PER_TYPE {
                                removed_indices[removed_count] = i;
                                removed_count += 1;
                            }
                        }
                    }
                }
            }
            
            // 从后向前注销，避免索引移位问题
            // 注意这里采用固定大小数组
            for i in (0..removed_count).rev() {
                let idx = removed_indices[i];
                
                // 暂存处理器描述用于日志
                let desc = if let Some(reg) = self.slots[type_index][idx].get_registration() {
                    reg.entry.description
                } else {
                    "unknown"
                };
                
                // 向前移动后面的处理器
                for j in idx..MAX_HANDLERS_PER_TYPE-1 {
                    self.slots[type_index][j] = self.slots[type_index][j + 1];
                }
                
                // 清空最后一个插槽
                self.slots[type_index][MAX_HANDLERS_PER_TYPE - 1] = HandlerSlot::Empty;
                
                println!("Unregistered handler for context {}: {} (type index: {})",
                         context_id, desc, type_index);
                
                total_count += 1;
            }
        }
        
        println!("Unregistered {} handlers for context {}", total_count, context_id);
        total_count
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
                        println!("{:?} Handlers:", trap_type);
                        handlers_found = true;
                    }
                    
                    // 获取保护级别字符串
                    let protection_str = if entry.is_system() {
                        "System"
                    } else {
                        "User"
                    };
                    
                    // 单独打印，避免使用format!和String::new()
                    println!("  {}. {} (Priority: {}, Protection: {})",
                             j + 1, entry.description, entry.priority, protection_str);
                    
                    // 注册者ID单独打印
                    if let Some(reg) = self.slots[i][j].get_registration() {
                        println!("     Registrar: {}", reg.entry.registrar_id);
                    }
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
    
    let mut guard = REGISTRY.lock();
    let result = guard.register(trap_type, handler, priority, description);
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    result
}

/// 安全版注册处理器函数
pub fn register_handler_with_owner(
    trap_type: TrapType,
    handler: TrapHandler,
    priority: u8,
    description: &'static str,
    protection_level: ProtectionLevel,
    registrar_id: RegistrarId,
    context_id: Option<ContextId>
) -> bool {
    println!("Registering handler: {} for {:?} with priority {}, protection: {:?}, registrar: {}",
             description, trap_type, priority, protection_level, registrar_id);
    
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let mut guard = REGISTRY.lock();
    
    // 创建Handler条目
    let entry = HandlerEntry::new_with_protection(
        handler, 
        priority, 
        description, 
        protection_level, 
        registrar_id
    );
    
    // 创建注册信息
    let registration = HandlerRegistration {
        entry,
        context_id,
    };
    
    // 调用内部注册方法
    let result = guard.register_internal(trap_type, registration);
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    result
}

/// 注销中断处理器
pub fn unregister_handler(trap_type: TrapType, description: &'static str) -> bool {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let mut guard = REGISTRY.lock();
    let result = guard.unregister(trap_type, description);
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    result
}

/// 安全版注销处理器函数
pub fn unregister_handler_secure(
    trap_type: TrapType,
    description: &'static str,
    registrar_id: RegistrarId
) -> Result<bool, SecurityError> {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let mut guard = REGISTRY.lock();
    
    // 查找处理器并验证权限
    let result = guard.unregister_secure(trap_type, description, registrar_id);
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    result
}

/// 分发中断到已注册的处理器
pub fn dispatch_trap(trap_type: TrapType, ctx: &mut TrapContext) -> TrapHandlerResult {
    // 注意：这个函数可能在已禁用中断的情况下调用
    // 在中断上下文中使用锁时需特别小心
    let guard = REGISTRY.lock();
    guard.dispatch(trap_type, ctx)
}

/// 获取特定中断类型的处理器数量
pub fn handler_count(trap_type: TrapType) -> usize {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let guard = REGISTRY.lock();
    let count = guard.handler_count(trap_type);
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    count
}

/// 安全版上下文关联处理器注销函数
pub fn unregister_handlers_for_context_secure(
    context_id: ContextId,
    registrar_id: RegistrarId
) -> usize {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let mut guard = REGISTRY.lock();
    let count = guard.unregister_context_secure(context_id, registrar_id);
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
    
    count
}

/// 打印所有注册的处理器信息（用于调试）
pub fn print_handlers() {
    // 禁用中断以确保安全访问注册表
    let was_enabled = crate::trap::infrastructure::disable_interrupts();
    
    let guard = REGISTRY.lock();
    guard.print_handlers();
    
    // 恢复中断状态
    crate::trap::infrastructure::restore_interrupts(was_enabled);
}
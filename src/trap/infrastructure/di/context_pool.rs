//! 上下文对象池管理器
//!
//! 提供上下文对象的创建、存储和销毁功能，
//! 确保在上下文生命周期结束时正确触发Drop处理

use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use spin::Mutex;
use crate::println;
use super::context::{ContextId, generate_context_id};
use crate::trap::ds::TrapType;
use crate::trap::ds::TrapContext;
use crate::trap::ds::TrapHandlerResult;

/// 上下文对象池错误类型
#[derive(Debug, Clone, Copy)]
pub enum PoolError {
    /// 池已满
    PoolFull,
    /// 无效上下文ID
    InvalidContextId,
    /// 上下文已存在
    ContextExists,
    /// 上下文不存在
    ContextNotFound,
    /// 无效的访问令牌
    InvalidToken,
    /// 上下文已被销毁
    ContextDestroyed,
    /// 访问被拒绝
    AccessDenied,
    /// 锁已被占用（死锁风险）
    LockBusy,
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolError::PoolFull => write!(f, "Context pool is full"),
            PoolError::InvalidContextId => write!(f, "Invalid context ID"),
            PoolError::ContextExists => write!(f, "Context already exists"),
            PoolError::ContextNotFound => write!(f, "Context not found"),
            PoolError::InvalidToken => write!(f, "Invalid access token"),
            PoolError::ContextDestroyed => write!(f, "Context has been destroyed"),
            PoolError::AccessDenied => write!(f, "Access denied"),
            PoolError::LockBusy => write!(f, "Lock is busy"),
        }
    }
}

/// 上下文对象特性，必须实现以支持上下文管理
pub trait ContextObject: Sized {
    /// 获取上下文ID
    fn id(&self) -> ContextId;
    
    /// 创建新的上下文对象
    fn new(id: ContextId) -> Self;
}

/// 上下文对象池大小
const CONTEXT_POOL_SIZE: usize = 64;

/// 上下文池槽位状态
struct PoolSlot<T: ContextObject> {
    /// 对象实例
    object: Option<T>,
    /// 是否使用中 - 不使用原子类型
    in_use: bool,
    /// 访问令牌
    token: u32,
    /// 对象版本号 - 防止访问已删除重新分配的对象
    version: usize,
}

impl<T: ContextObject> PoolSlot<T> {
    /// 创建新的空槽位
    const fn new() -> Self {
        Self {
            object: None,
            in_use: false,
            token: 0,
            version: 0,
        }
    }
    
    /// 设置对象
    fn set(&mut self, obj: T) -> u32 {
        // 生成新的访问令牌
        let token = rand_token();

        // 更新状态
        self.token = token;
        self.in_use = true;
        self.version += 1;

        // 存储对象
        self.object = Some(obj);

        token
    }
    
    /// 清除对象，返回被删除的对象
    fn clear(&mut self) -> Option<T> {
        // 标记为未使用
        self.in_use = false;

        // 无效化令牌
        self.token = 0;

        // 取出并返回对象
        self.object.take()
    }
    
    /// 验证访问令牌是否有效
    fn validate_token(&self, token: u32) -> bool {
        self.in_use && self.token == token
    }
}

/// 产生随机令牌
fn rand_token() -> u32 {
    // 在no_std环境中使用一个简单的计数器
    static mut TOKEN_COUNTER: u32 = 1;

    // 安全地生成一个唯一令牌
    unsafe {
        let token = TOKEN_COUNTER;
        // 确保令牌不为0（0表示无效令牌）
        if token == 0 {
            TOKEN_COUNTER = 2;
            1
        } else {
            TOKEN_COUNTER = token.wrapping_add(1);
            token
        }
    }
}

/// 上下文对象池管理器
pub struct ContextPool<T: ContextObject> {
    /// 对象槽位数组
    slots: [PoolSlot<T>; CONTEXT_POOL_SIZE],
    /// 已用对象计数
    count: usize,
    /// 用于查找对象的映射表 - 不使用原子类型
    id_to_index: [(ContextId, bool); CONTEXT_POOL_SIZE],
}

impl<T: ContextObject> ContextPool<T> {
    /// 创建新的上下文池
    pub const fn new() -> Self {
        // 创建空槽位数组
        let slots = [
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
            PoolSlot::new(), PoolSlot::new(), PoolSlot::new(), PoolSlot::new(),
        ];

        // 创建ID映射表
        let id_to_index = [
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
            (0, false), (0, false), (0, false), (0, false),
        ];

        Self {
            slots,
            count: 0,
            id_to_index,
        }
    }

    /// 创建并存储新的上下文对象
    pub fn create_context(&mut self, id: ContextId) -> Result<(ContextId, u32, usize), PoolError> {
        if self.count >= CONTEXT_POOL_SIZE {
            return Err(PoolError::PoolFull);
        }

        // 检查ID是否已存在
        if self.find_index_by_id(id).is_some() {
            return Err(PoolError::ContextExists);
        }

        // 查找空闲槽位
        let mut idx = CONTEXT_POOL_SIZE;
        for i in 0..CONTEXT_POOL_SIZE {
            if !self.slots[i].in_use {
                idx = i;
                break;
            }
        }

        if idx == CONTEXT_POOL_SIZE {
            return Err(PoolError::PoolFull);
        }

        // 创建并存储对象
        let context = T::new(id);
        let token = self.slots[idx].set(context);
        let version = self.slots[idx].version;

        // 更新映射表
        self.id_to_index[idx] = (id, true);

        // 更新计数
        self.count += 1;

        println!("Created context with ID {} at index {}, token: {}, version: {}",
                 id, idx, token, version);
        Ok((id, token, version))
    }

    /// 销毁上下文对象
    pub fn destroy_context(&mut self, id: ContextId) -> Result<(), PoolError> {
        // 查找匹配ID的对象
        let idx = match self.find_index_by_id(id) {
            Some(i) => i,
            None => return Err(PoolError::ContextNotFound),
        };

        // 关键：取出对象，触发Drop
        if let Some(obj_to_drop) = self.slots[idx].clear() {
            // 对象离开作用域时会自动调用Drop
            println!("Destroying context with ID {} from index {}", id, idx);

            // 更新映射表
            self.id_to_index[idx].1 = false;

            // 更新计数
            self.count -= 1;

            Ok(())
        } else {
            // 这种情况不应该发生
            println!("Warning: Slot marked as in-use but no object found at index {}", idx);

            // 更新状态以恢复一致性
            self.slots[idx].in_use = false;
            self.id_to_index[idx].1 = false;

            Err(PoolError::ContextNotFound)
        }
    }

    /// 查找ID对应的索引
    fn find_index_by_id(&self, id: ContextId) -> Option<usize> {
        for i in 0..CONTEXT_POOL_SIZE {
            if self.id_to_index[i].0 == id && self.id_to_index[i].1 {
                if self.slots[i].in_use {
                    return Some(i);
                }
            }
        }
        None
    }

    /// 验证访问令牌和版本
    pub fn validate_access(&self, id: ContextId, token: u32, version: usize) -> Result<usize, PoolError> {
        // 查找ID对应的索引
        let idx = match self.find_index_by_id(id) {
            Some(i) => i,
            None => return Err(PoolError::ContextNotFound),
        };

        // 验证令牌
        if !self.slots[idx].validate_token(token) {
            return Err(PoolError::InvalidToken);
        }

        // 验证版本号
        if self.slots[idx].version != version {
            return Err(PoolError::ContextDestroyed);
        }

        Ok(idx)
    }

    /// 安全地访问对象，传入一个回调函数
    pub fn with_object<F, R>(&self, id: ContextId, token: u32, version: usize, f: F) -> Result<R, PoolError>
    where
        F: FnOnce(&T) -> R,
    {
        // 验证访问
        let idx = self.validate_access(id, token, version)?;

        // 访问对象
        if let Some(obj) = &self.slots[idx].object {
            Ok(f(obj))
        } else {
            Err(PoolError::ContextNotFound)
        }
    }

    /// 安全地修改对象，传入一个回调函数
    pub fn with_object_mut<F, R>(&mut self, id: ContextId, token: u32, version: usize, f: F) -> Result<R, PoolError>
    where
        F: FnOnce(&mut T) -> R,
    {
        // 验证访问
        let idx = self.validate_access(id, token, version)?;

        // 修改对象
        if let Some(obj) = &mut self.slots[idx].object {
            Ok(f(obj))
        } else {
            Err(PoolError::ContextNotFound)
        }
    }

    /// 获取已用对象数量
    pub fn count(&self) -> usize {
        self.count
    }

    /// 清除所有对象（用于测试和重置）
    #[cfg(test)]
    pub fn clear_all(&mut self) {
        for i in 0..CONTEXT_POOL_SIZE {
            if self.slots[i].in_use {
                let _ = self.slots[i].clear();
                self.id_to_index[i].1 = false;
            }
        }
        self.count = 0;
        println!("Cleared all context objects");
    }

    /// 获取对象的引用（不安全，仅用于内部实现）
    ///
    /// # 安全性
    ///
    /// 此函数跳过了令牌验证，仅应在确保上下文安全的情况下使用
    pub(crate) fn get_object_unchecked(&self, id: ContextId) -> Option<&T> {
        if let Some(idx) = self.find_index_by_id(id) {
            if let Some(obj) = &self.slots[idx].object {
                return Some(obj);
            }
        }
        None
    }

    /// 迭代所有有效对象，执行指定的回调函数
    ///
    /// 此方法用于需要操作所有对象的场景，如全局清理
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(ContextId, &T),
    {
        for i in 0..CONTEXT_POOL_SIZE {
            if self.slots[i].in_use {
                if let Some(obj) = &self.slots[i].object {
                    f(obj.id(), obj);
                }
            }
        }
    }

    /// 迭代所有有效对象，可能修改它们，执行指定的回调函数
    pub fn for_each_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(ContextId, &mut T),
    {
        for i in 0..CONTEXT_POOL_SIZE {
            if self.slots[i].in_use {
                if let Some(obj) = &mut self.slots[i].object {
                    f(obj.id(), obj);
                }
            }
        }
    }
}

/// 进程控制块示例
pub struct ProcessControlBlock {
    /// 进程ID，也作为ContextId
    pub pid: ContextId,
    /// 进程名称
    pub name: &'static str,
    /// 状态标志
    pub state: u8,
}

impl ContextObject for ProcessControlBlock {
    fn id(&self) -> ContextId {
        self.pid
    }
    
    fn new(id: ContextId) -> Self {
        Self {
            pid: id,
            name: "unnamed",
            state: 0,
        }
    }
}

impl Drop for ProcessControlBlock {
    fn drop(&mut self) {
        // 打印日志
        println!("Process {}: Dropping. Triggering handler cleanup.", self.pid);
        
        // 调用handler清理函数
        let removed_count = super::unregister_handlers_for_context(self.pid);
        
        println!("Process {}: Cleaned up {} handlers.", self.pid, removed_count);
    }
}

/// 进程句柄，用于安全地提供对进程的访问
pub struct ProcessHandle {
    /// 进程ID
    pub pid: ContextId,
    /// 进程内部访问令牌
    token: u32,
    /// 进程版本号，用于检测对象是否被重新分配
    version: usize,
    /// 句柄是否有效标志 - 使用普通bool而非AtomicBool
    valid: bool,
}

impl ProcessHandle {
    /// 创建新的进程句柄
    fn new(pid: ContextId, token: u32, version: usize) -> Self {
        Self {
            pid,
            token,
            version,
            valid: true,
        }
    }

    /// 检查句柄是否有效
    fn check_valid(&self) -> Result<(), PoolError> {
        if !self.valid {
            return Err(PoolError::InvalidToken);
        }
        Ok(())
    }
    
    /// 获取进程状态
    pub fn get_state(&self) -> Result<u8, PoolError> {
        self.check_valid()?;
        
        // 获取池锁
        let pool_guard = PROCESS_POOL.try_lock();
        let pool = match pool_guard {
            Some(guard) => guard,
            None => return Err(PoolError::LockBusy),
        };
        
        // 安全访问
        pool.with_object(self.pid, self.token, self.version, |process| {
            process.state
        })
    }
    
    /// 设置进程状态
    pub fn set_state(&self, new_state: u8) -> Result<(), PoolError> {
        self.check_valid()?;
        
        // 获取池锁
        let mut pool_guard = PROCESS_POOL.try_lock();
        let pool = match pool_guard.as_mut() {
            Some(guard) => guard,
            None => return Err(PoolError::LockBusy),
        };
        
        // 安全修改
        pool.with_object_mut(self.pid, self.token, self.version, |process| {
            process.state = new_state;
        })
    }
    
    /// 获取进程名称
    pub fn get_name(&self) -> Result<&'static str, PoolError> {
        self.check_valid()?;
        
        // 获取池锁
        let pool_guard = PROCESS_POOL.try_lock();
        let pool = match pool_guard {
            Some(guard) => guard,
            None => return Err(PoolError::LockBusy),
        };
        
        // 安全访问
        pool.with_object(self.pid, self.token, self.version, |process| {
            process.name
        })
    }
    
    /// 设置进程名称
    pub fn set_name(&self, new_name: &'static str) -> Result<(), PoolError> {
        self.check_valid()?;
        
        // 获取池锁
        let mut pool_guard = PROCESS_POOL.try_lock();
        let pool = match pool_guard.as_mut() {
            Some(guard) => guard,
            None => return Err(PoolError::LockBusy),
        };
        
        // 安全修改
        pool.with_object_mut(self.pid, self.token, self.version, |process| {
            process.name = new_name;
        })
    }
    
    /// 为该进程注册中断处理器
    pub fn register_handler(
        &self,
        trap_type: TrapType,
        handler_fn: fn(&mut TrapContext) -> TrapHandlerResult,
        priority: u8,
        description: &'static str
    ) -> Result<bool, PoolError> {
        self.check_valid()?;
        
        // 注册处理器
        let result = super::register_handler(
            trap_type,
            handler_fn,
            priority,
            description,
            Some(self.pid)
        );
        
        Ok(result)
    }
    
    /// 使句柄无效
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // 使句柄无效，防止进一步使用
        self.valid = false;
    }
}

// 全局进程池实例
static PROCESS_POOL: Mutex<ContextPool<ProcessControlBlock>> = Mutex::new(ContextPool::new());

/// 创建新进程
pub fn create_process(pid: Option<ContextId>) -> Result<ProcessHandle, PoolError> {
    // 如果未提供PID，则生成一个
    let real_pid = pid.unwrap_or_else(generate_context_id);
    
    // 获取池锁
    let mut pool_guard = PROCESS_POOL.try_lock();
    let pool = match pool_guard.as_mut() {
        Some(guard) => guard,
        None => return Err(PoolError::LockBusy),
    };
    
    // 创建进程
    match pool.create_context(real_pid) {
        Ok((id, token, version)) => Ok(ProcessHandle::new(id, token, version)),
        Err(e) => Err(e),
    }
}

/// 销毁进程
pub fn destroy_process(pid: ContextId) -> Result<(), PoolError> {
    // 获取池锁
    let mut pool_guard = PROCESS_POOL.try_lock();
    let pool = match pool_guard.as_mut() {
        Some(guard) => guard,
        None => return Err(PoolError::LockBusy),
    };
    
    pool.destroy_context(pid)
}
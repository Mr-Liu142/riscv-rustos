//! SBI 扩展功能模块
//!
//! 本模块对基础SBI接口进行了更高级别的封装，提供了更易用的操作系统功能接口。
//! 包括控制台IO、系统管理、时钟管理、多核通信等功能。

use core::fmt::Write;
use sbi_rt::HartMask;
use super::api;

/// 系统管理相关功能
pub mod system {
    use super::api;
    
    /// 系统关机原因枚举
    #[derive(Debug, Clone, Copy)]
    pub enum ShutdownReason {
        /// 正常关机
        Normal,
        /// 系统故障
        SystemFailure,
        /// 用户请求
        UserRequest,
    }
    
    /// 安全关机函数
    ///
    /// 进行必要的清理工作，然后关闭系统
    /// # 参数
    ///
    /// * `reason` - 关机原因
    pub fn shutdown(reason: ShutdownReason) -> ! {
        // 这里可以添加一些关机前的清理工作
        
        // 输出关机信息
        match reason {
            ShutdownReason::Normal => crate::println!("System normal shutdown"),
            ShutdownReason::SystemFailure => crate::println!("System failure, forced shutdown"),
            ShutdownReason::UserRequest => crate::println!("User requested shutdown"),
        }
        
        // 调用SBI关机接口
        api::shutdown();
    }
    
    /// 系统重启类型枚举
    #[derive(Debug, Clone, Copy)]
    pub enum RebootType {
        /// 冷重启 - 完全重置系统
        Cold,
        /// 热重启 - 快速重启，不完全重置硬件
        Warm,
    }
    
    /// 系统重启函数
    ///
    /// # 参数
    ///
    /// * `reboot_type` - 重启类型
    pub fn reboot(reboot_type: RebootType) -> ! {
        match reboot_type {
            RebootType::Cold => crate::println!("System cold reboot..."),
            RebootType::Warm => crate::println!("System warm reboot..."),
        }
        
        // 目前SBI只支持冷重启，这里做一个封装以便未来扩展
        api::reboot();
    }
    
    /// 获取系统信息
    pub fn get_system_info() -> SystemInfo {
        let (major, minor) = api::get_spec_version();
        
        SystemInfo {
            sbi_spec_version_major: major,
            sbi_spec_version_minor: minor,
            sbi_impl_id: api::get_impl_id(),
            sbi_impl_version: api::get_impl_version(),
            mvendorid: api::get_mvendorid(),
            marchid: api::get_marchid(),
            mimpid: api::get_mimpid(),
        }
    }
    
    /// 系统信息结构体
    #[derive(Debug, Clone)]
    pub struct SystemInfo {
        /// SBI规范主版本号
        pub sbi_spec_version_major: usize,
        /// SBI规范次版本号
        pub sbi_spec_version_minor: usize,
        /// SBI实现ID
        pub sbi_impl_id: usize,
        /// SBI实现版本
        pub sbi_impl_version: usize,
        /// 机器制造商ID
        pub mvendorid: usize,
        /// 机器架构ID
        pub marchid: usize,
        /// 机器实现ID
        pub mimpid: usize,
    }
    
    impl SystemInfo {
        /// 打印系统信息
        pub fn print(&self) {
            crate::println!("==== System Information ====");
            crate::println!("SBI Spec Version: {}.{}", self.sbi_spec_version_major, self.sbi_spec_version_minor);
            crate::println!("SBI Implementation ID: {}", self.sbi_impl_id);
            crate::println!("SBI Implementation Version: {}", self.sbi_impl_version);
            crate::println!("Machine Vendor ID: 0x{:x}", self.mvendorid);
            crate::println!("Machine Architecture ID: 0x{:x}", self.marchid);
            crate::println!("Machine Implementation ID: 0x{:x}", self.mimpid);
            crate::println!("============================");
        }
    }
}

/// 控制台输入输出相关功能
pub mod console {
    use super::api;
    use core::fmt;
    
    /// 控制台输出缓冲区大小
    const CONSOLE_BUFFER_SIZE: usize = 128;
    
    /// 控制台输出缓冲区
    struct ConsoleBuffer {
        buffer: [u8; CONSOLE_BUFFER_SIZE],
        len: usize,
    }
    
    impl ConsoleBuffer {
        /// 创建新的控制台缓冲区
        const fn new() -> Self {
            Self {
                buffer: [0; CONSOLE_BUFFER_SIZE],
                len: 0,
            }
        }
        
        /// 清空缓冲区
        fn clear(&mut self) {
            self.len = 0;
        }
        
        /// 将缓冲区内容写入控制台
        fn flush(&mut self) {
            for i in 0..self.len {
                api::console_putchar(self.buffer[i] as char);
            }
            self.clear();
        }
        
        /// 向缓冲区添加一个字节
        fn push(&mut self, byte: u8) {
            if self.len >= CONSOLE_BUFFER_SIZE {
                self.flush();
            }
            self.buffer[self.len] = byte;
            self.len += 1;
        }
    }
    
    /// 缓冲式控制台输出器
    pub struct BufferedConsole {
        buffer: ConsoleBuffer,
    }
    
    impl BufferedConsole {
        /// 创建新的缓冲式控制台
        pub const fn new() -> Self {
            Self {
                buffer: ConsoleBuffer::new(),
            }
        }
        
        /// 刷新缓冲区，将内容输出到控制台
        pub fn flush(&mut self) {
            self.buffer.flush();
        }
    }
    
    impl fmt::Write for BufferedConsole {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            for byte in s.bytes() {
                self.buffer.push(byte);
            }
            Ok(())
        }
    }
    
    /// 静态全局缓冲式控制台
    static mut BUFFERED_CONSOLE: BufferedConsole = BufferedConsole::new();
    
    /// 打印格式化字符串到控制台
    ///
    /// 使用缓冲区提高输出效率
    pub fn print(args: fmt::Arguments) {
        use core::fmt::Write;
        unsafe {
            BUFFERED_CONSOLE.write_fmt(args).unwrap();
            BUFFERED_CONSOLE.flush();
        }
    }
    
    /// 等待并获取一个字符
    ///
    /// 如果没有输入，将阻塞直到有输入
    pub fn getchar() -> char {
        loop {
            if let Some(c) = api::console_getchar() {
                return c;
            }
        }
    }
    
    /// 无阻塞获取一个字符
    ///
    /// 如果没有输入，返回None
    pub fn try_getchar() -> Option<char> {
        api::console_getchar()
    }
    
    /// 读取一行输入
    ///
    /// # 参数
    ///
    /// * `buffer` - 存储读取内容的缓冲区
    /// * `echo` - 是否回显输入的字符
    ///
    /// # 返回值
    ///
    /// 实际读取的字符数
    pub fn getline(buffer: &mut [u8], echo: bool) -> usize {
        let mut count = 0;
        
        while count < buffer.len() - 1 {
            let c = getchar();
            
            // 处理退格键
            if c == '\u{8}' || c == '\u{7f}' {
                if count > 0 {
                    count -= 1;
                    if echo {
                        api::console_putchar('\u{8}');  // 退格
                        api::console_putchar(' ');      // 清除字符
                        api::console_putchar('\u{8}');  // 再次退格
                    }
                }
                continue;
            }
            
            // 处理回车键
            if c == '\r' || c == '\n' {
                buffer[count] = 0;
                if echo {
                    api::console_putchar('\n');
                }
                break;
            }
            
            // 普通字符
            buffer[count] = c as u8;
            count += 1;
            
            if echo {
                api::console_putchar(c);
            }
        }
        
        count
    }
}

/// 时钟和定时器相关功能
pub mod timer {
    use super::api;
    
    /// 获取当前的时间计数器值
    /// 
    /// 这个函数需要在RISC-V的S模式下通过读取time CSR来实现
    /// 由于在Rust中不能直接访问特权级CSR，需要通过内联汇编实现
    #[inline]
    pub fn get_time() -> u64 {
        let time: u64;
        unsafe {
            core::arch::asm!(
                "rdtime {0}",
                out(reg) time,
                options(nomem, nostack)
            );
        }
        time
    }
    
    /// 设置定时器，在指定的时间后触发时钟中断
    ///
    /// # 参数
    ///
    /// * `time_value` - 绝对时间值
    pub fn set_timer(time_value: u64) {
        api::set_timer(time_value);
    }
    
    /// 设置相对定时器，在当前时间后的指定时间差触发时钟中断
    ///
    /// # 参数
    ///
    /// * `delta` - 相对当前时间的时间差
    pub fn set_timer_rel(delta: u64) {
        let current = get_time();
        set_timer(current + delta);
    }
    
    /// 睡眠指定的时钟周期
    ///
    /// 注意：此函数会阻塞线程执行，并且需要中断处理程序支持
    /// 这个函数只是一个示例，实际使用需要配合中断处理
    ///
    /// # 参数
    ///
    /// * `cycles` - 睡眠的时钟周期数
    pub fn sleep_cycles(cycles: u64) {
        let start = get_time();
        while get_time() - start < cycles {
            // 空循环等待
            core::hint::spin_loop();
        }
    }
}

/// 多核处理器通信相关功能
pub mod hart {
    use super::api;
    use sbi_rt::HartMask;
    
    /// 创建一个包含所有可用核心的HartMask
    pub fn all_harts() -> HartMask {
        // 假设系统最多支持8个核心
        const MAX_HARTS: usize = 8;
        HartMask::from_mask_base(usize::MAX, 0)
    }
    
    /// 创建一个包含单个核心的HartMask
    ///
    /// # 参数
    ///
    /// * `hart_id` - 处理器核心ID
    pub fn single_hart(hart_id: usize) -> HartMask {
        HartMask::from_mask_base(1 << hart_id, 0)
    }
    
    /// 发送处理器间中断到指定核心
    ///
    /// # 参数
    ///
    /// * `hart_id` - 目标处理器核心ID
    pub fn send_ipi_to_hart(hart_id: usize) {
        api::send_ipi(single_hart(hart_id));
    }
    
    /// 发送处理器间中断到所有核心
    pub fn send_ipi_to_all() {
        api::send_ipi(all_harts());
    }
    
    /// 在指定核心上执行远程TLB刷新
    ///
    /// # 参数
    ///
    /// * `hart_id` - 目标处理器核心ID
    pub fn fence_i_on_hart(hart_id: usize) {
        api::remote_fence_i(single_hart(hart_id));
    }
    
    /// 在所有核心上执行远程TLB刷新
    pub fn fence_i_on_all() {
        api::remote_fence_i(all_harts());
    }
    
    /// 在指定核心上执行SFENCE.VMA指令
    ///
    /// # 参数
    ///
    /// * `hart_id` - 目标处理器核心ID
    /// * `start` - 开始地址
    /// * `size` - 地址范围大小
    pub fn sfence_vma_on_hart(hart_id: usize, start: usize, size: usize) {
        api::remote_sfence_vma(single_hart(hart_id), start, size);
    }
    
    /// 在所有核心上执行SFENCE.VMA指令
    ///
    /// # 参数
    ///
    /// * `start` - 开始地址
    /// * `size` - 地址范围大小
    pub fn sfence_vma_on_all(start: usize, size: usize) {
        api::remote_sfence_vma(all_harts(), start, size);
    }
}

/// TLB（地址转换缓冲区）相关功能
pub mod tlb {
    use super::hart;
    
    /// 刷新当前核心的TLB（全部）
    pub fn flush_local() {
        unsafe {
            core::arch::asm!("sfence.vma", options(nostack));
        }
    }
    
    /// 刷新当前核心指定地址范围的TLB
    ///
    /// # 参数
    ///
    /// * `start` - 开始地址
    /// * `size` - 地址范围大小
    pub fn flush_local_range(start: usize, size: usize) {
        let end = start + size;
        // 按页(4KB)对齐进行刷新
        let page_size = 4096;
        let start_page = start & !(page_size - 1);
        let end_page = (end + page_size - 1) & !(page_size - 1);
        
        for addr in (start_page..end_page).step_by(page_size) {
            unsafe {
                core::arch::asm!(
                    "sfence.vma {0}, zero",
                    in(reg) addr,
                    options(nostack)
                );
            }
        }
    }
    
    /// 刷新所有核心的TLB（全部）
    pub fn flush_all_harts() {
        // 首先刷新本地TLB
        flush_local();
        
        // 然后通知其他核心刷新TLB
        hart::fence_i_on_all();
    }
    
    /// 刷新所有核心指定地址范围的TLB
    ///
    /// # 参数
    ///
    /// * `start` - 开始地址
    /// * `size` - 地址范围大小
    pub fn flush_range_all_harts(start: usize, size: usize) {
        // 首先刷新本地TLB范围
        flush_local_range(start, size);
        
        // 然后通知其他核心刷新指定范围TLB
        hart::sfence_vma_on_all(start, size);
    }
}
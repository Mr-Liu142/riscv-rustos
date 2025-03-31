//! SBI API接口
//!
//! 提供了对SBI调用的封装，包括控制台、时钟、系统管理等功能。
//! 基于sbi-rt库实现。

use sbi_rt::{
    self,
    legacy,
    HartMask,
    Shutdown, ColdReboot, // 具体类型，实现了ResetType
    NoReason, SystemFailure, // 具体类型，实现了ResetReason
};

/// 系统关机
pub fn shutdown() -> ! {
    sbi_rt::system_reset(Shutdown, NoReason);
    unreachable!("关机失败！");
}

/// 系统重启
pub fn reboot() -> ! {
    sbi_rt::system_reset(ColdReboot, SystemFailure);
    unreachable!("重启失败！");
}

/// 向控制台输出一个字符
pub fn console_putchar(c: char) {
    legacy::console_putchar(c as usize);
}

/// 从控制台读取一个字符
pub fn console_getchar() -> Option<char> {
    let c = legacy::console_getchar();
    if c == usize::MAX {
        None
    } else {
        Some(c as u8 as char)
    }
}

/// 设置下一次时钟中断的时间
pub fn set_timer(time: u64) {
    sbi_rt::set_timer(time);
}

/// 发送处理器间中断
/// 
/// # 参数
/// 
/// * `hart_mask` - 目标处理器掩码
pub fn send_ipi(hart_mask: HartMask) {
    sbi_rt::send_ipi(hart_mask);
}

/// 远程TLB刷新
/// 
/// # 参数
/// 
/// * `hart_mask` - 目标处理器掩码
pub fn remote_fence_i(hart_mask: HartMask) {
    sbi_rt::remote_fence_i(hart_mask);
}

/// 远程TLB刷新(SFENCE.VMA)
/// 
/// # 参数
/// 
/// * `hart_mask` - 目标处理器掩码
/// * `start` - 开始地址
/// * `size` - 地址范围大小
pub fn remote_sfence_vma(hart_mask: HartMask, start: usize, size: usize) {
    sbi_rt::remote_sfence_vma(hart_mask, start, size);
}

/// 远程TLB刷新(SFENCE.VMA.ASID)
/// 
/// # 参数
/// 
/// * `hart_mask` - 目标处理器掩码
/// * `start` - 开始地址
/// * `size` - 地址范围大小
/// * `asid` - 地址空间ID
pub fn remote_sfence_vma_asid(hart_mask: HartMask, start: usize, size: usize, asid: usize) {
    sbi_rt::remote_sfence_vma_asid(hart_mask, start, size, asid);
}

/// 获取SBI规范版本
pub fn get_spec_version() -> (usize, usize) {
    let version = sbi_rt::get_spec_version();
    (version.major(), version.minor())
}

/// 获取SBI实现ID
pub fn get_impl_id() -> usize {
    sbi_rt::get_sbi_impl_id()
}

/// 获取SBI实现版本
pub fn get_impl_version() -> usize {
    sbi_rt::get_sbi_impl_version()
}

/// 获取可见的MVENDORID CSR值
pub fn get_mvendorid() -> usize {
    sbi_rt::get_mvendorid()
}

/// 获取可见的MARCHID CSR值
pub fn get_marchid() -> usize {
    sbi_rt::get_marchid()
}

/// 获取可见的MIMPID CSR值
pub fn get_mimpid() -> usize {
    sbi_rt::get_mimpid()
}
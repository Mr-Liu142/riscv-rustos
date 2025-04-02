//! 增强型异常处理器
//!
//! 此模块提供更详细的异常处理器实现，用于在关键异常发生时
//! 打印详细的诊断信息并使系统停机，便于开发者定位问题。

use crate::println;
use crate::trap::ds::{TrapContext, TrapHandlerResult, TrapCause, TrapType};
use crate::util::sbi::system::{shutdown, ShutdownReason};

/// 通用异常处理函数，打印详细信息并停机
///
/// # 参数
///
/// * `ctx` - 异常上下文
/// * `exception_type` - 异常类型描述
/// * `should_panic` - 是否应该触发系统停机
fn handle_exception_with_details(
    ctx: &mut TrapContext,
    exception_type: &str,
    should_panic: bool
) -> TrapHandlerResult {
    let cause = ctx.get_cause();
    
    // 打印分隔线和标题
    println!("\n═════════════════════════════════════════════════════");
    println!("FATAL ERROR: {}", exception_type);
    println!("═════════════════════════════════════════════════════");
    
    // 打印详细信息
    println!("Cause: {:?} (Code: {})", cause.to_trap_type(), cause.code());
    println!("Instruction Address: {:#018x}", ctx.sepc);
    println!("Fault Address/Value: {:#018x}", ctx.stval);
    
    // 打印寄存器状态
    println!("\nRegister State:");
    println!("  sstatus: {:#018x}", ctx.sstatus);
    println!("  ra(x1):  {:#018x}  sp(x2):   {:#018x}", ctx.x[1], ctx.x[2]);
    println!("  gp(x3):  {:#018x}  tp(x4):   {:#018x}", ctx.x[3], ctx.x[4]);
    println!("  t0(x5):  {:#018x}  t1(x6):   {:#018x}", ctx.x[5], ctx.x[6]);
    println!("  t2(x7):  {:#018x}  s0/fp(x8):{:#018x}", ctx.x[7], ctx.x[8]);
    println!("  a0(x10): {:#018x}  a1(x11):  {:#018x}", ctx.x[10], ctx.x[11]);
    println!("  a2(x12): {:#018x}  a3(x13):  {:#018x}", ctx.x[12], ctx.x[13]);
    
    // 结束分隔线
    println!("═════════════════════════════════════════════════════\n");
    
    // 如果需要停机，调用系统停机函数
    if should_panic {
        println!("System halting due to unrecoverable exception.");
        // 短暂延迟，确保消息能够输出
        for _ in 0..10000000 {
            core::hint::spin_loop();
        }
        shutdown(ShutdownReason::SystemFailure);
    }
    
    TrapHandlerResult::Handled
}

/// 指令页错误增强处理器
pub fn enhanced_instruction_page_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    handle_exception_with_details(
        ctx,
        "INSTRUCTION PAGE FAULT",
        true
    )
}

/// 加载页错误增强处理器
pub fn enhanced_load_page_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    handle_exception_with_details(
        ctx,
        "LOAD PAGE FAULT",
        true
    )
}

/// 存储页错误增强处理器
pub fn enhanced_store_page_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    handle_exception_with_details(
        ctx,
        "STORE PAGE FAULT",
        true
    )
}

/// 非法指令增强处理器
pub fn enhanced_illegal_instruction_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    // 尝试获取异常指令的值
    let instruction_bytes = ctx.stval;
    
    // 创建更具体的异常描述
    let description = "ILLEGAL INSTRUCTION";
    
    // 打印额外的指令相关信息
    println!("Illegal instruction value: {:#010x}", instruction_bytes);
    
    // 尝试解析一些常见的非法指令情况
    if instruction_bytes == 0 {
        println!("(Null instruction/Fetch error)");
    } else if instruction_bytes & 0x3 != 0x3 {
        println!("(Misaligned instruction)");
    } else if (instruction_bytes & 0x7F) == 0x7B {
        println!("(Possible privileged instruction)");
    }
    
    handle_exception_with_details(
        ctx,
        description,
        true
    )
}

/// 指令访问错误增强处理器
pub fn enhanced_instruction_access_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    handle_exception_with_details(
        ctx,
        "INSTRUCTION ACCESS FAULT",
        true
    )
}

/// 加载访问错误增强处理器
pub fn enhanced_load_access_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    handle_exception_with_details(
        ctx,
        "LOAD ACCESS FAULT",
        true
    )
}

/// 存储访问错误增强处理器
pub fn enhanced_store_access_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    handle_exception_with_details(
        ctx,
        "STORE ACCESS FAULT",
        true
    )
}

/// 未知异常增强处理器
pub fn enhanced_unknown_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    let cause = ctx.get_cause();
    let type_str = if cause.is_interrupt() {
        "UNKNOWN INTERRUPT"
    } else {
        "UNKNOWN EXCEPTION"
    };
    
    println!("Code: {}", cause.code());
    
    handle_exception_with_details(
        ctx,
        type_str,
        true
    )
}

/// 断点异常处理器
/// 断点异常处理器
pub fn enhanced_breakpoint_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    // 保存原始PC
    let orig_pc = ctx.sepc;
    
    // 打印更详细的调试信息
    println!("Breakpoint at PC: {:#x}, Instruction bytes: {:#x}", orig_pc, ctx.stval);
    
    // 检查是否为压缩指令
    let is_compressed = false;  // 这需要读取内存中的指令来确定，简化版先假设不是压缩指令
    
    // 处理断点异常
    let result = handle_exception_with_details(
        ctx,
        "BREAKPOINT",
        false // 断点不需要停机
    );
    
    // 根据指令是否压缩，更新PC
    let instruction_size = if is_compressed { 2 } else { 4 };
    ctx.set_return_addr(orig_pc + instruction_size);
    
    println!("Breakpoint handled: PC advanced from {:#x} to {:#x}", orig_pc, ctx.sepc);
    
    // 在返回前进一步验证目标地址的有效性
    // 在实际代码中，这需要一个内存访问检查，简化版先省略
    
    // 返回处理结果
    result
}

static mut HANDLERS_REGISTERED: bool = false;

/// 注册所有增强型异常处理器
pub fn register_enhanced_handlers() {
    // 检查是否已经注册，防止重复注册
    unsafe {
        if HANDLERS_REGISTERED {
            println!("Enhanced exception handlers already registered");
            return;
        }
        HANDLERS_REGISTERED = true;
    }
    use crate::trap::infrastructure::di;
    
    // 注册页错误处理器
    di::register_handler(
        TrapType::InstructionPageFault,
        enhanced_instruction_page_fault_handler,
        10, // 高优先级
        "Enhanced Instruction Page Fault Handler"
    );
    
    di::register_handler(
        TrapType::LoadPageFault,
        enhanced_load_page_fault_handler,
        10,
        "Enhanced Load Page Fault Handler"
    );
    
    di::register_handler(
        TrapType::StorePageFault,
        enhanced_store_page_fault_handler,
        10,
        "Enhanced Store Page Fault Handler"
    );
    
    // 注册非法指令处理器
    di::register_handler(
        TrapType::IllegalInstruction,
        enhanced_illegal_instruction_handler,
        10,
        "Enhanced Illegal Instruction Handler"
    );
    
    // 注册指令访问错误处理器
    di::register_handler(
        TrapType::InstructionAccessFault,
        enhanced_instruction_access_fault_handler,
        10,
        "Enhanced Instruction Access Fault Handler"
    );
    
    // 注册断点处理器
    di::register_handler(
        TrapType::Breakpoint,
        enhanced_breakpoint_handler,
        10,
        "Enhanced Breakpoint Handler"
    );
    
    // 注册未知异常处理器
    di::register_handler(
        TrapType::Unknown,
        enhanced_unknown_handler,
        10,
        "Enhanced Unknown Exception Handler"
    );
    
    println!("Enhanced exception handlers registered successfully");
}
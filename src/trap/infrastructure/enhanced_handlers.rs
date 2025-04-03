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

/// 地址未对齐异常增强处理器
///
/// 处理三种类型的未对齐异常：
/// 1. 指令地址未对齐 (code=0)
/// 2. 加载地址未对齐 (code=4)
/// 3. 存储地址未对齐 (code=6)
pub fn enhanced_misaligned_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    let cause = ctx.get_cause();
    let trap_type = cause.to_trap_type();
    
    // 确定异常类型
    let (exception_type, alignment_req) = match trap_type {
        TrapType::InstructionMisaligned => ("INSTRUCTION MISALIGNED", 2),
        TrapType::LoadMisaligned => ("LOAD MISALIGNED", 0),
        TrapType::StoreMisaligned => ("STORE MISALIGNED", 0),
        _ => return TrapHandlerResult::Pass, // 不是未对齐异常，传递给下一个处理器
    };
    
    // 打印分隔线和标题
    println!("\n═════════════════════════════════════════════════════");
    println!("FATAL ERROR: {}", exception_type);
    println!("═════════════════════════════════════════════════════");
    
    // 打印详细信息
    println!("Cause: Code {} ({})", cause.code(), exception_type);
    println!("Instruction Address: {:#018x}", ctx.sepc);
    println!("Misaligned Address: {:#018x}", ctx.stval);
    
    // 计算地址未对齐的程度和需要的对齐
    let misalignment = ctx.stval & 0xF;
    let required_alignment = if alignment_req > 0 {
        alignment_req
    } else {
        // 尝试根据异常类型和上下文推断所需的对齐
        match trap_type {
            TrapType::LoadMisaligned | TrapType::StoreMisaligned => {
                // 尝试从指令猜测访问大小，但这只是一个简化的启发式方法
                if misalignment & 0x7 != 0 {
                    8 // 可能是双字访问
                } else if misalignment & 0x3 != 0 {
                    4 // 可能是字访问
                } else if misalignment & 0x1 != 0 {
                    2 // 可能是半字访问
                } else {
                    1 // 字节访问不需要对齐
                }
            },
            _ => 2, // 默认为指令对齐
        }
    };
    
    // 添加有关未对齐异常的具体描述
    println!("\nMisalignment Details:");
    match trap_type {
        TrapType::InstructionMisaligned => {
            println!("  Problem: Attempted to fetch instruction from misaligned address.");
            println!("  Address {:#018x} is not aligned to {} bytes boundary.", ctx.stval, required_alignment);
            println!("  RISC-V requires instruction addresses to be aligned to 2 or 4 bytes.");
            println!("  This may be caused by a jump/branch to an odd address or a corrupted return address.");
        },
        TrapType::LoadMisaligned => {
            println!("  Problem: Attempted to load data from misaligned address.");
            println!("  Address {:#018x} is not aligned to {} bytes boundary.", ctx.stval, required_alignment);
            println!("  Multi-byte loads must be aligned to the size of the access.");
            println!("  For example, a 4-byte (word) load requires 4-byte alignment.");
        },
        TrapType::StoreMisaligned => {
            println!("  Problem: Attempted to store data to misaligned address.");
            println!("  Address {:#018x} is not aligned to {} bytes boundary.", ctx.stval, required_alignment);
            println!("  Multi-byte stores must be aligned to the size of the access.");
            println!("  For example, a 4-byte (word) store requires 4-byte alignment.");
        },
        _ => { }, // 不应该到达这里
    }
    
    // 打印当前指令的相关信息
    println!("\nInstruction Information:");
    println!("  PC Address: {:#018x}", ctx.sepc);
    println!("  This is likely where the misaligned access was attempted.");
    
    // 打印寄存器状态
    println!("\nRegister State:");
    println!("  sstatus: {:#018x}", ctx.sstatus);
    println!("  ra(x1):  {:#018x}  sp(x2):   {:#018x}", ctx.x[1], ctx.x[2]);
    println!("  gp(x3):  {:#018x}  tp(x4):   {:#018x}", ctx.x[3], ctx.x[4]);
    println!("  t0(x5):  {:#018x}  t1(x6):   {:#018x}", ctx.x[5], ctx.x[6]);
    println!("  t2(x7):  {:#018x}  s0/fp(x8):{:#018x}", ctx.x[7], ctx.x[8]);
    println!("  a0(x10): {:#018x}  a1(x11):  {:#018x}", ctx.x[10], ctx.x[11]);
    println!("  a2(x12): {:#018x}  a3(x13):  {:#018x}", ctx.x[12], ctx.x[13]);
    
    // 建议修复方法
    println!("\nPossible Solutions:");
    println!("  1. Ensure all memory accesses are properly aligned.");
    println!("  2. Check for pointer arithmetic errors that could lead to misalignment.");
    println!("  3. For loads/stores, consider using unaligned access instructions if supported by your hardware.");
    println!("  4. Verify that function pointers and return addresses are correctly aligned.");
    
    // 结束分隔线
    println!("═════════════════════════════════════════════════════\n");
    
    // 如果需要停机，调用系统停机函数
    println!("System halting due to unrecoverable misaligned address exception.");
    // 短暂延迟，确保消息能够输出
    for _ in 0..10000000 {
        core::hint::spin_loop();
    }
    crate::util::sbi::system::shutdown(crate::util::sbi::system::ShutdownReason::SystemFailure);
    
    TrapHandlerResult::Handled
}

/// 内存访问错误处理器
///
/// 处理内存访问相关错误：
/// - 加载访问错误 (Load Access Fault, code=5)
/// - 存储访问错误 (Store Access Fault, code=7)
pub fn enhanced_memory_access_fault_handler(ctx: &mut TrapContext) -> TrapHandlerResult {
    let cause = ctx.get_cause();
    let trap_type = cause.to_trap_type();
    
    // 确定是什么类型的访问错误
    let fault_type = match trap_type {
        TrapType::LoadAccessFault => "LOAD ACCESS FAULT",
        TrapType::StoreAccessFault => "STORE ACCESS FAULT",
        _ => return TrapHandlerResult::Pass, // 非访问错误，传递给其他处理器
    };
    
    println!("\n═════════════════════════════════════════════════════");
    println!("FATAL ERROR: {}", fault_type);
    println!("═════════════════════════════════════════════════════");
    
    // 详细的访问错误信息
    println!("Cause: {:?} (Code: {})", trap_type, cause.code());
    println!("Instruction Address: {:#018x}", ctx.sepc);
    println!("Faulting Address: {:#018x}", ctx.stval);
    
    // 分析可能的原因
    let address = ctx.stval;
    println!("\nProblem Analysis:");
    
    // 检查地址对齐
    let alignment_issue = match trap_type {
        TrapType::LoadAccessFault => {
            // 检查不同大小的加载操作对齐要求
            (address & 0x1) != 0 || (address & 0x3) != 0 || (address & 0x7) != 0
        },
        TrapType::StoreAccessFault => {
            // 检查不同大小的存储操作对齐要求
            (address & 0x1) != 0 || (address & 0x3) != 0 || (address & 0x7) != 0
        },
        _ => false,
    };
    
    if alignment_issue {
        println!("  - Misalignment detected: address {:#018x} is not aligned", address);
        println!("    Alignment status: 2-byte={}, 4-byte={}, 8-byte={}", 
                 (address & 0x1) == 0, 
                 (address & 0x3) == 0, 
                 (address & 0x7) == 0);
        println!("    Note: This may contribute to the access fault on some implementations.");
    }
    
    // 检查地址范围
    if address < 0x80000000 || address >= 0x88000000 {
        println!("  - Address {:#018x} may be outside valid physical memory range", address);
        println!("    The typical RISC-V memory range for simple systems is 0x80000000-0x88000000");
    }
    
    // 内存映射和权限问题
    println!("  - Memory at this address may not be mapped in the page tables");
    println!("  - You may not have sufficient privileges to access this memory region");
    println!("  - The physical memory at this address may not exist");
    println!("  - The memory controller may have rejected the access");
    
    if alignment_issue {
        println!("\nThis appears to involve both alignment and access permission issues.");
        println!("On RISC-V systems, unaligned accesses to unmapped/protected memory");
        println!("regions typically result in access faults rather than misaligned exceptions.");
    }
    
    // 寄存器状态
    println!("\nRegister State:");
    println!("  sstatus: {:#018x}", ctx.sstatus);
    println!("  ra(x1):  {:#018x}  sp(x2):   {:#018x}", ctx.x[1], ctx.x[2]);
    println!("  gp(x3):  {:#018x}  tp(x4):   {:#018x}", ctx.x[3], ctx.x[4]);
    println!("  t0(x5):  {:#018x}  t1(x6):   {:#018x}", ctx.x[5], ctx.x[6]);
    println!("  t2(x7):  {:#018x}  s0/fp(x8):{:#018x}", ctx.x[7], ctx.x[8]);
    println!("  a0(x10): {:#018x}  a1(x11):  {:#018x}", ctx.x[10], ctx.x[11]);
    println!("  a2(x12): {:#018x}  a3(x13):  {:#018x}", ctx.x[12], ctx.x[13]);
    
    // 可能的解决方案
    println!("\nPossible Solutions:");
    println!("  1. Ensure the memory address is within valid memory range");
    println!("  2. Check memory permissions and page table mappings");
    println!("  3. If this is an alignment issue, ensure all accesses are properly aligned");
    println!("  4. Verify that physical memory exists at the target address");
    println!("  5. Check for use-after-free or buffer overflow issues in your code");
    
    println!("═════════════════════════════════════════════════════\n");
    
    // 系统停机
    println!("System halting due to unrecoverable memory access fault.");
    for _ in 0..10000000 {
        core::hint::spin_loop();
    }
    crate::util::sbi::system::shutdown(crate::util::sbi::system::ShutdownReason::SystemFailure);
    
    TrapHandlerResult::Handled
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

    // 注册未对齐地址处理器，分别注册三种类型
    di::register_handler(
        TrapType::InstructionMisaligned,
        enhanced_misaligned_handler,
        10,
        "Enhanced Instruction Misaligned Handler"
    );
    
    di::register_handler(
        TrapType::LoadMisaligned,
        enhanced_misaligned_handler,
        10,
        "Enhanced Load Misaligned Handler"
    );
    
    di::register_handler(
        TrapType::StoreMisaligned,
        enhanced_misaligned_handler,
        10,
        "Enhanced Store Misaligned Handler"
    );

    di::register_handler(
        TrapType::LoadAccessFault,
        enhanced_memory_access_fault_handler,
        10,
        "Enhanced Load Access Fault Handler"
    );
    
    di::register_handler(
        TrapType::StoreAccessFault,
        enhanced_memory_access_fault_handler,
        10,
        "Enhanced Store Access Fault Handler"
    );
    
    
    println!("Enhanced exception handlers registered successfully");
}
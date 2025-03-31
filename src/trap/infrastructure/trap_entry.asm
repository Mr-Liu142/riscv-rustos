# 中断入口点汇编代码
# 保存和恢复所有寄存器，包括特权级寄存器

.section .text
.globl __trap_entry
.globl __trap_return
.align 4  # 确保4字节对齐

# RISC-V寄存器上下文大小 (32 gp + 4 CSR) * 8 = 288字节
.equ CONTEXT_SIZE, 288

# 中断入口点
__trap_entry:
    # 分配栈空间保存上下文
    addi sp, sp, -CONTEXT_SIZE
    
    # 保存通用寄存器 (x0是零寄存器，不需要保存)
    sd x1, 8(sp)    # ra
    sd x2, 16(sp)   # sp (原始sp值)
    sd x3, 24(sp)   # gp
    sd x4, 32(sp)   # tp
    sd x5, 40(sp)   # t0
    sd x6, 48(sp)   # t1
    sd x7, 56(sp)   # t2
    sd x8, 64(sp)   # s0/fp
    sd x9, 72(sp)   # s1
    sd x10, 80(sp)  # a0
    sd x11, 88(sp)  # a1
    sd x12, 96(sp)  # a2
    sd x13, 104(sp) # a3
    sd x14, 112(sp) # a4
    sd x15, 120(sp) # a5
    sd x16, 128(sp) # a6
    sd x17, 136(sp) # a7
    sd x18, 144(sp) # s2
    sd x19, 152(sp) # s3
    sd x20, 160(sp) # s4
    sd x21, 168(sp) # s5
    sd x22, 176(sp) # s6
    sd x23, 184(sp) # s7
    sd x24, 192(sp) # s8
    sd x25, 200(sp) # s9
    sd x26, 208(sp) # s10
    sd x27, 216(sp) # s11
    sd x28, 224(sp) # t3
    sd x29, 232(sp) # t4
    sd x30, 240(sp) # t5
    sd x31, 248(sp) # t6
    
    # 保存特权级CSR寄存器
    csrr t0, sstatus
    sd t0, 256(sp)  # 保存sstatus
    
    csrr t0, sepc
    sd t0, 264(sp)  # 保存sepc（中断返回地址）
    
    csrr t0, scause
    sd t0, 272(sp)  # 保存scause（中断原因）
    
    csrr t0, stval
    sd t0, 280(sp)  # 保存stval（中断附加信息）
    
    # 为Rust处理函数准备参数 - 传递上下文指针
    mv a0, sp
    
    # 调用Rust中断处理函数
    call handle_trap
    
    # 跳转到中断返回代码
    j __trap_return

# 中断返回代码
__trap_return:
    # 恢复特权级CSR寄存器
    ld t0, 256(sp)
    csrw sstatus, t0  # 恢复sstatus
    
    ld t0, 264(sp)
    csrw sepc, t0     # 恢复sepc
    
    # 不需要恢复scause和stval，它们是只读的或由硬件设置
    
    # 恢复通用寄存器
    ld x1, 8(sp)    # ra
    # 暂时跳过sp (x2)
    ld x3, 24(sp)   # gp
    ld x4, 32(sp)   # tp
    ld x5, 40(sp)   # t0
    ld x6, 48(sp)   # t1
    ld x7, 56(sp)   # t2
    ld x8, 64(sp)   # s0/fp
    ld x9, 72(sp)   # s1
    ld x10, 80(sp)  # a0
    ld x11, 88(sp)  # a1
    ld x12, 96(sp)  # a2
    ld x13, 104(sp) # a3
    ld x14, 112(sp) # a4
    ld x15, 120(sp) # a5
    ld x16, 128(sp) # a6
    ld x17, 136(sp) # a7
    ld x18, 144(sp) # s2
    ld x19, 152(sp) # s3
    ld x20, 160(sp) # s4
    ld x21, 168(sp) # s5
    ld x22, 176(sp) # s6
    ld x23, 184(sp) # s7
    ld x24, 192(sp) # s8
    ld x25, 200(sp) # s9
    ld x26, 208(sp) # s10
    ld x27, 216(sp) # s11
    ld x28, 224(sp) # t3
    ld x29, 232(sp) # t4
    ld x30, 240(sp) # t5
    ld x31, 248(sp) # t6
    
    # 最后恢复sp
    ld x2, 16(sp)   # 先加载原始sp值到t0
    addi sp, sp, CONTEXT_SIZE  # 调整栈指针
    
    # 返回到中断点
    sret
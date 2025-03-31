# 中断入口点汇编代码

.section .text
.globl __trap_entry
.globl __trap_return
.align 2

# 中断入口点
__trap_entry:
    # 预留栈空间保存寄存器状态
    addi sp, sp, -256
    
    # 保存通用寄存器
    sd x1, 0(sp)
    sd x2, 8(sp)
    sd x3, 16(sp)
    # 继续保存所有寄存器...
    sd x31, 248(sp)
    
    # 调用Rust中断处理函数
    call handle_trap
    
    # 跳转到中断返回代码
    j __trap_return

# 中断返回代码
__trap_return:
    # 恢复寄存器
    ld x1, 0(sp)
    ld x2, 8(sp)
    ld x3, 16(sp)
    # 继续恢复所有寄存器...
    ld x31, 248(sp)
    
    # 恢复栈指针
    addi sp, sp, 256
    
    # 返回到中断点
    sret
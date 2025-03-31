# 变量定义
TARGET := riscv64gc-unknown-none-elf
MODE := debug
KERNEL_ELF := target/$(TARGET)/$(MODE)/riscv-rustos
KERNEL_BIN := $(KERNEL_ELF).bin

# QEMU模拟器配置
QEMU := qemu-system-riscv64
QEMUOPTS := -machine virt -nographic -bios default -device loader,file=$(KERNEL_BIN),addr=0x80200000

# 编译配置
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# 默认目标
.PHONY: kernel build clean qemu run

build: kernel

# 编译内核
kernel:
	cargo build
	$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $(KERNEL_BIN)

# 清理
clean:
	cargo clean

# 运行QEMU
qemu: build
	$(QEMU) $(QEMUOPTS)

# 快捷命令：构建并运行
run: build qemu

#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::panic::PanicInfo;
use core::arch::asm;

mod console;

// 启动栈大小
const STACK_SIZE: usize = 4096 * 4;

// 用于存放栈的内存区域
#[link_section = ".bss.stack"]
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut msg = "Panicked: ";
    if let Some(location) = info.location() {
        msg = "Panicked at";
        console::print_str(msg);
        console::print_str(location.file());
        console::print_str(":");
        console::print_num(location.line() as usize);
        console::print_str(": ");
        if let Some(message) = info.message() {
            console::print_str(format_args!("{}", message).as_str().unwrap_or("Unknown error"));
        }
    } else {
        console::print_str(msg);
    }
    loop {}
}

#[no_mangle]
#[link_section = ".text.entry"]
fn _start() -> ! {
    unsafe {
        // 设置栈指针
        let stack_top = STACK.as_ptr().add(STACK_SIZE);
        asm!(
            "mv sp, {0}",
            in(reg) stack_top,
        );
        
        // 清除BSS段
        extern "C" {
            fn sbss();
            fn ebss();
        }
        let sbss_addr = sbss as usize;
        let ebss_addr = ebss as usize;
        
        // 逐字节清零
        for addr in sbss_addr..ebss_addr {
            core::ptr::write_volatile(addr as *mut u8, 0);
        }
        
        // 跳转到Rust主函数
        rust_main();
    }
    
    loop {}
}

#[no_mangle]
fn rust_main() -> ! {
    // 使用直接的SBI调用确保输出可见
    console::print_str("Hello, world!\n");
    loop {}
}
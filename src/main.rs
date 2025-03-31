#![no_std]
#![no_main]
#![feature(panic_info_message)]

use core::panic::PanicInfo;
use core::arch::asm;

mod console;
mod util;

// 启动栈大小
const STACK_SIZE: usize = 4096 * 4;

// 用于存放栈的内存区域
#[link_section = ".bss.stack"]
static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        console::print_str("Panicked at ");
        console::print_str(location.file());
        console::print_str(":");
        console::print_num(location.line() as usize);
        console::print_str(": ");
        if let Some(message) = info.message() {
            if let Some(args_str) = format_args!("{}", message).as_str() {
                console::print_str(args_str);
            } else {
                console::print_str("Unknown error");
            }
        }
    } else {
        console::print_str("Panicked: Unknown location");
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
    println!("Hello, RISC-V RustOS!");
    
    // 获取SBI版本信息
    let (major, minor) = util::sbi::get_spec_version();
    println!("SBI版本: {}.{}", major, minor);
    
    println!("SBI实现ID: {}", util::sbi::get_impl_id());
    println!("SBI实现版本: {}", util::sbi::get_impl_version());
    
    loop {}
}

// 只导出print函数，println!宏已经通过#[macro_export]导出到了crate根
pub use console::print;
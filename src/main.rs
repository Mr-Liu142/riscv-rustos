#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(naked_functions)]
#![feature(asm_const)]

use core::panic::PanicInfo;
use core::arch::asm;

mod console;
mod util;
mod trap;

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

    // 初始化中断系统
    trap::init();  // 这应该内部调用DI系统的初始化


    // 初始化中断系统
    trap::init();
    
    // 使用新封装的系统信息功能
    let sys_info = util::sbi::system::get_system_info();
    sys_info.print();
    
    // 测试控制台输入功能
    println!("Please input some text (max 20 characters):");
    let mut buffer = [0u8; 21];
    let len = util::sbi::console::getline(&mut buffer, true);
    println!("You entered {} characters: {}", len, core::str::from_utf8(&buffer[..len]).unwrap_or("Invalid UTF-8"));
    
    // 测试时钟功能
    println!("Current time count: {}", util::sbi::timer::get_time());
    println!("Waiting for a while...");
    util::sbi::timer::sleep_cycles(10000000); // 等待一段时间
    println!("Current time count: {}", util::sbi::timer::get_time());
    
    // 演示TLB刷新
    println!("Flushing local TLB...");
    util::sbi::tlb::flush_local();
    
    // 设置一个相对定时器
    println!("Setting relative timer, interrupt will be triggered after 1 second...");
    // 注意：实际使用需要设置中断处理程序
    util::sbi::timer::set_timer_rel(10000000); // 假设10M周期约为1秒
    
    // 循环等待
    println!("System startup completed, entering main loop");
    loop {
        // 尝试获取控制台输入
        if let Some(c) = util::sbi::console::try_getchar() {
            match c {
                'q' => {
                    println!("User requested shutdown");
                    util::sbi::system::shutdown(util::sbi::system::ShutdownReason::UserRequest);
                }
                'r' => {
                    println!("User requested reboot");
                    util::sbi::system::reboot(util::sbi::system::RebootType::Cold);
                }
                _ => {
                    println!("Key pressed: {}", c);
                }
            }
        }
        
        // 使用自旋循环提示处理器可以省电
        core::hint::spin_loop();
    }
}

// 只导出print函数，println!宏已经通过#[macro_export]导出到了crate根
pub use console::print;

use core::fmt;
use crate::util::sbi;

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    Stdout.write_fmt(args).unwrap();
}

pub fn print_str(s: &str) {
    for c in s.chars() {
        sbi::console_putchar(c);
    }
}

pub fn print_num(num: usize) {
    if num == 0 {
        sbi::console_putchar('0');
        return;
    }
    
    let mut n = num;
    let mut buf = [0u8; 20]; // 足够存储64位整数
    let mut i = 0;
    
    while n > 0 {
        buf[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
    }
    
    while i > 0 {
        i -= 1;
        sbi::console_putchar(buf[i] as char);
    }
}

struct Stdout;

impl core::fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print_str(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}
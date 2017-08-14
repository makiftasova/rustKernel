#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![no_std]

extern crate volatile;
extern crate rlibc;
extern crate spin;

#[macro_use]
mod vga_buffer;

#[no_mangle]
pub extern "C" fn rust_main() {
    use core::fmt::Write;
    vga_buffer::WRITER.lock().write_str("Hello VGA_BUFFER!");
    write!(vga_buffer::WRITER.lock(), "some numbers: {} {}", 42, 1.337);

    vga_buffer::clear_screen();
    println!("Hello {}{}", "World", "!");

    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt() -> ! {
    loop {}
}

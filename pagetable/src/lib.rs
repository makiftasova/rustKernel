#![feature(lang_items)]
#![feature(const_fn)]
#![feature(unique)]
#![no_std]

extern crate volatile;
extern crate rlibc;
extern crate spin;
extern crate multiboot2;
#[macro_use]
extern crate bitflags;
extern crate x86_64;

#[macro_use]
mod vga_buffer;
mod memory;

#[no_mangle]
pub extern "C" fn rust_main(multiboot_information_address: usize) {
    use memory::FrameAllocator;

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };
    let memory_map_tag = boot_info.memory_map_tag().expect(
        "memory map tag required!",
    );

    println!("Memory Areas:");
    for area in memory_map_tag.memory_areas() {
        println!(
            "\tstart: 0x{:x},\tlength: 0x{:x}",
            area.base_addr,
            area.length
        );
    }

    let elf_sections_tag = boot_info.elf_sections_tag().expect(
        "ELF-sections tag required!",
    );

    println!("Kenel sections:");
    for section in elf_sections_tag.sections() {
        println!(
            "\taddr: 0x{:x},\tsize: 0x{:x},\tflags: 0x{:x}",
            section.addr,
            section.size,
            section.flags
        );
    }

    let kernel_start = elf_sections_tag.sections().map(|s| s.addr).min().unwrap();

    let kernel_end = elf_sections_tag
        .sections()
        .map(|s| s.addr + s.size)
        .max()
        .unwrap();

    println!(
        "Kernel:\tstart: 0x{:x}\tend: 0x{:x}",
        kernel_start,
        kernel_end
    );

    let multiboot_start = multiboot_information_address;
    let multiboot_end = multiboot_start + (boot_info.total_size as usize);

    println!(
        "Multiboot:\tstart: 0x{:x}\tend: 0x{:x}",
        multiboot_start,
        multiboot_end
    );

    let mut frame_allocator = memory::AreaFrameAllocator::new(
        kernel_start as usize,
        kernel_end as usize,
        multiboot_start,
        multiboot_end,
        memory_map_tag.memory_areas(),
    );

    memory::test_paging(&mut frame_allocator);


    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}


#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println!("\n\nPANIC in {} at line {}", file, line);
    println!("\t{}", fmt);
    loop {}
}

#![no_std]
#![no_main]
#![feature(asm)]
#![feature(custom_test_frameworks)]
#![test_runner(blog_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use blog_os::println;
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::{
    instructions::segmentation,
    structures::paging::{Mapper, Page, PageTableFlags, Size4KiB},
    VirtAddr,
};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use blog_os::allocator;
    use blog_os::memory::{self, BootInfoFrameAllocator};

    println!("Hello World{}", "!");
    blog_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    // init(&mut mapper);

    unsafe {
        core::ptr::copy(
            user as u64 as *const u8,
            0x_4444_4444_0000 as *mut u8,
            0x100,
        )
    }

    jump_to_user();
}

fn init(mapper: &mut impl Mapper<Size4KiB>) {
    let page_range = {
        let heap_start = VirtAddr::new(user as u64);
        let heap_end = heap_start + 0x1000u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    let flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;

    for page in page_range {
        unsafe { mapper.update_flags(page, flags).unwrap().flush() }
    }
}

fn jump_to_user() -> ! {
    println!("Jumping to userspace");

    let stack_end = {
        const STACK_SIZE: usize = 4096 * 5;
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

        let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
        let stack_end = stack_start + STACK_SIZE;
        stack_end
    };

    unsafe {
        segmentation::load_ds(blog_os::gdt::GDT.1.user_data_selector);
        segmentation::load_es(blog_os::gdt::GDT.1.user_data_selector);
        segmentation::load_fs(blog_os::gdt::GDT.1.user_data_selector);
        segmentation::load_gs(blog_os::gdt::GDT.1.user_data_selector);

        asm!(
            // Setup interrupt stack frame
            "push {user_data_segment_selector:r}", // ss
            "push {user_stack}",
            "pushfq",
            "push {user_code_segment_selector:r}", // cs
            "push {f}",

            // Return from constructed interrupt stack frame
            "iretq",

            user_stack = in(reg) stack_end.as_u64(),
            user_data_segment_selector = in(reg) blog_os::gdt::GDT.1.user_data_selector.0,
            user_code_segment_selector = in(reg) blog_os::gdt::GDT.1.user_code_selector.0,
            f = in(reg) 0x_4444_4444_0000u64,
            options(noreturn),
        )
    }
}

extern "C" fn user() -> ! {
    loop {}
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    blog_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    blog_os::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}

use crate::{gdt, println};
use lazy_static::lazy_static;
use pic8259_simple::ChainedPics;
use spin;
#[cfg(feature = "x2apic")]
use x2apic::lapic::{LocalApic, LocalApicBuilder};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub const VECTOR_OFFSET: u8 = if cfg!(feature = "x2apic") {
    PIC_2_OFFSET + 8
} else {
    PIC_1_OFFSET
};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = VECTOR_OFFSET,
    Spurious,
    ApicError,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[cfg(feature = "x2apic")]
lazy_static! {
    pub static ref LAPIC: spin::Mutex<LocalApic> = {
        let lapic = LocalApicBuilder::new()
            .timer_vector(InterruptIndex::Timer.as_usize())
            .spurious_vector(InterruptIndex::Spurious.as_usize())
            .error_vector(InterruptIndex::ApicError.as_usize())
            .build()
            .unwrap();
        spin::Mutex::new(lapic)
    };
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Spurious.as_usize()].set_handler_fn(spurious_handler);

        #[cfg(feature = "x2apic")]
        {
            idt[InterruptIndex::ApicError.as_usize()].set_handler_fn(apic_error_handler);
        }
        
        idt
    };
}

pub fn init() {
    IDT.load();
    unsafe { PICS.lock().initialize() }

    #[cfg(feature = "x2apic")]
    unsafe {
        PICS.lock().disable();
        LAPIC.lock().enable();
    }
    #[cfg(not(feature = "x2apic"))]
    crate::pit::Timer::init();

    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    println!("Timer interrupt!");
    #[cfg(not(feature = "x2apic"))]
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
    #[cfg(feature = "x2apic")]
    unsafe {
        LAPIC.lock().end_of_interrupt()
    }
}

extern "x86-interrupt" fn spurious_handler(stack_frame: &mut InterruptStackFrame) {
    panic!("SPURIOUS INTERRUPT\n{:#?}", stack_frame);
}

#[cfg(feature = "x2apic")]
extern "x86-interrupt" fn apic_error_handler(stack_frame: &mut InterruptStackFrame) {
    panic!("APIC ERROR\n{:#?}", stack_frame);
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}

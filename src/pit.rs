//! This module configures the 8284 programmable interval timer (PIT).

use crate::println;
use core::arch::x86_64::_rdtsc;
use core::sync::atomic::{self, Ordering};
use x86_64::instructions::port::Port;

const TIMER_FREQ: u32 = 10;

/// The 8284 programmable interval timer (PIT).
///
/// This timer fires IRQ0 periodically and is used for preemption ([`timer_handler`](arch::x86_64::irq::timer_handler)).
///
/// For an overview see Chapter 11, “8254 Timers,” in the Intel® 495 Series Chipset Family On-Package Platform Controller Hub (PCH) Datasheet, Volume 1 of 2.
/// For timer registers summary see Chapter 19, “8254 Timer,” in the Intel® 495 Series Chipset Family On-Package Platform Controller Hub (PCH) Datasheet, Volume 2 of 2
pub struct Timer;

impl Timer {
    /// The 8254 unit's counter frequency in Hz.
    const COUNTER_FREQ: u32 = 1_193_181;

    /// Timer Control Word Register (TCW).
    const TCW_PORT: u16 = 0x43;

    /// Counter 0 - Counter Access Ports Register (C0_CAPR).
    const C0_CAPR: u16 = 0x40;

    /// Initialize the Timer.
    pub fn init() {
        println!("Initializing PIT");

        unsafe {
            // Select a counter by writing a control word
            let mut tcw = Port::<u8>::new(Self::TCW_PORT);

            #[allow(clippy::inconsistent_digit_grouping)]
            {
                // 00 — Counter 0 select
                // 11 — Read/Write LSB then MSB
                // x10 — Rate generator (divide by n counter)
                // 0 — Binary countdown is used. The largest possible binary count is 2^16
                tcw.write(0b00_11_010_0);
            }

            // Wait some time for the timer to process
            delay(1_000_000);

            // Write an initial count for that counter
            let initial_count = (Self::COUNTER_FREQ / TIMER_FREQ) as u16;
            let [initial_count_lsb, initial_count_msb] = initial_count.to_le_bytes();
            let mut c0_capr = Port::<u8>::new(Self::C0_CAPR);
            c0_capr.write(initial_count_lsb);
            delay(1_000_000);
            c0_capr.write(initial_count_msb);
        }
    }
}

/// Blocks the program for *at least* `cycles`.
fn delay(cycles: u64) {
    let start = unsafe { _rdtsc() };
    atomic::fence(Ordering::SeqCst);
    while unsafe { _rdtsc() } - start < cycles {
        atomic::fence(Ordering::SeqCst);
    }
}

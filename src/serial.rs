use core::fmt;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// An interface to a serial port that allows sending out individual bytes.
pub struct SerialPort {
    data: Port<u8>,
}

impl SerialPort {
    const SERIAL1: Self = unsafe { Self::new(0x3F8) };

    /// Creates a new serial port interface on the given I/O port.
    ///
    /// This function is unsafe because the caller must ensure that the given base address
    /// really points to a serial port device.
    pub const unsafe fn new(base: u16) -> SerialPort {
        SerialPort {
            data: Port::new(base),
        }
    }

    /// Sends a byte on the serial port.
    pub fn send(&mut self, data: u8) {
        unsafe {
            match data {
                8 | 0x7F => {
                    self.data.write(8);
                    self.data.write(b' ');
                    self.data.write(8)
                }
                _ => {
                    self.data.write(data);
                }
            }
        }
    }

    /// Receives a byte on the serial port.
    pub fn receive(&mut self) -> u8 {
        unsafe {
            self.data.read()
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

pub static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::SERIAL1);

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        SERIAL1
            .lock()
            .write_fmt(args)
            .expect("Printing to serial failed");
    });
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}

#![no_main]
#![no_std]

// extern crate panic_halt;
extern crate panic_semihosting;

use core::cell::RefCell;

use boot::{Image, MappedFlash};
use cortex_m_rt::entry;

use embedded_hal::digital::v2::OutputPin;
use embedded_storage::nor_flash::{ErrorType, NorFlashError, NorFlashErrorKind, ReadNorFlash};
use hal::{drivers::pins::Level};
use lpc55_hal as hal;
// use embedded_time::rate::Extensions;

use cortex_m_semihosting::{hprintln};

#[entry]
fn main() -> ! {
    let hal = hal::new();

    hprintln!("Start of code");

    let pins = hal::Pins::take().unwrap();

    let mut syscon = hal.syscon;
    let mut gpio = hal.gpio.enabled(&mut syscon);
    let mut iocon = hal.iocon.enabled(&mut syscon);

    // For now, trying to initialize the clocks again in the target locks up the
    // system.  There is probably something that needs to be fixed in the hal.
    // For now, just run at our default slow clock.
    /*
    let mut anactrl = hal.anactrl;
    let mut pmc = hal.pmc;
    let clocks = hal::ClockRequirements::default()
        .system_frequency(50.MHz())
        .configure(&mut anactrl, &mut pmc, &mut syscon)
        .unwrap();
    let _ = clocks;
    */

    let mut red = pins
        .pio1_6
        .into_gpio_pin(&mut iocon, &mut gpio)
        .into_output(Level::High);

    let flash = InternalFlash {
        base: 0x20000,
        len: 0x20000,
    };
    let flash = RefCell::new(flash);

    let image = Image::from_flash(&flash).unwrap();
    image.validate().unwrap();
    chain(&image).unwrap();

    loop {
        red.set_low().unwrap();
        hal::wait_at_least(300_000);
        red.set_high().unwrap();
        hal::wait_at_least(300_000);
    }
}

/// Represents a memory-mapped simple flash partition.  This has no error
/// recovery.
pub struct InternalFlash {
    base: usize,
    len: usize,
}

#[derive(Debug)]
pub struct FlashError;

impl NorFlashError for FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        NorFlashErrorKind::Other
    }
}

impl ErrorType for InternalFlash {
    type Error = FlashError;
}

impl ReadNorFlash for InternalFlash {
    const READ_SIZE: usize = 1;

    fn capacity(&self) -> usize {
        self.len
    }

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> core::result::Result<(), FlashError> {
        let total = if let Some(t) = (offset as usize).checked_add(bytes.len()) {
            t
        } else {
            return Err(FlashError);
        };
        if total > self.len {
            return Err(FlashError);
        }

        // As long as the region we have is valid, there should be no overflow
        // checks.
        let base = self.base + offset as usize;

        let memory = unsafe {
            core::slice::from_raw_parts(base as *const u8, bytes.len())
        };

        bytes.copy_from_slice(memory);

        Ok(())
    }
}

impl MappedFlash for InternalFlash {
    fn get_base(&self) -> usize {
       self.base
    }
}

// Errors with image handling
#[derive(Debug)]
pub enum ImageError {
    Flash,
    Invalid,
}

impl From<FlashError> for ImageError {
    fn from(_value: FlashError) -> Self {
        ImageError::Flash
    }
}
/*
impl Into<ImageError> for FlashError {
    fn into(self) -> ImageError {
        ImageError::Flash
    }
}
*/

// For debugging.
// TODO: Handle the storage error, converting to a better error.
#[inline(never)]
pub fn chain<'f, F: MappedFlash>(image: &Image<'f, F>) -> Result<(), ImageError> {
    // Chain the next image, assuming the image has been validated.

    let reset_base = image.get_image_base();
    let reset = unsafe {&*(reset_base as *const ResetVector)};
    hprintln!("chain {:x?}", reset);
    unsafe {
        #[allow(unused_mut)]
        let mut p = cortex_m::Peripherals::steal();
        p.SCB.vtor.write(reset_base as u32);

        cortex_m::asm::bootload(reset_base as *const u32);
    }

    // Ok(())
}

// TODO: We don't really want to just read this directly, as it will fault if no
// image was written here. But, read without faulting is still WIP.

/// The Cortex-M reset vector, sits at the start of the vector table.
#[derive(Debug)]
#[repr(C)]
struct ResetVector {
    msp: u32,
    reset: u32,
}

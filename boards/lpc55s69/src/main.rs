#![no_main]
#![no_std]

// extern crate panic_halt;
extern crate panic_semihosting;

use asraw::{AsRaw, AsMutRaw};
use cortex_m_rt::entry;

use embedded_hal::digital::v2::OutputPin;
use embedded_storage::ReadStorage;
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

    let mut flash = InternalFlash {
        base: 0x20000,
        len: 0x20000,
    };
    chain(&mut flash).unwrap();

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

impl ReadStorage for InternalFlash {
    type Error = FlashError;

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

// The boot flash has to be mapped.
pub trait MappedFlash {
    fn get_base(&self) -> usize;
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
pub fn chain<F: ReadStorage<Error = FlashError> + MappedFlash>(flash: &mut F) -> Result<(), ImageError> {
    // TODO: use maybeuninit here?
    let mut header = ImageHeader::default();
    flash.read(0, header.as_mut_raw())?;
    hprintln!("Header: {:#x?}", header);
    if header.magic != IMAGE_MAGIC {
        hprintln!("Image magic not present.");
        return Err(ImageError::Invalid);
    }

    // TODO: Verify the header/TLV.

    // Get the base.
    let base = flash.get_base();

    let reset_base = base + header.hdr_size as usize;
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

const IMAGE_MAGIC: u32 = 0x96f3b83d;

#[derive(Debug, Default)]
#[repr(C)]
struct ImageVersion {
    major: u8,
    minor: u8,
    revision: u16,
    build_num: u32,
}

#[derive(Debug, Default)]
#[repr(C)]
struct ImageHeader {
    magic: u32,
    load_addr: u32,
    hdr_size: u16,
    protect_tlv_size: u16,
    img_size: u32,
    flags: u32,
    version: ImageVersion,
    pad1: u32,
}

impl AsRaw for ImageHeader {}
unsafe impl AsMutRaw for ImageHeader {}

/// Header in front of the tlv section.
#[allow(unused)]
#[repr(C)]
struct TlvInfo {
    magic: u16,
    // size of TLV area (inclusing this header).
    tlv_tot: u16,
}

/// In front of a single TLV entry.
#[allow(unused)]
#[repr(C)]
struct Tlv {
    kind: u16,
    len: u16,
}

/// The Cortex-M reset vector, sits at the start of the vector table.
#[derive(Debug)]
#[repr(C)]
struct ResetVector {
    msp: u32,
    reset: u32,
}

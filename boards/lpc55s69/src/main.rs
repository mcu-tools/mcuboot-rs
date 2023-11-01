#![no_main]
#![no_std]

#[cfg(not(any(feature = "semihosting",feature = "rtt")))]
extern crate panic_halt;
#[cfg(feature = "semihosting")]
extern crate panic_semihosting;
#[cfg(feature = "rtt")]
use panic_probe as _;

#[cfg(feature = "rtt")]
use defmt_rtt as _;

use core::cell::RefCell;

use boot::{Image, MappedFlash};
use cortex_m_rt::entry;

use embedded_hal::{digital::v2::OutputPin, timer::CountDown};
use hal::{drivers::{pins::Level, Timer, timer::Elapsed}, peripherals::ctimer::Ctimer, Enabled};
use lpc55_hal as hal;
use embedded_time::rate::Extensions;
use embedded_time::duration::Extensions as DurationExtensions;
use embedded_time::duration::Microseconds;
use embedded_time::fixed_point::FixedPoint;

#[cfg(feature = "semihosting")]
mod logging {
    pub use cortex_m_semihosting::{hprintln};
}

mod flash;

// Use 'info' if we are using defmt.
#[cfg(feature = "rtt")]
mod logging {
    macro_rules! hprintln {
        ($e:expr) => {
            defmt::error!($e);
        };
        ($e:expr, $($args:expr),+) => {
            defmt::error!($e, $($args),+);
        };
    }
    pub(crate) use hprintln;
}

// If semihosting is not available, just discard printed messages.  It also
// "uses" the arguments so disabling printing doesn't cause additional warnings.
#[cfg(not(any(feature = "semihosting",feature = "rtt")))]
mod logging {
    macro_rules! hprintln {
        ($_e:expr) => {{}};
        ($_e:expr, $($x:expr),+) => {
            $(let _ = $x;);+
        };
    }
    pub(crate) use hprintln;
}

pub(crate) use logging::hprintln;

#[entry]
fn main() -> ! {
    let hal = hal::new();

    hprintln!("---------- Start of code ----------");

    let pins = hal::Pins::take().unwrap();

    let mut syscon = hal.syscon;
    let mut gpio = hal.gpio.enabled(&mut syscon);
    let mut iocon = hal.iocon.enabled(&mut syscon);
    // let mut scb = hal.SCB;
    // let mut cpuid = hal.CPUID;

    /*
    scb.enable_icache();
    scb.enable_dcache(&mut cpuid);

    let flash = hal.flash.release();
    let bits = wait_done(&flash);
    hprintln!("wait_done status: {:x}", bits);
    hprintln!("Check 0 {:?}", read_check(&flash, 0));
    hprintln!("Check 20000 {:?}", read_check(&flash, 0x20000));
    hprintln!("Check 40000 {:?}", read_check(&flash, 0x40000));
    */

    /*
    let st = flash.int_status.read();
    if st.done().bit_is_set() {
    hprintln!("Read done");
} else {
    hprintln!("Read not done");
}
     */

    // Read the status register to make sure it works.

    // For now, trying to initialize the clocks again in the target locks up the
    // system.  There is probably something that needs to be fixed in the hal.
    // For now, just run at our default slow clock.
    let mut anactrl = hal.anactrl;
    let mut pmc = hal.pmc;
    let clocks = hal::ClockRequirements::default()
        .system_frequency(100.MHz())
        .configure(&mut anactrl, &mut pmc, &mut syscon)
        .unwrap();
    let _ = clocks;

    // Try using the timer to determine how long some of these things take.
    let ctimer = hal
        .ctimer
        .1
        .enabled(&mut syscon, clocks.support_1mhz_fro_token().unwrap());
    let mut cdriver = Timer::new(ctimer);

    /*
    for addr in [0, 0x20000, 0x40000] {
        let (ok, elapsed) = measure(&mut cdriver, || {
            (0..1000).map(|_| read_check(&flash, addr)).last().unwrap()
        });
        hprintln!("Check 0x{:x} {:?} {}us", addr, ok, elapsed);
    }

    // There is an image there, read it to get it into the cache.
    hprintln!("@20000->{:>8x}",
              unsafe {
                  *(0x20000 as *const u32)
              });

    // Erase at 0x20000.
    let (ok, elapsed) = measure(&mut cdriver, || erase(&flash, 0x20000, 512));
    hprintln!("Erase 0x20000 {:?} {}us", ok, elapsed);

    // Recheck.
    let (ok, elapsed) = measure(&mut cdriver, || read_check(&flash, 0x20000));
    hprintln!("Read check 0x20000 {:?} {}us", ok, elapsed);

    // Program a test pattern.
    let mut pattern = [0u8; 512];
    for i in 0..512 {
        pattern[i] = (i & 0xff) as u8;
    }
    let (ok, elapsed) = measure(&mut cdriver, || program_page(&flash, 0x20000, &pattern));
    hprintln!("Program 0x20000 {:?} {}us", ok, elapsed);

    // Invalidate the caches.
    /*
    scb.invalidate_icache();
    unsafe {
        scb.invalidate_dcache_by_address(0x20000, 512);
    }
    */

    // Print out some, to see.
    for i in 0..32 {
        hprintln!("{:>08x}",
                  unsafe {
                      *((0x20000 + i * 4) as *const u32)
                  }
        );
    }

    /*
    cdriver.start(1_000_000.microseconds());
    let vvv = read_check(&flash, 0);
    let now = cdriver.elapsed();
    // hprintln!("Check 0 {:?}", read_check(&flash, 0));
    hprintln!("Check 0 {:?} {}", vvv, now);
    hprintln!("Check 20000 {:?}", read_check(&flash, 0x20000));
    hprintln!("Check 40000 {:?}", read_check(&flash, 0x40000));
    */
    */

    let mut red = pins
        .pio1_6
        .into_gpio_pin(&mut iocon, &mut gpio)
        .into_output(Level::High);

    let flash = hal.flash.release();
    let flash = flash::LpcFlash::new(flash);
    let slot0 = flash.partition(0x20000, 0x20000).unwrap();

    let slot0 = RefCell::new(slot0);

    let image = Image::from_flash(&slot0).unwrap();
    let ((), elapsed) = measure(&mut cdriver, || image.validate().unwrap());
    hprintln!("validate: {}us", elapsed.integer());
    chain(&image).unwrap();

    loop {
        red.set_low().unwrap();
        hal::wait_at_least(300_000);
        red.set_high().unwrap();
        hal::wait_at_least(300_000);
    }
}

fn measure<T, TT: Ctimer<Enabled>, F: FnOnce() -> T>(timer: &mut Timer<TT>, action: F) -> (T, Microseconds) {
    timer.start(1_000_000.microseconds());
    let before = timer.elapsed();
    let result = action();
    let after = timer.elapsed();
    (result, after - before)
}

/*
// Try putting some code into RAM, and see if we can execute it there. In this
// case, we want to try accessing hardware.
#[inline(never)]
#[link_section = ".data.flash"]
fn wait_done(flash: &FLASH) -> u32 {
    while flash.int_status.read().done().bit_is_clear() {
    }
    let bits = flash.int_status.read().bits();
    flash.int_clr_status.write(|w| w.done().set_bit().err().set_bit().fail().set_bit().ecc_err().set_bit());
    bits
}
*/

/*
/// Determine if a page has been programmed. If this returns true, it is likely
/// that reads from that page will not result in bus faults.
#[inline(never)]
//#[link_section = ".data.flash"]
fn read_check(flash: &FLASH, addr: u32) -> bool {
    // Wait for anything to complete, and clear status.
    /*
    while flash.int_status.read().done().bit_is_clear() {
    }
    */
    flash.int_clr_status.write(|w| w.done().set_bit().err().set_bit().fail().set_bit().ecc_err().set_bit());

    flash.starta.write(|w| unsafe{w.bits(addr >> 4)});
    flash.stopa.write(|w| unsafe{w.bits(addr >> 4)});
    flash.cmd.write(|w| unsafe{w.bits(6)});
    while flash.int_status.read().done().bit_is_clear() {
    }

    let good = flash.int_status.read().fail().bit_is_clear();

    flash.int_clr_status.write(|w| w.done().set_bit().err().set_bit().fail().set_bit().ecc_err().set_bit());

    good
}
*/

/*
/// Erase a range.
fn erase(flash: &FLASH, base: u32, length: u32) -> bool {
    flash.int_clr_status.write(|w| w.done().set_bit().err().set_bit().fail().set_bit().ecc_err().set_bit());
    if length % 512 != 0 {
        return false;
    }
    let ending = base + length - 511;
    flash.starta.write(|w| unsafe{w.bits(base >> 4)});
    flash.stopa.write(|w| unsafe{w.bits(ending >> 4)});
    flash.cmd.write(|w| unsafe{w.bits(4)});
    while flash.int_status.read().done().bit_is_clear() {
    }

    let good = flash.int_status.read().fail().bit_is_clear();

    flash.int_clr_status.write(|w| w.done().set_bit().err().set_bit().fail().set_bit().ecc_err().set_bit());

    good
}
*/

/*
/// Program a single page.
fn program_page(flash: &FLASH, base: u32, page: &[u8]) -> bool {
    flash.int_clr_status.write(|w| w.done().set_bit().err().set_bit().fail().set_bit().ecc_err().set_bit());
    if page.len() != 512 {
        return false;
    }
    for word in 0..32 {
        flash.starta.write(|w| unsafe{w.bits(word)});
        for column in 0..4 {
            let base = ((word << 4) + (column << 2)) as usize;
            let cell = LittleEndian::read_u32(&page[base..base+4]);
            flash.dataw[column as usize].write(|w| unsafe{w.bits(cell)});
        }
        flash.cmd.write(|w| unsafe{w.bits(8)});
        while flash.int_status.read().done().bit_is_clear() {
        }
        let good = flash.int_status.read().fail().bit_is_clear();
        if !good {
            return false;
        }
    }

    flash.starta.write(|w| unsafe{w.bits(base >> 4)});
    flash.cmd.write(|w| unsafe{w.bits(12)});
    while flash.int_status.read().done().bit_is_clear() {
    }
    let good = flash.int_status.read().fail().bit_is_clear();
    good
}
*/

// For debugging.
// TODO: Handle the storage error, converting to a better error.
#[inline(never)]
pub fn chain<'f, F: MappedFlash>(image: &Image<'f, F>) -> Result<(), flash::Error> {
    // Chain the next image, assuming the image has been validated.

    let reset_base = image.get_image_base();
    // let reset = unsafe {&*(reset_base as *const ResetVector)};
    // hprintln!("chain {}", reset);
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

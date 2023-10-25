#![no_main]
#![no_std]

// extern crate panic_halt;
extern crate panic_semihosting;

use cortex_m_rt::entry;

use embedded_hal::digital::v2::OutputPin;
use hal::{drivers::pins::Level};
use lpc55_hal as hal;
use embedded_time::rate::Extensions;

use cortex_m_semihosting::{hprintln};

#[entry]
fn main() -> ! {
    let hal = hal::new();

    hprintln!("Start of code");

    let pins = hal::Pins::take().unwrap();

    let mut anactrl = hal.anactrl;
    let mut pmc = hal.pmc;
    let mut syscon = hal.syscon;
    let mut gpio = hal.gpio.enabled(&mut syscon);
    let mut iocon = hal.iocon.enabled(&mut syscon);

    let clocks = hal::ClockRequirements::default()
        .system_frequency(50.MHz())
        .configure(&mut anactrl, &mut pmc, &mut syscon)
        .unwrap();
    let _ = clocks;

    let mut red = pins
        .pio1_6
        .into_gpio_pin(&mut iocon, &mut gpio)
        .into_output(Level::High);

    loop {
        red.set_low().unwrap();
        hal::wait_at_least(300_000);
        red.set_high().unwrap();
        hal::wait_at_least(300_000);
    }
}

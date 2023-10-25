#![no_main]
#![no_std]

// extern crate panic_halt;
extern crate panic_semihosting;

use cortex_m_rt::{entry, exception, ExceptionFrame};

use embedded_hal::digital::v2::OutputPin;
use hal::drivers::pins::Level;
use lpc55_hal as hal;
use embedded_time::rate::Extensions;

use cortex_m_semihosting::{hprintln};

use core::arch::asm;

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

    // show(0x20000);
    show(0x40000);

    loop {
        red.set_low().unwrap();
        hal::wait_at_least(300_000);
        red.set_high().unwrap();
        hal::wait_at_least(300_000);
    }
}

/// Marker.  This is indeed unsafe, but we only access from the safe operation, and from our fault handler.
#[derive(Clone, Copy)]
enum FaultState {
    None,
    Trying,
    Failed,
}

static mut FAULT_STATE: FaultState = FaultState::None;

/// Safely read from a given location.
#[inline(never)]
fn safe_read(addr: usize) -> Option<u32> {
    let ptr = unsafe {&*(addr as *const u32)};
    unsafe { FAULT_STATE = FaultState::Trying };
    let value = *ptr;
    match unsafe {FAULT_STATE} {
        FaultState::None => panic!("This shouldn't happen"),
        FaultState::Trying => {
            unsafe {FAULT_STATE = FaultState::None};
            Some(value)
        }
        FaultState::Failed => {
            unsafe {FAULT_STATE = FaultState::None};
            None
        }
    }
}

fn show(addr: usize) {
    hprintln!("Code at 0x{:x} = 0x{:x?}", addr, safe_read(addr));
}

// Hard fault handler.  This will happen with the unexpected access.
#[exception]
unsafe fn HardFault(exn: &ExceptionFrame) -> ! {
    match FAULT_STATE {
        FaultState::None => (),
        FaultState::Trying => {
            FAULT_STATE = FaultState::Failed;
            // Return back to the user's code.

            // Return two past, so avoid retrying this particular instruction,
            // which we "hope" was a 16-bit instruction.
            // let pc = exn.pc + 2;

            // This doesn't work, as we don't have any easy to way to get back
            // to the unwound stack our trampoline created for us. This might be
            // doable with a newer cortex-m-rt that would need lpc55-hal to be
            // made to work with.
            asm!(
                // "/* {r0} */",
                "/* TODO: How much to unwinde the stack? */",
                "bx {pc}",
                in("r0") exn.r0,
                in("r1") exn.r1,
                in("r2") exn.r2,
                in("r3") exn.r3,
                in("r12") exn.r12,
                in("lr") exn.lr,
                pc = in(reg) exn.pc,
                // lr = in(reg) exn.lr,
                );
        }
        FaultState::Failed => (),
    }
    hprintln!("\nFault!: {:#x?}", exn);
    loop {
    }
}

// Can this return?
#[exception]
fn DefaultHandler(_ex: i16) {
}

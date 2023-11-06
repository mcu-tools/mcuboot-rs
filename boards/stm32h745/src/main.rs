#![no_main]
#![no_std]

// use panic_semihosting as _;
use panic_probe as _;
use defmt_rtt as _;
use defmt::{warn, info};

use hal::{rcc::PllConfigStrategy, flash::UnlockedFlashBank};
use hal::pac;
use hal::flash::FlashExt;
use hal::gpio::GpioExt;
use hal::pwr::PwrExt;
use hal::rcc::RccExt;
use fugit::RateExtU32;
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};

use stm32h7xx_hal as hal;

#[cortex_m_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // - power & clocks -------------------------------------------------------

    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.smps().vos0(&dp.SYSCFG).freeze();
    let ccdr = dp
        .RCC
        .constrain()
        .pll1_strategy(PllConfigStrategy::Iterative) // pll1 drives system clock
        .sys_ck(480.MHz()) // system clock @ 480 MHz
        .freeze(pwrcfg, &dp.SYSCFG);

    // - pins -----------------------------------------------------------------

    let gpiob = dp.GPIOB.split(ccdr.peripheral.GPIOB);
    let mut led_user = gpiob.pb14.into_push_pull_output();
    led_user.set_low();

    warn!("Running");
    info!("Dual bank? {}", dp.FLASH.dual_bank());
    // - Flash memory.
    let (mut flash1, mut flash2) = dp.FLASH.split();
    info!("Capacity1: 0x{:x}", flash1.len());
    // info!("Capacity2: 0x{:x}", flash2.len());

    if let Some(ref mut f2) = flash2 {
        info!("Capacity2: 0x{:x}", f2.len());
    }

    // Erase a ways in on bank 2.
    {
        let mut f = flash1.unlocked();
        let esize = <UnlockedFlashBank as NorFlash>::ERASE_SIZE;
        let wsize = <UnlockedFlashBank as NorFlash>::WRITE_SIZE;
        info!("Erase size: {}", esize);
        info!("Write size: {}", wsize);
        f.erase(0x40000, 0x40000).unwrap();

        // Write a boring pattern.
        let mut buf = [0u8; 128];
        for i in 0..128 {
            buf[i as usize] = i;
        }
        f.write(0x40000, &buf).unwrap();
    }
    info!("Address of flash: 0x{:x}", flash1.address());

    // Fault.
    // let _ = unsafe { *(0x0040_0000 as *const u32) };

    // - main loop ------------------------------------------------------------

    loop {
        loop {
            led_user.toggle();
            for _ in 0..1 {
                // cortex_m::asm::delay(480_000_000);
                cortex_m::asm::delay(100_000_000);
            }
        }
    }
}

// Make this a little easier to debug.
#[inline(never)]
fn show() {
    warn!("Show what is happening");
}

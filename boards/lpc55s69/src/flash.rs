//! LPC55S6x flash driver.
//!
//! Replacement flash driver from the one in the hal.  This attempts to do a few
//! basic things:
//!
//! - Implement embedded_storage::nor_flash::ReadNorFlash and NorFlash
//!   interfaces
//! - Implement a robust read that will return an error instead of busfaulting
//!   on unprogrammed data.
//!
//! To use this driver, you should release the FLASH PAC from the hal's driver.
//!
//!     let flash = hal.flash.release();
//!     let fl = flash::LpcFlash::new(flash);

use core::cell::RefCell;

use boot::MappedFlash;
use embedded_storage::nor_flash::{NorFlashErrorKind, ErrorType, ReadNorFlash};
use hal::raw::FLASH;
use lpc55_hal as hal;

#[cfg(feature = "semihosting")]
use cortex_m_semihosting::{hprintln};

// It works for us to just use the NorFlashError directly.
type Result<T> = core::result::Result<T, NorFlashErrorKind>;

pub struct LpcFlash {
    raw: RefCell<hal::raw::FLASH>,
}

const LPC_FLASH_BASE: usize = 0;
const LPC_FLASH_SIZE: usize = 630 * 1024;

// Flash for the entire device.
impl LpcFlash {
    pub fn new(raw: hal::raw::FLASH) -> LpcFlash {
        LpcFlash { raw: RefCell::new(raw) }
    }

    pub fn partition(&self, base: usize, length: usize) -> Result<LpcPartition> {
        LpcPartition::new(self, base, length)
    }
}

// A single flash partition.  References the parent.
pub struct LpcPartition<'a> {
    flash: &'a LpcFlash,
    base: usize,
    length: usize,
}

impl<'a> LpcPartition<'a> {
    pub fn new(flash: &'a LpcFlash, base: usize, length: usize) -> Result<Self> {
        if length == 0 {
            return Err(NorFlashErrorKind::OutOfBounds);
        }
        // This wouldn't be right if the flash was at the end of the address
        // space. But as such, it does prevent overflow.  It is safe to subtract
        // one because we checked that above.
        let end = match base.checked_add(length) {
            Some(e) => e - 1,
            None => return Err(NorFlashErrorKind::OutOfBounds),
        };
        // No overflow check, as these are consts.
        let self_range = LPC_FLASH_BASE .. LPC_FLASH_BASE + LPC_FLASH_SIZE;
        if !(self_range.contains(&base) && self_range.contains(&end)) {
            return Err(NorFlashErrorKind::OutOfBounds);
        }

        Ok(LpcPartition { flash, base, length })
    }
}

impl<'a> ErrorType for LpcPartition<'a> {
    type Error = NorFlashErrorKind;
}

impl<'a> LpcPartition<'a> {
    fn valid_bound(&self, offset: usize, length: usize) -> bool {
        if length == 0 {
            // Note that we don't check the offset for 0 byte reads.
            return true;
        }

        // Won't overflow as doesn't reach end of address space.
        let self_range = self.base .. self.base + self.length - 1;
        let end = offset + (length - 1);
        self_range.contains(&offset) && self_range.contains(&end)
    }
}

impl<'a> ReadNorFlash for LpcPartition<'a> {
    // We allow arbitrary alignment of reads.
    const READ_SIZE: usize = 1;
    fn capacity(&self) -> usize {
        self.length
    }

    fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<()> {
        let offset = offset as usize;

        let offset = offset.checked_add(self.base).ok_or(NorFlashErrorKind::OutOfBounds)?;

        if !self.valid_bound(offset, buf.len()) {
            hprintln!("Out of bounds: 0x{:x} len:0x{:x}", offset, buf.len());
            return Err(NorFlashErrorKind::OutOfBounds);
        }

        // Validate that the entire range has been written.
        let end = offset + buf.len();
        let mut bpage = offset & !511;
        while bpage < end {
            // hprintln!("Read check: 0x{:x}", bpage);
            if !read_check(&self.flash.raw.borrow(), bpage as u32) {
                // Indicate read error with Other
                return Err(NorFlashErrorKind::Other);
            }
            bpage += 512;
        }

        // Copy the data.
        let slice = unsafe {
            core::slice::from_raw_parts(offset as *const u8, buf.len())
        };
        buf.copy_from_slice(slice);

        Ok(())
    }
}

impl<'a> MappedFlash for LpcPartition<'a> {
    fn get_base(&self) -> usize {
        LPC_FLASH_BASE + self.base
    }
}

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

//! Simulated flash
//!
//! The NOR-type flashes used in microcontrollers differs quite a bit in terms
//! of capabilities provided.  This simulator attempts to capture the diversity
//! of these devices.
//!
//! We make these simulated flash devices available via the embedded-flash
//! traits, at least some of the NOR-specific ones, notably ReadNorFlash and
//! NorFlash.  Most of the rest of the traits are fairly useless as they tend to
//! abstract the funcionality of the device and make it impossible to use the
//! devices robustly.
//!
//! The NorFlash defines a READ_SIZE, an ERASE_SIZE, and a WRITE_SIZE.  We
//! require that the erase size be a multiple of the WRITE_SIZE (they can be the
//! same).  At this point in time, the READ_SIZE is always 1.  There are a
//! couple of different families of devices that are common:
//!
//! - Old style: ERASE_SIZE is 4k-128k, WRITE_SIZE is typically 1-8, sometimes
//!   as much as 16 or 32, although these might need to be considered a different
//!   class of device.
//! - Large write: ERASE_SIZE is 128k, WRITE_SIZE is 32.  Large to write, but
//!   also large erase sizes.  Might be best handled as above.
//! - Paged: ERASE_SIZE is 512, WRITE_SIZE is 512.  The write size is much
//!   larger than thye others, but the smaller erases allow us to treat the device
//!   more like blocks.

use embedded_storage::nor_flash::{ReadNorFlash, ErrorType, NorFlashError, NorFlashErrorKind, NorFlash, self};

/// The richer error type used in the simulator.
#[derive(Debug, Clone, Copy)]
pub enum SimError {
    Inner(NorFlashErrorKind),
}

impl From<NorFlashErrorKind> for SimError {
    fn from(inner: NorFlashErrorKind) -> Self {
        SimError::Inner(inner)
    }
}

type Result<T> = core::result::Result<T, SimError>;

struct SimFlash<const WRITE_SIZE: usize, const ERASE_SIZE: usize> {
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> SimFlash<WRITE_SIZE, ERASE_SIZE> {
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> ErrorType for SimFlash<WRITE_SIZE, ERASE_SIZE> {
    type Error = SimError;
}

impl NorFlashError for SimError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            SimError::Inner(inner) => *inner,
            // SimError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
        }
    }
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> ReadNorFlash for SimFlash<WRITE_SIZE, ERASE_SIZE> {
    const READ_SIZE: usize = 1;
    fn capacity(&self) -> usize {
        todo!()
    }
    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<()> {
        nor_flash::check_read(self, offset, bytes.len())?;
        todo!()
    }
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> NorFlash for SimFlash<WRITE_SIZE, ERASE_SIZE> {
    const WRITE_SIZE: usize = WRITE_SIZE;
    const ERASE_SIZE: usize = ERASE_SIZE;
    fn erase(&mut self, from: u32, to: u32) -> Result<()> {
        nor_flash::check_erase(self, from, to)?;
        todo!()
    }
    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
        nor_flash::check_write(self, offset, bytes.len())?;
        todo!()
    }
}

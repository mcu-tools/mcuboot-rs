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

use std::ops::Range;

use embedded_storage::nor_flash::{
    self, ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

/// The richer error type used in the simulator.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SimError {
    Inner(NorFlashErrorKind),
    Unwritten,
    NotErased,
}

impl From<NorFlashErrorKind> for SimError {
    fn from(inner: NorFlashErrorKind) -> Self {
        SimError::Inner(inner)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PageState {
    Erased,
    Written,
    Unknown,
}

type Result<T> = core::result::Result<T, SimError>;

pub struct SimFlash<const WRITE_SIZE: usize, const ERASE_SIZE: usize> {
    data: Vec<u8>,
    page_state: Vec<PageState,>
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> SimFlash<WRITE_SIZE, ERASE_SIZE> {
    // Some terminology:
    // - Page - the unit written
    // - Sector - the unit erased

    const PAGES_PER_SECTOR: usize = ERASE_SIZE / WRITE_SIZE;

    /// Create a new simulated flash device.  The size will be based on the
    /// given number of pages.
    pub fn new(sectors: usize) -> Result<Self> {
        // TODO: Ideally, these would be checked at compile time.
        assert!(WRITE_SIZE <= ERASE_SIZE);
        assert!(ERASE_SIZE % WRITE_SIZE == 0);

        let page_state = vec![PageState::Unknown; sectors * Self::PAGES_PER_SECTOR];
        let data = vec![0xff; sectors * ERASE_SIZE];
        Ok(SimFlash {data, page_state})
    }

    /// Given a byte value, return what page contains that byte.
    fn page_of(&self, offset: usize) -> usize {
        offset / WRITE_SIZE
    }

    /// Given a 'from' and 'to' value in bytes (a range), return a range over
    /// the page affected.
    fn pages(&self, from: usize, to: usize) -> Range<usize> {
        self.page_of(from) .. self.page_of(to - 1) + 1
    }
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> ErrorType
    for SimFlash<WRITE_SIZE, ERASE_SIZE>
{
    type Error = SimError;
}

impl NorFlashError for SimError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            SimError::Inner(inner) => *inner,
            SimError::Unwritten => NorFlashErrorKind::Other,
            SimError::NotErased => NorFlashErrorKind::Other,
            // SimError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
        }
    }
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> ReadNorFlash
    for SimFlash<WRITE_SIZE, ERASE_SIZE>
{
    const READ_SIZE: usize = 1;
    fn capacity(&self) -> usize {
        self.data.len()
    }

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<()> {
        nor_flash::check_read(self, offset, bytes.len())?;
        let offset = offset as usize;

        for i in self.pages(offset, offset + bytes.len()) {
            if self.page_state[i] != PageState::Written {
                return Err(SimError::Unwritten);
            }
        }

        bytes.copy_from_slice(&self.data[offset .. offset + bytes.len()]);
        Ok(())
    }
}

impl<const WRITE_SIZE: usize, const ERASE_SIZE: usize> NorFlash
    for SimFlash<WRITE_SIZE, ERASE_SIZE>
{
    const WRITE_SIZE: usize = WRITE_SIZE;
    const ERASE_SIZE: usize = ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<()> {
        nor_flash::check_erase(self, from, to)?;

        for i in self.pages(from as usize, to as usize) {
            self.page_state[i] = PageState::Erased;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
        nor_flash::check_write(self, offset, bytes.len())?;
        let offset = offset as usize;

        for i in self.pages(offset, offset + bytes.len()) {
            if self.page_state[i] != PageState::Erased {
                return Err(SimError::NotErased);
            }
        }

        for i in self.pages(offset, offset + bytes.len()) {
            self.page_state[i] = PageState::Written;
        }

        self.data[offset .. offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
}

#[test]
fn test_simflash() {
    let mut f1 = SimFlash::<32, {128*1024}>::new(6).unwrap();
    let mut buf = [0u8; 256];
    assert_eq!(f1.capacity(), 6*128*1024);
    assert_eq!(f1.read(0, &mut buf), Err(SimError::Unwritten));
    assert_eq!(f1.erase(128*1024, 256*1024), Ok(()));
    assert_eq!(f1.write(128*1024, &mut buf), Ok(()));

    buf.fill(0x42);
    assert_eq!(f1.read(128*1024, &mut buf), Ok(()));
}

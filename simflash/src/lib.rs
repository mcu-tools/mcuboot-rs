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

use storage::{
    Error, Flash, ReadFlash, Result,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PageState {
    Erased,
    Written,
    Unknown,
}

pub struct SimFlash {
    read_size: usize,
    write_size: usize,
    erase_size: usize,
    data: Vec<u8>,
    page_state: Vec<PageState,>
}

impl SimFlash {
    // Some terminology:
    // - Page - the unit written
    // - Sector - the unit erased

    fn pages_per_sector(&self) -> usize {
        self.erase_size / self.write_size
    }

    /// Create a new simulated flash device.  The size will be based on the
    /// given number of pages.
    pub fn new(read_size: usize, write_size: usize, erase_size: usize, sectors: usize) -> Result<Self> {
        // TODO: Ideally, these would be checked at compile time.
        assert!(write_size <= erase_size);
        assert!(erase_size % write_size == 0);

        let pages_per_sector = erase_size / write_size;

        let page_state = vec![PageState::Unknown; sectors * pages_per_sector];
        let data = vec![0xff; sectors * erase_size];
        Ok(SimFlash {read_size, write_size, erase_size, data, page_state})
    }

    /// Given a byte value, return what page contains that byte.
    fn page_of(&self, offset: usize) -> usize {
        offset / self.write_size
    }

    /// Given a 'from' and 'to' value in bytes (a range), return a range over
    /// the page affected.
    fn pages(&self, from: usize, to: usize) -> Range<usize> {
        self.page_of(from) .. self.page_of(to - 1) + 1
    }

    /// Install a given image into the flash at the given offset.  For now, the
    /// offset must be aligned.
    pub fn install(&mut self, bytes: &[u8], offset: usize) -> Result<()> {
        // Set this to past the device, so that we will always try erasing.
        assert_eq!(offset as usize % self.erase_size, 0);

        let mut last_erased = self.page_state.len() / self.pages_per_sector();
        let mut pos = 0;
        let mut buf = vec![0u8; self.write_size];
        while pos < bytes.len() {
            let dev_pos = pos + offset as usize;
            let dev_sector = dev_pos / self.erase_size;
            if dev_sector != last_erased {
                self.erase(dev_sector * self.erase_size,
                           dev_sector * self.erase_size + 1)?;
                last_erased = dev_sector;
            }

            let len = self.write_size.min(bytes.len() - pos);
            buf.fill(0xff);
            buf[..len].copy_from_slice(&bytes[pos .. pos + len]);
            self.write(dev_pos, &buf)?;

            pos += self.write_size;
        }
        Ok(())
    }
}

impl ReadFlash for SimFlash {
    fn read_size(&self) -> usize {
        self.read_size
    }

    fn capacity(&self) -> usize {
        self.data.len()
    }

    fn read(&mut self, offset: usize, bytes: &mut [u8]) -> Result<()> {
        storage::check_read(self, offset, bytes.len())?;
        let offset = offset as usize;

        for i in self.pages(offset, offset + bytes.len()) {
            if self.page_state[i] != PageState::Written {
                return Err(Error::NotWritten);
            }
        }

        bytes.copy_from_slice(&self.data[offset .. offset + bytes.len()]);
        Ok(())
    }
}

impl Flash for SimFlash {
    fn write_size(&self) -> usize {
        self.write_size
    }

    fn erase_size(&self) -> usize {
        self.erase_size
    }

    fn erase(&mut self, from: usize, to: usize) -> Result<()> {
        storage::check_erase(self, from, to)?;

        for i in self.pages(from as usize, to as usize) {
            self.page_state[i] = PageState::Erased;
        }
        Ok(())
    }

    fn write(&mut self, offset: usize, bytes: &[u8]) -> Result<()> {
        storage::check_write(self, offset, bytes.len())?;
        let offset = offset as usize;

        for i in self.pages(offset, offset + bytes.len()) {
            if self.page_state[i] != PageState::Erased {
                return Err(Error::NotErased);
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
    let mut f1 = SimFlash::new(1, 32, 128*1024, 6).unwrap();
    let mut buf = [0u8; 256];
    assert_eq!(f1.capacity(), 6*128*1024);
    assert_eq!(f1.read(0, &mut buf), Err(Error::NotWritten));
    assert_eq!(f1.erase(128*1024, 256*1024), Ok(()));
    assert_eq!(f1.write(128*1024, &mut buf), Ok(()));

    buf.fill(0x42);
    assert_eq!(f1.read(128*1024, &mut buf), Ok(()));
}

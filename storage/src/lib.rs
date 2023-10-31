//! Storage types.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

// TODO: Do we want to use errors?

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Error {
    NotAligned,
    OutOfBounds,
    NotWritten,
    NotErased,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Read only interface into flash.
pub trait ReadFlash {
    /// What is the read size (alignment and size multiple).
    fn read_size(&self) -> usize;
    fn read(&mut self, offset: usize, bytes: &mut [u8]) -> Result<()>;
    fn capacity(&self) -> usize;
}

/// Flash that can be written to.
pub trait Flash: ReadFlash {
    /// Write size (alignment and size multiple).
    fn write_size(&self) -> usize;
    /// Erase size (alignment and size multiple).
    fn erase_size(&self) -> usize;

    fn erase(&mut self, from: usize, to: usize) -> Result<()>;
    fn write(&mut self, offset: usize, bytes: &[u8]) -> Result<()>;
}

// Utilities taken from embedded-storage for validating arguments.
pub fn check_read<T: ReadFlash>(
    flash: &T,
    offset: usize,
    length: usize,
) -> Result<()> {
    check_slice(flash, flash.read_size(), offset, length)
}

pub fn check_erase<T: Flash>(
    flash: &T,
    from: usize,
    to: usize,
) -> Result<()> {
    if from > to || to > flash.capacity() {
        return Err(Error::OutOfBounds);
    }
    if from % flash.erase_size() != 0 || to % flash.erase_size() != 0 {
        return Err(Error::NotAligned);
    }
    Ok(())
}

pub fn check_write<T: Flash>(
    flash: &T,
    offset: usize,
    length: usize,
) -> Result<()> {
    check_slice(flash, flash.write_size(), offset, length)
}

pub fn check_slice<T: ReadFlash>(
    flash: &T,
    align: usize,
    offset: usize,
    length: usize,
) -> Result<()> {
    if length > flash.capacity() || offset > flash.capacity() - length {
        return Err(Error::OutOfBounds);
    }
    if offset % align != 0 || length % align != 0 {
        return Err(Error::NotAligned);
    }
    Ok(())
}

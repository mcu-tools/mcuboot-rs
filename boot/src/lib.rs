//! This is a basic bootloader built for embedded rust.
//!
//! It supports various features.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

mod image;
mod status;

pub use image::Image;
pub use status::SlotInfo;

type Result<T> = core::result::Result<T, Error>;

// Use the error kind to avoid this depending on the particular flash.
#[derive(Debug)]
pub enum Error {
    Flash(storage::Error),
    InvalidImage,
    CannotUpgrade,
}

/// Convert the nor flash error into our error type.
impl From<storage::Error> for Error {
    fn from(e: storage::Error) -> Self {
        Error::Flash(e)
    }
}

/// Some kinds of flash can be mapped into memory.  This is needed for XIP devices.
pub trait MappedFlash {
    /// Return the base address of this flash partition, as mapped into memory.
    fn get_base(&self) -> usize;
}

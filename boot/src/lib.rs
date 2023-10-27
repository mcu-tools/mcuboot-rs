//! This is a basic bootloader built for embedded rust.
//!
//! It supports various features.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

mod image;

pub use image::Image;

/// Some kinds of flash can be mapped into memory.  This is needed for XIP devices.
pub trait MappedFlash {
    /// Return the base address of this flash partition, as mapped into memory.
    fn get_base(&self) -> usize;
}

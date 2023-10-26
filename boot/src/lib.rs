//! This is a basic bootloader built for embedded rust.
//!
//! It supports various features.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

mod image;

pub use image::Image;

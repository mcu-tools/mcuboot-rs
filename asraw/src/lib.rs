//! AsRaw provides a safe way to view a structure as its bytes, and an unsafe
//! way to have this as a mutable view.  Generally, this is safe and meaningful
//! for structures that are repr(C).  `as_mut_raw` is only safe in this case.

#![cfg_attr(not(any(feature = "std", test)), no_std)]

use core::{mem, slice};

pub trait AsRaw : Sized {
    fn as_raw(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const _ as *const u8,
                                  mem::size_of::<Self>())
        }
    }
}

/// Provide a view into a structure.  This is an unsafe trait, because, in
/// general, it isn't safe to interpret arbitrary bytes as another type.
/// However, if the struct is `repr(C)`, and all types used are valid for all
/// possible values, this will be safe.
pub unsafe trait AsMutRaw : Sized {
    fn as_mut_raw(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut _ as *mut u8,
                                      mem::size_of::<Self>())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Eq, PartialEq)]
    #[repr(C)]
    struct Item {
        a: u32,
        b: u8,
        c: u16,
    }

    impl AsRaw for Item {}
    unsafe impl AsMutRaw for Item {}

    #[test]
    fn as_raw() {
        let a = Item {
            a: 0x12345678,
            b: 0x54,
            c: 0xabcd,
        };
        let raw_a = a.as_raw();
        // We don't know our endianness, so make sure at least one of these is true.
        assert!(&raw_a[0..4] == &[0x78, 0x56, 0x34, 0x12] ||
                &raw_a[0..4] == &[0x12, 0x34, 0x56, 0x78]);
        // There are some padding assumptions, which should be true on most
        // modern architectures.
        assert!(&raw_a[6..8] == &[0xab, 0xcd] ||
                &raw_a[6..8] == &[0xcd, 0xab]);
        assert_eq!(raw_a[4], 0x54);
    }

    #[test]
    fn as_raw_mut() {
        let mut a = Item::default();
        {
            let raw = a.as_mut_raw();
            raw.copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x54, 0xde, 0xab, 0xcd]);
        }
        let big = Item {
            a: 0x12345678,
            b: 0x54,
            c: 0xabcd,
        };
        let little = Item {
            a: 0x78563412,
            b: 0x54,
            c: 0xcdab,
        };
        // Note that the padding is filled in, and we assume the derived Eq only
        // checks the defined fields.
        assert!(a == big || a == little);
    }
}

//! Boot image support

use core::mem::size_of;

use asraw::{AsRaw, AsMutRaw};
use embedded_storage::ReadStorage;

// TODO: Move the error into a more general place.
type Result<T, E> = core::result::Result<T, Error<E>>;

// Errors are constrained by the flash type's error.
pub enum Error<E> {
    Flash(E),
    InvalidImage,
}

// TODO: Can I get a From to work for this, so just `?` can be used?
// Unclear why this doesn't work.
/*
fn eget<E>(e: E) -> Error<E> {
    Error::Flash(e)
}
*/

/// Perform some operation that returns a result from a flash device, converting
/// the error to our local error type.
macro_rules! flash {
    ($e:expr) => {
        $e.map_err(|e| Error::Flash(e))
    };
}

/// Convert a flash error.  Doesn't infer very well.
/*
fn doflash<F: ReadStorage, T>(r: core::result::Result<T, F::Error>) -> Result<T, F::Error> {
    r.map_err(|e| Error::Flash(e))
}
*/

/// Try to make this image into a u32, returning a locally meaningful result
/// type.
fn to_u32<E>(v: usize) -> Result<u32, E> {
    v.try_into().map_err(|_| Error::InvalidImage)
}

/// The image header contains the following magic value, indicating the
/// interpretation of the rest of the image header.
pub const IMAGE_MAGIC: u32 = 0x96f3b83d;

/// An image is a bootable image residing in a flash partition.  There is a
/// header at the beginning, and metadata immediately following the image.
pub struct Image {
    #[allow(dead_code)]
    header: ImageHeader,
}

impl Image {
    /// Make an image from flash, if the image is valid.
    pub fn from_flash<F: ReadStorage>(flash: &mut F) -> Result<Image, F::Error> {
        let mut header =ImageHeader::default();
        flash!(flash.read(0, header.as_mut_raw()))?;

        if header.magic != IMAGE_MAGIC {
            return Err(Error::InvalidImage);
        }

        // Find the base address of the TLV.
        let tlv_base = (header.img_size as usize)
            .checked_add(header.hdr_size as usize)
            .ok_or(Error::InvalidImage)?;

        // Overflow of the partition will be checked by the flash device.
        // Capacity is not guaranteed to be returned.

        // Simple case of just a single TLV entry for hash.  TODO: More
        // sophisticated handling should be done separate from here.
        let mut info = TlvInfo::default();
        flash!(flash.read(to_u32(tlv_base)?, info.as_mut_raw()))?;

        println!("header: {:#x?}", header);
        println!("tlv: {:#x?}", info);

        if info.magic != INFO_MAGIC {
            return Err(Error::InvalidImage);
        }

        let mut pos = size_of::<TlvEntry>();
        while pos < info.len as usize {
            let mut entry = TlvEntry::default();
            flash!(flash.read(to_u32(tlv_base + pos)?, entry.as_mut_raw()))?;
            println!("entry: {:x?}", entry);

            pos += size_of::<TlvEntry>() + entry.len as usize;
        }

        Ok(Image {
            header,
        })
    }
}

#[cfg(test)]
mod tester {
    use embedded_storage::ReadStorage;

    use super::Image;

    const TEST: &[u8] = include_bytes!("../../hello/lpc55s69/signed.bin");

    struct Simple<'a>(&'a [u8]);

    struct Error;

    impl<'a> ReadStorage for Simple<'a> {
        type Error = Error;
        fn capacity(&self) -> usize { todo!() }
        fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
            let offset = offset as usize;

            // Let bound checking catch the errors in the test.
            buf.copy_from_slice(&self.0[offset .. offset + buf.len()]);
            Ok(())
        }
    }

    #[test]
    fn test_load() {
        let mut flash = Simple(TEST);

        let _ = Image::from_flash(&mut flash);
        todo!()
    }
}

/// The image begins with the following header.  This is intended to be
/// interpreted as a C struct.
#[derive(Debug, Default)]
#[repr(C)]
struct ImageHeader {
    /// Magic number, indicates this particular header.
    magic: u32,
    /// The address to load this image.  Only used for non-XIP.  It seems to be
    /// used if non-zero, which assumes that RAM does not start at address zero.
    load_addr: u32,
    /// The size of the header.  This struct is at the beginning, and there is
    /// some amount of padding before the actual image starts.  This is used
    /// because many architectures place alignment requirements on the runable
    /// image.
    hdr_size: u16,
    /// The size of the protected TLV.  The size is included here.  See below on
    /// the TLV for the meaning of this value.
    protected_tlv_size: u16,
    /// The size of the image, not counting the header.
    img_size: u32,
    /// Flags for this image.  These indicate aspects, but are largely unused.
    flags: u32,
    /// Version of this particular image.
    version: ImageVersion,
    /// Padding, to reach a nicely aligned minimum size.
    pad1: u32,
}

impl AsRaw for ImageHeader {}
unsafe impl AsMutRaw for ImageHeader {}

/// Each image has a version.  This is a pseudo-semantic version used to
/// determine upgrade elligibility and compatible between multi-image setups.
#[derive(Debug, Default)]
#[repr(C)]
struct ImageVersion {
    major: u8,
    minor: u8,
    revision: u16,
    build_num: u32,
}

/// The TLV block contains this header.
#[derive(Debug, Default)]
#[repr(C)]
struct TlvInfo {
    /// Magic one of TLV_INFO_MAGIC or TLV_PROT_INFO_MAGIC.
    magic: u16,
    /// Length of TLV, including this header.
    len: u16,
}

const INFO_MAGIC: u16 = 0x6907;

impl AsRaw for TlvInfo {}
unsafe impl AsMutRaw for TlvInfo {}

/// Each TLV entry is preceeded by this header.
#[derive(Debug, Default)]
#[repr(C)]
struct TlvEntry {
    /// Magic one of TLV_INFO_MAGIC or TLV_PROT_INFO_MAGIC.
    kind: u16,
    /// Length of TLV, including this header.
    len: u16,
}

impl AsRaw for TlvEntry {}
unsafe impl AsMutRaw for TlvEntry {}

//! Boot image support

use core::{mem::size_of, cell::RefCell};

use asraw::{AsRaw, AsMutRaw};
use embedded_storage::nor_flash::{NorFlashError, NorFlashErrorKind, ReadNorFlash};

// TODO: Move the error into a more general place.
type Result<T> = core::result::Result<T, Error>;

// Use the error kind to avoid this depending on the particular flash.
#[derive(Debug)]
pub enum Error {
    Flash(NorFlashErrorKind),
    InvalidImage,
}

/// Convert the nor flash error into our error type.
impl<E: NorFlashError> From<E> for Error {
    fn from(e: E) -> Self {
        Error::Flash(e.kind())
    }
}

/// Try to make this image into a u32, returning a locally meaningful result
/// type.
fn to_u32(v: usize) -> Result<u32> {
    v.try_into().map_err(|_| Error::InvalidImage)
}

/// The image header contains the following magic value, indicating the
/// interpretation of the rest of the image header.
pub const IMAGE_MAGIC: u32 = 0x96f3b83d;

/// An image is a bootable image residing in a flash partition.  There is a
/// header at the beginning, and metadata immediately following the image.
/// This holds on to a RefCell to the flash to bind the data to a particular flash.
pub struct Image<'f, F: ReadNorFlash> {
    flash: &'f RefCell<F>,
    #[allow(dead_code)]
    header: ImageHeader,
    tlv_base: usize,
}

impl<'f, F: ReadNorFlash> Image<'f, F> {
    /// Make an image from flash, if the image has a valid header. This does not
    /// indicate that the image itself is valid, merely that the header
    /// indicates an image is present.
    pub fn from_flash(flash: &'f RefCell<F>) -> Result<Image<'f, F>> {
        let mut header =ImageHeader::default();
        flash.borrow_mut().read(0, header.as_mut_raw())?;

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
        flash.borrow_mut().read(to_u32(tlv_base)?, info.as_mut_raw())?;

        println!("header: {:#x?}", header);
        println!("tlv: {:#x?}", info);

        if info.magic != TLV_INFO_MAGIC {
            return Err(Error::InvalidImage);
        }

        let mut pos = size_of::<TlvEntry>();
        while pos < info.len as usize {
            let mut entry = TlvEntry::default();
            flash.borrow_mut().read(to_u32(tlv_base + pos)?, entry.as_mut_raw())?;
            println!("entry: {:x?}", entry);

            pos += size_of::<TlvEntry>() + entry.len as usize;
        }

        Ok(Image {
            flash,
            header,
            tlv_base,
        })
    }

    /// Iterate over the elements of the Tlv.
    pub fn tlvs<'a>(&'a self) -> Result<TlvIter<'a, 'f, F>> {
        // Check the header.
        let mut info = TlvInfo::default();
        self.flash.borrow_mut().read(to_u32(self.tlv_base)?, info.as_mut_raw())?;

        if info.magic != TLV_INFO_MAGIC {
            return Err(Error::InvalidImage);
        }

        Ok(TlvIter {
            image: self,
            pos: size_of::<TlvInfo>(),
            limit: info.len as usize,
        })
    }

    /// Validate this image. Check the TLV entries, making sure that they are
    /// sufficient, and that indicated items, such as hashes and signatures are
    /// valid.
    pub fn validate(&self) -> Result<()> {
        // Things we must see.
        let mut seen_sha = false;

        for elt in self.tlvs()? {
            let elt = elt?;
            println!("TLV: 0x{:x}", elt.kind());
            match elt.kind() {
                TLV_SHA256 => {
                    if seen_sha {
                        // Only a single hash is allowed.
                        return Err(Error::InvalidImage);
                    }
                    seen_sha = true;
                    println!("Would verify sha");
                }
                kind => {
                    println!("Unexpected TLV 0x{:x}", kind);
                    return Err(Error::InvalidImage);
                }
            }
        }
        if !seen_sha {
            println!("Expecting SHA TLV");
            return Err(Error::InvalidImage);
        }
        Ok(())
    }
}

pub struct TlvIter<'a, 'f, F: ReadNorFlash> {
    image: &'a Image<'f, F>,
    pos: usize,
    limit: usize,
}

pub struct TlvIterEntry<'f, F: ReadNorFlash> {
    flash: &'f RefCell<F>,
    kind: u16,
    pos: usize,
    len: usize,
}

/// Helper like '?' for iterator operations, where errors should return
/// Some(Err(e)) instead of just the error.  This macro contains a return.
macro_rules! iter_try {
    ($e:expr) => {
        match $e {
            Ok(r) => r,
            Err(e) => return Some(Err(e.into())),
        }
    };
}

impl<'a, 'f, F: ReadNorFlash> Iterator for TlvIter<'a, 'f, F> {
    type Item = Result<TlvIterEntry<'f, F>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.limit {
            return None;
        }

        let mut entry = TlvEntry::default();
        let pos = iter_try!(self.image.tlv_base.checked_add(self.pos).ok_or(Error::InvalidImage));
        let pos32 = iter_try!(to_u32(pos));
        iter_try!(self.image.flash.borrow_mut().read(pos32, entry.as_mut_raw()));
        let data_pos = iter_try!(pos.checked_add(size_of::<TlvEntry>()).ok_or(Error::InvalidImage));
        self.pos = iter_try!(data_pos.checked_add(entry.len as usize).ok_or(Error::InvalidImage));
        Some(Ok(TlvIterEntry {
            flash: self.image.flash,
            kind: entry.kind,
            pos: data_pos,
            len: entry.len as usize,
        }))
    }
}

impl<'f, F: ReadNorFlash> TlvIterEntry<'f, F> {
    /// What is the kind of this TLV entry.
    pub fn kind(&self) -> u16 {
        self.kind
    }

    /// What is the size of the payload.
    pub fn data_len(&self) -> usize {
        self.len
    }

    /// Read the payload into the given bytes.
    pub fn read_data(&self, data: &mut [u8]) -> Result<()> {
        if data.len() != self.len {
            // TODO: Is something more meaningful here?
            return Err(Error::InvalidImage);
        }
        let pos = to_u32(self.pos)?;
        self.flash.borrow_mut().read(pos, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tester {
    use core::cell::RefCell;

    // use embedded_storage::{ReadStorage, nor_flash::ReadNorFlash};
    use embedded_storage::nor_flash::{ErrorType, NorFlashError, ReadNorFlash, NorFlashErrorKind};

    use super::Image;

    const TEST: &[u8] = include_bytes!("../../hello/lpc55s69/signed.bin");

    struct Simple<'a>(&'a [u8]);

    #[derive(Debug)]
    struct Error;

    impl NorFlashError for Error {
        fn kind(&self) -> NorFlashErrorKind {
            NorFlashErrorKind::Other
        }
    }

    impl<'a> ErrorType for Simple<'a> {
        type Error = Error;
    }

    impl<'a> ReadNorFlash for Simple<'a> {
        const READ_SIZE: usize = 1;
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
        let flash = RefCell::new(Simple(TEST));

        let image = Image::from_flash(&flash).unwrap();
        image.validate().unwrap();
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

const TLV_INFO_MAGIC: u16 = 0x6907;

// Supported TLVS
const TLV_SHA256: u16 = 0x10;

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

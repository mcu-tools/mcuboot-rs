//! Image upgrade status
//!
//! The image upgrade keeps track of its progress through a set of 'status' data
//! that is represented at the end of one or more of the partitions of the
//! flash.  The characterists of the flash determine which of two fairly
//! different techniques we will use to store this data.
//!
//! The status represents one a small number of states that we can be in:
//!
//! - None. The images are just present, and we aren't expecting to do an
//!   upgrade.
//! - Request. A new image is in the upgrade slot, and we've marked for
//!   an upgrade.
//! - Started. Status data is calculated and recorded.  What data depends on the
//!   upgrade method.
//! - Move done.  The move stage of an upgrade has completed.
//! - Copy done.  The swap itself is finished.
//! - Image ok.  The image is valid, and a revert will not be attempted.
//!
//! None does not have status data associated with it.
//! Request has status data associated with the upgrade slot
//! The remaining status data is always stored in the destination slot.
//!
//! Depending on the request, copy done and image ok might be set together.
//!
//! The image goes through the following steps.
//!
//! (m = meta, md = move done, cd = copy done, ok = image ok)
//!
//! +------------------+------------------+--------------------------------+
//! | Slot 0 status    | Slot 1 status    | State
//! +------------------+------------------+--------------------------------+
//! | blank            | blank            | None
//! | any              | magic            | Request
//! | magic+m          | magic            | Started
//! | magic+m+md       | magic            | Move Done
//! | magic+m+md+cd    | magic            | Copy Done
//! | magic+m+md+cd+ok | magic            | Image ok - no further changes.
//! | magic+m+md+cd    | magic+m          | Started revert
//! | magic+m+md+cd    | magic+m+md       | Move Done revert
//! | magic+m+md+cd+ok | magic+m+md+cd+ok | Copy Done revert - no changes
//! | magic+m+md+cd+ok | magic+m+md+cd+ok | Copy Done revert - no changes
//! +------------------+------------------+--------------------------------+
//!
//! The characteristics of the flash device itself indicate whether we are in
//! "paged" status mode, or in "overwrite" status mode.
//!
//! Paged mode views the flash as follows (high address at the top, each section
//! is one sector).
//! +-----+--------------------------+
//! | n-1 | magic
//! |     | age + flags
//! |     | status-hash
//! |     | hash-seed
//! |     | image-A size in bytes
//! |     | image-B size in bytes
//! |     | encryption state
//! |     | first k sector hashes
//! +-----+--------------------------+
//! | n-2 | magic
//! |     | age + flags
//! |     | status-hash
//! |     | hash-seed
//! |     | first k sector hashes
//! +-----+--------------------------+
//! | n-3 | next page-size/4 hashes
//! +-----+--------------------------+
//! | n-4 | next page-size/4 hashes
//! +-----+--------------------------+
//! |     | etc
//! +-----+--------------------------+
//!
//! Overwrite mode is instead, as follows.  It makes the assumption that the
//! write size is smaller, and blocks for the flags can be left unwritten.
//! There is a single sector at the end of flash containing the information.
//! +-----+--------------------------------+
//! | n-1 | magic
//! |     | overwrite marker + alignment
//! |     | status hash (skips the flags as those can change.)
//! |     | hash-seed
//! |     | image-A size in bytes
//! |     | image-B size in bytes
//! |     | encryption state
//! |     |   .. pad to write boundary ..
//! |     | flag - move done
//! |     |   .. pad to write boundary ..
//! |     | flag - copy done
//! |     |   .. pad to write boundary ..
//! |     | flag - image ok
//! |     |   .. pad to write boundary ..
//! |     | first k sector hashes
//! +-----+--------------------------------+
//! | n-2 | next sector-size/4 hashes
//! +-----+--------------------------------+
//! | n-3 | next sector-size/4 hashes
//! |     |   .. pad to write boundary ..
//! |     | end of image payload
//! +-----+--------------------------------+
//! (if not clear, the flags each _start_ at a write boundary)
//!
//! Because of partial writes within a sector, overwrite mode allows the end of
//! the image to share the last sector with the status data.  The number of
//! sectors involved will depend on the sizes of the images.

// use storage::ReadFlash;

use core::mem::size_of;

use crate::Result;
use asraw::{AsRaw, AsMutRaw};
use storage::Flash;

mod sizes {
    /// Maximum expected image size.
    const MAX_IMAGE: usize = 1024 * 1024;

    /// Smallest page size for paged mode.
    const SMALLEST_PAGED_SECTOR: usize = 512;

    /// Smallest sector in overwrite mode.
    const SMALLEST_SECTOR: usize = 4096;

    /// Number of hashes expected in paged.
    const PAGED_HASHES: usize = MAX_IMAGE.div_ceil(SMALLEST_PAGED_SECTOR);

    /// Number of hashes expected in overwrite.
    const OVERWRITE_HASHES: usize = MAX_IMAGE.div_ceil(SMALLEST_SECTOR);

    /// Max needed hashes.  Will need to store twice this in status are.
    #[allow(dead_code)]
    pub const MAX_HASHES: usize = {
        if PAGED_HASHES > OVERWRITE_HASHES {
            PAGED_HASHES
        } else {
            OVERWRITE_HASHES
        }
    };
    // PAGED_HASHES.max(OVERWRITE_HASHES);

    /// How many sectors of hash can we expect in paged.
    pub const MAX_PAGED_HASH_SECTORS: usize =
        (PAGED_HASHES + PAGED_HASHES).div_ceil(SMALLEST_PAGED_SECTOR);
    pub const MAX_OVERWRITE_HASH_SECTORS: usize =
        (OVERWRITE_HASHES + OVERWRITE_HASHES).div_ceil(SMALLEST_SECTOR);

    pub const MAX_HASH_SECTORS: usize = {
        if MAX_PAGED_HASH_SECTORS > MAX_OVERWRITE_HASH_SECTORS {
            MAX_PAGED_HASH_SECTORS
        } else {
            MAX_OVERWRITE_HASH_SECTORS
        }
    };

    pub type HashVec<T> = heapless::Vec<T, MAX_HASH_SECTORS>;
    // pub type PHashVec = heapless::Vec<usize, MAX_PAGED_HASH_SECTORS>;
    // pub type OHashVec = heapless::Vec<usize, MAX_OVERWRITE_HASH_SECTORS>;
}

/// Information needed to calculate status layout.
#[derive(Debug)]
pub struct SlotInfo {
    /// Device write size.
    pub write_size: usize,
    /// Device erase size.
    pub erase_size: usize,
    /// Size of full flash slot.
    pub capacity: usize,
    /// Size, in bytes, of the image, including trailing TLV, etc.
    pub image_size: usize,
}

impl SlotInfo {
    /// Build SlotInfo out of an image and a flash device.
    pub fn from_data<F: Flash>(image_size: usize, flash: &F) -> SlotInfo {
        let write_size = flash.write_size();
        let erase_size = flash.erase_size();
        let capacity = flash.capacity();
        SlotInfo { write_size, erase_size, capacity, image_size }
    }

    /// Determine the status style for this slot.
    pub fn status_style(&self) -> StatusStyle {
        if self.write_size <= 32 {
            return StatusStyle::OverWrite;
        }

        if self.erase_size <= 4096 {
            return StatusStyle::Paged;
        }

        // It is unclear how to support this flash.
        panic!("Device configuration unsupported");
    }

    /// Given our info, compute the status layout for this particular slot.  The
    /// other slot information is needed to calculate this.
    pub fn status_layout(&self, upgrade: &SlotInfo) -> Result<StatusLayout> {
        // Use the larger of the two erase sizes for the swap.
        let erase_size = self.erase_size.max(upgrade.erase_size);

        assert!(self.erase_size.is_power_of_two());
        assert!(self.write_size.is_power_of_two());

        let image_sectors = [
            self.image_size.div_ceil(erase_size),
            upgrade.image_size.div_ceil(erase_size)
        ];
        let style = self.status_style();
        // println!("Erase size: {}", erase_size);
        // println!("Image sectors: {:?}", image_sectors);
        // println!("Tail size: {}", size_of::<StatusTail>());
        // println!("Style: {:?}", style);

        // Calculate the layout of our last page, or two, depending on mode.
        let mut pos = erase_size;

        // The tail goes at the end.
        pos -= size_of::<StatusTail>();
        let tail_pos = pos;

        // The status flags are present
        let flags = if style == StatusStyle::OverWrite {
            // Round down to be write aligned.
            pos = pos & !(self.write_size - 1);

            pos -= self.write_size;
            let move_done_flag = pos;

            pos -= self.write_size;
            let copy_done_flag = pos;

            pos -= self.write_size;
            let image_ok_flag = pos;

            Some([move_done_flag, copy_done_flag, image_ok_flag])
        } else {
            None
        };

        let end_hashes = pos;
        pos &= !(erase_size - 1);

        let total_image_sectors = image_sectors[0] + image_sectors[1];
        let inline_hashes = ((end_hashes - pos) / 4).min(total_image_sectors);

        // Calculate additional pages of hashes.
        let mut hash_pages = sizes::HashVec::new();
        let mut count = total_image_sectors - inline_hashes;
        while count > 0 {
            let n = (erase_size / 4).min(count);
            hash_pages.push(n).unwrap();
            count -= n;
        }

        // println!("Hashes: {} bytes", end_hashes - pos);
        // println!("Tail pos: {}", tail_pos);
        // println!("flags pos: {:?}", flags);
        // println!("inline hashes: {}", inline_hashes);
        // println!("Additional hashes: {:?}", hash_pages);

        Ok(StatusLayout {
            style,
            erase_size,
            write_size: self.write_size,
            image_sectors,
            tail_pos,
            flags,
            inline_hashes,
            hash_pages,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum StatusStyle {
    Paged,
    OverWrite
}

#[derive(Debug)]
pub struct StatusLayout {
    pub style: StatusStyle,
    pub erase_size: usize,
    pub write_size: usize,
    pub image_sectors: [usize; 2],
    pub tail_pos: usize,
    pub flags: Option<[usize; 3]>,
    pub inline_hashes: usize,
    pub hash_pages: sizes::HashVec<usize>,
}

impl StatusLayout {
    pub fn read<F: Flash>(&self, flash: &mut F) -> Result<()> {
        // Calculate the address of the last page.
        let last_page = ((flash.capacity() / flash.erase_size()) - 1) * flash.erase_size();

        println!("Last page: {:x}", last_page);
        let last_tail_pos = last_page + self.tail_pos;

        let mut last_tail = StatusTail::default();
        flash.read(last_tail_pos, last_tail.as_mut_raw())?;

        Ok(())
    }
}

/// The status tail.  This data is placed at the very end of the slot.
#[derive(Debug, Default)]
#[repr(C)]
struct StatusTail {
    /// The encryption key, used if we are encrypting in/out of slot0.
    enc_key: [u8; 16],
    /// Size of the main image, in bytes, includes TLV.
    main_size: u32,
    /// Size of the upgrade image, in bytes, includes TLV.
    upgrade_size: u32,
    /// The hash seed.  Added to the beginning of the hash to make it unique.
    hash_seed: u32,
    /// Log2 of the write_size in this slot.  (1 << write_log) gives the write size.
    write_log: u8,
    /// Log2 of the erase size.  This is the larest of the two slots.
    erase_log: u8,
    /// Flags to indicate status.  Flags are here, unless the 'age' field is set
    /// to 0xff, which indicates that we are in overwrite not paged mode, and
    /// the flags are before this data.
    flags: u8,
    /// Age of this page, or 0xff to indicate overwrite mode.
    age: u8,
    /// The magic number.  This should land at the end of the image.
    magic: [u8; 16],
}

impl AsRaw for StatusTail {}
unsafe impl AsMutRaw for StatusTail {}

/*
#[repr(u8)]
pub enum Flags {
    MoveDone = 0b0001,
    CopyDone = 0b0010,
    ImageOk = 0b0100,
}
*/

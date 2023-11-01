//! Flash styles
//!
//! Various microcontrollers have various types of flash memories available to them.

use crate::SimFlash;
use crate::Result;

/// The configuration of a single flash area.
pub struct AreaLayout {
    pub read_size: usize,
    pub write_size: usize,
    pub erase_size: usize,
    pub sectors: usize,
}

impl AreaLayout {
    pub fn build(&self) -> Result<SimFlash> {
        SimFlash::new(
            self.read_size,
            self.write_size,
            self.erase_size,
            self.sectors,
        )
    }
}

/// STM32F4-style.
/// These devices have a fairly small number of relatively large sectors.  Note
/// that if you wish to use MCUboot across an area where the sector sizes
/// differ, MCUboot will see the smaller sectors as if they were a larger sector
/// of whatever the largest size within the region is.
/// This tends to stress the extreme in terms of small, as the image and the
/// status area must fit entirely within the seconary area, which is a single sector.
pub static STM32F_MAIN: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 8,
    erase_size: 128*1024,
    sectors: 2,
};
pub static STM32F_UPGRADE: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 8,
    erase_size: 128*1024,
    sectors: 1,
};

/// K64-style.
/// These devices have small uniform sectors.
pub static K64_MAIN: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 8,
    erase_size: 4*1024,
    sectors: 128/4 + 1,
};
pub static K64_UPGRADE: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 8,
    erase_size: 4*1024,
    sectors: 128/4 + 1,
};

/// External flash configuration.  The external partition is the same size, so
/// the image needs to have room.  The external flash has a large write alignment.
pub static EXT_MAIN: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 4,
    erase_size: 4*1024,
    sectors: 128/4,
};
pub static EXT_UPGRADE: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 256,
    erase_size: 4*1024,
    sectors: 128/4,
};

/// Page-style devices.  Based on the LPC55S69.
pub static LPC_MAIN: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 512,
    erase_size: 512,
    sectors: 128*2,
};
pub static LPC_UPGRADE: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 512,
    erase_size: 512,
    sectors: 128*2,
};

/// Another large write, based on the STM32H745
pub static STM32H_MAIN: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 32,
    erase_size: 128*1024,
    sectors: 4,
};
pub static STM32H_UPGRADE: AreaLayout = AreaLayout {
    read_size: 1,
    write_size: 32,
    erase_size: 128*1024,
    sectors: 3,
};

/// All of the flash devices, as pairs.
pub static ALL_FLASHES: [(&'static AreaLayout, &'static AreaLayout); 5] = [
    (&STM32F_MAIN, &STM32F_UPGRADE),
    (&K64_MAIN, &K64_UPGRADE),
    (&EXT_MAIN, &EXT_UPGRADE),
    (&LPC_MAIN, &LPC_UPGRADE),
    (&STM32H_MAIN, &STM32H_UPGRADE),
];

/// An iterator that returns each of the device pairs on each iteration.
pub fn all_flashes() -> impl Iterator<Item = Result<(SimFlash, SimFlash)>> {
    ALL_FLASHES.iter().map(|(a, b)| {
        let a = match a.build() {
            Ok(a) => a,
            Err(e) => return Err(e),
        };
        let b = match b.build() {
            Ok(b) => b,
            Err(e) => return Err(e),
        };
        Ok((a, b))
    })
}

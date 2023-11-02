//! Image generation.

use std::{fs::{File, self}, io::Write, process::{Command, Stdio}};

use rand::{SeedableRng, RngCore};
use rand_xoshiro::Xoshiro256Plus;

use anyhow::{Result, anyhow};
use temp_dir::TempDir;

pub struct GeneratedImage {
    pub data: Vec<u8>,
}

pub struct GenBuilder {
    /// Size of the zeroed header.
    header_size: usize,
    /// Total size of the image, not counting the TLV.
    size: usize,
    /// Seed for the PRNG
    seed: usize,
    /// Version
    version: String,
}

impl Default for GenBuilder {
    fn default() -> Self {
        GenBuilder {
            header_size: 256,
            size: 76_137,
            seed: 1,
            version: "0.1.0".to_string(),
        }
    }
}

impl GenBuilder {
    pub fn size(&mut self, size: usize) -> &mut Self {
        self.size = size;
        self
    }

    pub fn seed(&mut self, seed: usize) -> &mut Self {
        self.seed = seed;
        self
    }

    pub fn build(&self) -> Result<GeneratedImage> {
        let mut rng = Xoshiro256Plus::seed_from_u64(self.seed as u64);
        let mut input = vec![0u8; self.size];
        rng.fill_bytes(&mut input);

        // The header is required to be zeros, so just fill that in.
        input[..self.header_size].fill(0);

        let tmp = TempDir::new()?;

        let src = tmp.path().join("image.bin");
        let dest = tmp.path().join("image-signed.bin");

        File::create(&src)?.write_all(&input)?;

        // Run imgtool.
        let mut cmd = Command::new("imgtool");
        cmd.arg("sign");

        cmd.arg("--header-size");
        cmd.arg(&format!("{}", self.header_size));

        cmd.arg("-v");
        cmd.arg(&self.version);

        // This can be removed in very recent versions.
        cmd.arg("--align");
        cmd.arg("4");

        // TODO: Figure this out from the flash?
        cmd.arg("--slot-size");
        cmd.arg(format!("{}", 128*1024));

        cmd.arg(&src);
        cmd.arg(&dest);

        cmd.stdin(Stdio::null());

        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow!("Unable to run imgtool: {}", status));
        }

        let data = fs::read(&dest)?;

        Ok(GeneratedImage { data })
    }
}

#[cfg(test)]
mod tester {
    use std::cell::RefCell;
    use boot::Image;

    use crate::styles;

    use super::GenBuilder;

    #[test]
    fn test_gen() {
        let img = GenBuilder::default()
            .build()
            .unwrap();
        let mut flash = styles::LPC_MAIN.build().unwrap();
        flash.install(&img.data, 0).unwrap();
        let flash = RefCell::new(flash);
        let image = Image::from_flash(&flash).unwrap();
        image.validate().unwrap();
    }
}

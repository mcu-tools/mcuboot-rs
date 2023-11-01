// Image testing.

use std::cell::RefCell;

use boot::Image;
use simflash::SimFlash;

static IMG1: &[u8] = include_bytes!("../data/sample-signed.bin");

#[test]
fn image_test() {
    let mut fl = SimFlash::new(1, 512, 512, 256).unwrap();
    fl.install(IMG1, 0).unwrap();

    let fl = RefCell::new(fl);

    // Validate that this is a good image.
    let image = Image::from_flash(&fl).unwrap();
    image.validate().unwrap();
}

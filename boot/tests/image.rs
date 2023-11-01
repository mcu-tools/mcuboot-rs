// Image testing.

use std::cell::RefCell;

use boot::Image;

static IMG1: &[u8] = include_bytes!("../data/sample-signed.bin");

#[test]
fn image_test() {
    for flashes in simflash::styles::all_flashes() {
        let (mut main, _upgrade) = flashes.unwrap();

        main.install(IMG1, 0).unwrap();

        let main = RefCell::new(main);

        // Validate that this is a good image.
        let image = Image::from_flash(&main).unwrap();
        image.validate().unwrap();
    }
}

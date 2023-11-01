// Image testing.

use std::cell::RefCell;

use boot::{Image, SlotInfo};

static IMG1: &[u8] = include_bytes!("../data/sample-signed.bin");

#[test]
fn image_test() {
    for flashes in simflash::styles::all_flashes() {
        let (mut main, mut upgrade) = flashes.unwrap();

        main.install(IMG1, 0).unwrap();
        upgrade.install(IMG1, 0).unwrap();

        let main = RefCell::new(main);
        let upgrade = RefCell::new(upgrade);

        // Validate that this is a good image.
        let image = Image::from_flash(&main).unwrap();
        image.validate().unwrap();

        let uimage = Image::from_flash(&upgrade).unwrap();
        uimage.validate().unwrap();

        println!("---");
        println!("main: {:x?}", image.header);
        println!("upgrade: {:x?}", uimage.header);

        // Compute the status area here.
        let main_size = image.full_image_size();
        let upgrade_size = image.full_image_size();
        let info = SlotInfo::from_data(main_size, &*main.borrow());
        println!("info: {:x?}", info);
        let upgrade_info = SlotInfo::from_data(upgrade_size, &*upgrade.borrow());
        println!("uinfo: {:x?}", upgrade_info);
        // println!("info: {:#x?}", info);
        let sminfo = info.status_layout(&upgrade_info);
        println!("main status: {:#x?}", sminfo);
        let suinfo = upgrade_info.status_layout(&info);
        println!("upgrade status: {:#x?}", suinfo);
    }
    todo!();
}

# MCUboot - In Rust

This project is the beginnings of a fresh implementation of MCUboot in Rust.  At
this point, it implements SHA256 image verification, booting a chained image,
and works on the LPC55S69 LPCXpresso development, using the lp55-hal crate.
This is more a proof of concept.

Some things that are implemented:

-   The boot code lives in its own crate.  This supports `cargo test` to perform
    some unit testing (which is incomplete).  When built with 'std' enabled, the
    crate prints various pieces of information out, and is useful for debuggin.
-   The boot crate can be used `no_std`, and has no diagnostics, and returns
    very simple error codes.
-   `boards/lpc55s69` contains a build of a bootloader using the boot crate.
    Upon successfully validaing an image, it will chain boot to that crate.
-   `hello/lpc55s69` contains a simple hello/blinky application to demonstrate
    the bootloader.  The signed version can be placed in the appropriate slot to
    test the bootloader.

Things that still need to be done:

-   Other types of verification, such as signatures.
-   Upgrades.  The main motivation of this project is to develop a new swap
    algorithm and status storage that is appropriate for large-write devices
    (hence the LPC55S69).
-   Everything else.  MCUboot has a lot of other functionality.
-   Broader support.  Probably worth adding at least one other board to work to
    generalize the board support to make these support files minimal.
-   Zephyr support.  Related to getting Rust to work on Zephyr would make this
    an ideal application of that.

Some issues with the LPC55S69 specifically:

-   There are several issue with the tip version, and some of the other
    dependencies I use.  This uses a submodule with a fork of the lpc55-hal
    crate to address these issues.
-   The HW SHA256 engine doesn't compile, and rather than fix this at this time,
    just replace the offending code with a `todo!()`.  When adding the idea of
    crypto providers to `boot`, this can be addressed to add support for the
    LPC's hardware crypto support.
-   Clock initialization can only be done once.  Until addressed, the boot main
    doesn't initialize any clocks.  It appears that clock initialization is
    leaving the CPU connected to the PLL while trying to program it.

## How to try it.

In order to test this out, you will currently need an LPCXpresso55S69
development board.  I was able to get it to work with the jlink firmware.

-   Install the jlink tools for your platform.
-   Install the Arm toolchain gdb.  On ubuntu, this is `gdb-arm-none-eabi`
-   Install socat.
-   Make sure you have rust installed.  Rustup is the easiest way to do this.
-   Install imgtool from the main mcuboot project:
```
$ pip install -u imgtool
```
-   Install the thumbv8m.main-none-eabi target if you haven't
```
$ rustup target add thumbv8m.main-none-eabi
```
-   Build the 'hello world' application.
```
$ cd hello/lpc55s69
$ cargo build
$ make sign
$ cd ../..
```
this should generate a `signed.bin` which is a signed version.
-   Test the boot code.
```
$ cd boot
$ cargo test
$ cd ..
```
note that, at this time, these tests are incomplete, and actually fail to print
out what it is doing.
-   Test on the target
You'll need three windows for this.  Each window should be in the
`boards/lpc55s69` directory.

In one window, start the jlink gdb server:
```
$ make jlink
```
In another window, run socat to dump semihosting messages.
```
$ make semi
```
And in the third window, start gdb.
```
$ cargo run
```
This should load both the executable under test, and the signed.bin image
created earlier.

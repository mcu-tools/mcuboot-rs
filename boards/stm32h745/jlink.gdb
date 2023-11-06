# Debug using gdb

set history save on
set confirm off

# find commit-hash using `rustc -Vv`
set substitute-path /rustc/cc66ad468955717ab92600c770da8c1601a4ff33 \
    /home/davidb/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust

target extended-remote :2331
load
monitor reset

# monitor semihosting enable
# monitor semihosting breakOnError
# monitor semihosting IOClient 3

# Load the target image with the signed version of the image:
# restore ../../hello/lpc55s69/signed.bin binary 0x20000

# To debug the target application, replace the symbols with these
# file ../../hello/lpc55s69/target/thumbv8m.main-none-eabi/debug/hello-lpc55s69

# b main
# b lpc55_hal::drivers::clocks::ClockRequirements::configure

# continue

# Debug using gdb

set history save on
set confirm off

# find commit-hash using `rustc -Vv`
set substitute-path /rustc/cc66ad468955717ab92600c770da8c1601a4ff33 \
    /home/davidb/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust

target extended-remote :2331
load
monitor reset

monitor semihosting enable
# monitor semihosting breakOnError
# monitor semihosting IOClient 3

# continue
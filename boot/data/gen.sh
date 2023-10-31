#! /bin/bash

# Generate a simple image.  The data is random, so will be different each time this script is run.

out=sample.bin

dd if=/dev/zero bs=256 count=1 > $out
dd if=/dev/urandom bs=1024 count=75 >> $out

imgtool sign \
        sample.bin sample-signed.bin \
        --align 4 \
        -v "0.1.0" \
        --header-size 256 \
        --slot-size 0x20000

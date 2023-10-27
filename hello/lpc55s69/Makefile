# Make for silly stuff

all:
	echo all is not a useful target.

# Start the jlink server so gdb can program the board.
not-jlink:
	env -u DISPLAY \
	    JLinkGDBServer -strict -device LPC55S69 -if SWD -vd

# Generate the signed image
sign: signed.bin
signed.bin:
	cargo objcopy -- -O binary unsigned.bin
	imgtool sign \
		unsigned.bin signed.bin \
		--align 4 \
		-v '0.1.0' \
		--header-size 1024 \
		--slot-size 0x40000

.PHONY: signed.bin

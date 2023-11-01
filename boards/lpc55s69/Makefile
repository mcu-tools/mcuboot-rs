# Make for silly stuff

all:
	echo all is not a useful target.

# Start the jlink server so gdb can program the board.
jlink:
	env -u DISPLAY \
	    JLinkGDBServer -strict -device LPC55S69 -if SWD -vd

semi:
	socat - TCP4:localhost:2333

rtt:
	socat - TCP4:localhost:19021 | \
		defmt-print -e target/thumbv8m.main-none-eabi/debug/mcuboot-lpc55s69

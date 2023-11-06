# Make for silly stuff

all:
	echo all is not a useful target.

# Start the jlink server so gdb can program the board.
jlink:
	env -u DISPLAY \
	    JLinkGDBServer -strict -device STM32H745ZI_M7 -if SWD -vd

# Start the gdb connection, without loading.
gdb:
	arm-none-eabi-gdb -q -x debug.gdb target/thumbv7em-none-eabihf/debug/stm32h7-boot

semi:
	socat - TCP4:localhost:2333

rtt:
	defmt-print -e target/thumbv7em-none-eabihf/debug/stm32h7-boot --show-skipped-frames tcp
	# socat - TCP4:localhost:19021

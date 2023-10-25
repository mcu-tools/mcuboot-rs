# Make for silly stuff

all:
	echo all is not a useful target.

# Start the jlink server so gdb can program the board.
jlink:
	env -u DISPLAY \
	    JLinkGDBServer -strict -device LPC55S69 -if SWD -vd

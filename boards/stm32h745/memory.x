/**
 * See: https://github.com/stm32-rs/stm32h7xx-hal/blob/master/memory.x
 *      https://www.st.com/resource/en/reference_manual/dm00176879-stm32h745-755-and-stm32h747-757-advanced-arm-based-32-bit-mcus-stmicroelectronics.pdf
 */

MEMORY
{
    SRAM1         : ORIGIN = 0x30000000, LENGTH = 128K /* 32-bit AHB bus matrix, D2 domain */
    SRAM2         : ORIGIN = 0x30020000, LENGTH = 128K /* 32-bit AHB bus matrix, D2 domain */
    SRAM3   (RW)  : ORIGIN = 0x30040000, LENGTH = 32K  /* 32-bit AHB bus matrix, D2 domain */
    SRAM4   (RW)  : ORIGIN = 0x38000000, LENGTH = 64K  /* 32-bit AHB bus matrix, D3 domain */
    BSRAM         : ORIGIN = 0x38800000, LENGTH = 4K   /* 32-bit AHB bus matrix, D3 domain */
    AXISRAM (RWX) : ORIGIN = 0x24000000, LENGTH = 512K /* 64-bit AXI bus matrix, D1 domain */
    DTCMRAM (RWX) : ORIGIN = 0x20000000, LENGTH = 128K /* 64-bit AXI bus matrix, D1 domain */
    FLASH2  (RX)  : ORIGIN = 0x08100000, LENGTH = 1M   /* 64-bit AXI bus matrix, D1 domain */
    FLASH1  (RX)  : ORIGIN = 0x08000000, LENGTH = 1M   /* 64-bit AXI bus matrix, D1 domain */
    ITCMRAM (RWX) : ORIGIN = 0x00000000, LENGTH = 64K  /* 64-bit AXI bus matrix, D1 domain */
}

/* stm32h7xx-hal uses a PROVIDE that expects RAM and FLASH symbols to exist */
REGION_ALIAS(RAM, DTCMRAM);
REGION_ALIAS(FLASH, FLASH1);

/* The location of the stack can be overridden using the
   `_stack_start` symbol.  Place the stack at the end of RAM */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS
{
    .itcmram : ALIGN(4) {
        *(.itcmram .itcmram.*);
        . = ALIGN(4);
    } > ITCMRAM

    .dtcmram : ALIGN(4) {
        *(.dtcmram .dtcmram.*);
        . = ALIGN(4);
    } > DTCMRAM

    .axisram : ALIGN(8) {
        *(.axisram .axisram.*);
        . = ALIGN(8);
    } > AXISRAM

    /* The SRAM1 and SRAM2 section are commonly used as the stack and
       heap for the CM4 core in dualcore versions and should thus not
       be used in examples */

    .sram3 (NOLOAD) : ALIGN(4) {
        *(.sram3 .sram3.*);
        . = ALIGN(4);
    } > SRAM3

    .sram4 (NOLOAD) : ALIGN(4) {
        *(.sram4 .sram4.*);
        . = ALIGN(4);
     } > SRAM4
};

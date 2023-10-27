/* Partition table for development.
   0    - 128k  - bootloader
   128k - 384k  - slot 0
   384k - 640k  - slot 1
*/
MEMORY
{
  BOOT_HEADER : ORIGIN = 0x0020000, LENGTH = 1024
  FLASH : ORIGIN = 0x00020000 + 1024, LENGTH = 256K - 1024
  RAM : ORIGIN = 0x20000000, LENGTH = 256K
}

SECTIONS {
  /* ### Boot header */
  .boot_header ORIGIN(BOOT_HEADER) :
  {
    KEEP(*(.boot_header));
  } > BOOT_HEADER
} INSERT BEFORE .text;

/* Pico 1 / Pico W / Pico WH -- RP2040, 2 MiB of QSPI flash, 264 KiB of SRAM. */

MEMORY {
    /* The RP2040 bootrom copies the first 256 bytes of flash into SRAM and
       runs them; that stub is what configures the QSPI interface for
       execute-in-place, so nothing else in flash is addressable until it has
       run. `BOOT2` is where it has to live, and it is exactly 256 bytes
       because the last four are a checksum the bootrom verifies. */
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100

    /* 256K, not the 264K on the box. RAM0-3 are striped into one contiguous
       bank at 0x2000_0000; the remaining two 4 KiB banks sit above it with a
       gap, so covering them would mean a separate region for 8 KiB that
       nothing here needs. */
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}

SECTIONS {
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;

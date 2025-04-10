/*
 * The RPi3 firmware loads the kernel at physical address 0x80000.
 * However, we link and eventually run the kernel from a high address.
 */
__kernel_start = 0xffffffffc0080000;

SECTIONS
{
    . = __kernel_start;
    .text :
    {
        KEEP(*(.text.boot))
        *(.text*)
    }
    .rodata :
    {
        *(.rodata*)
    }
    .data :
    {
        *(.data*)
    }
    /*
     * The __bss_start and __bss_end symbols need to be 8-byte aligned so that
     * the code that clears the BSS in boot.s works correctly.
     */
    .bss (NOLOAD) : ALIGN(8)
    {
        __bss_start = .;
        *(.bss*);
        . = ALIGN(8);
        __bss_end = .;
    }
}

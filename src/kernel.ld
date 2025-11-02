/*
 * The RPi3 firmware loads the kernel at physical address 0x80000.
 * However, we link and eventually run the kernel from a high address.
 */
__kernel_start = 0xffffffffc0080000;

SECTIONS
{
    . = __kernel_start;
    /*
     * Use 4KiB alignment for .text and the other sections because we will map
     * them using different attributes in the page tables.
     */
    .text : ALIGN(4K)
    {
        __text_start = .;
        KEEP(*(.text.boot))
        *(.text*)
        . = ALIGN(4K);
        __text_end = .;
    }
    .rodata : ALIGN(4K)
    {
        __rodata_start = .;
        *(.rodata*)
        . = ALIGN(4K);
        __rodata_end = .;
    }
    .data : ALIGN(4K)
    {
        __data_start = .;
        *(.data*)
        __data_end = .;
    }
    /*
     * We don't use 4KiB alignment here because the .bss section will be mapped
     * with the same attributes as the .data section in the page tables. The
     * __bss_start and __bss_end symbols still need to be 8-byte aligned so
     * that the code that clears the BSS in boot.s works correctly.
     */
    .bss (NOLOAD) : ALIGN(8)
    {
        __bss_start = .;
        *(.bss*);
        . = ALIGN(4K);
        __bss_end = .;
    }
    __kernel_size = . - __kernel_start;
}

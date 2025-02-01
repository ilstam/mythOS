/* The RPi3 firmware loads the kernel at this address */
__kernel_load_addr = 0x80000;

SECTIONS
{
    . = __kernel_load_addr;
    .text :
    {
        KEEP(*(.text.boot))
    }
}

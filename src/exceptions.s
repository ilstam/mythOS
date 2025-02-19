// NOTE: This code must not exceed 0x80 bytes
.macro exception_handler, handler
	bl \handler
	eret
.endmacro

.section .text

// The first 11 bits of VBAR are RES0
.p2align 11
__exception_table:
// Exception from the current EL while using SP_EL0
.org 0x000
	exception_handler el1_sp0_sync_handler
.org 0x080
	exception_handler el1_sp0_irq_handler
.org 0x100
	exception_handler el1_sp0_fiq_handler
.org 0x180
	exception_handler el1_sp0_serror_handler
// Exception from the current EL while using SP_ELx
.org 0x200
	exception_handler el1_sp1_sync_handler
.org 0x280
	exception_handler el1_sp1_irq_handler
.org 0x300
	exception_handler el1_sp1_fiq_handler
.org 0x380
	exception_handler el1_sp1_serror_handler
// Exception from a lower EL and at least one lower EL is AArch64
.org 0x400
	exception_handler el0_64_sync_handler
.org 0x480
	exception_handler el0_64_irq_handler
.org 0x500
	exception_handler el0_64_fiq_handler
.org 0x580
	exception_handler el0_64_serror_handler
// Exception from a lower EL and all lower ELs are AArch32
.org 0x600
	exception_handler el0_32_sync_handler
.org 0x680
	exception_handler el0_32_irq_handler
.org 0x700
	exception_handler el0_32_fiq_handler
.org 0x780
	exception_handler el0_32_serror_handler

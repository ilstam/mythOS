// NOTE: The ExceptionFrame layout is defined in exceptions.rs
.macro save_context
	sub sp, sp, #8 * 34

	stp x0,  x1,  [sp, #8 * 0]
	stp x2,  x3,  [sp, #8 * 2]
	stp x4,  x5,  [sp, #8 * 4]
	stp x6,  x7,  [sp, #8 * 6]
	stp x8,  x9,  [sp, #8 * 8]
	stp x10, x11, [sp, #8 * 10]
	stp x12, x13, [sp, #8 * 12]
	stp x14, x15, [sp, #8 * 14]
	stp x16, x17, [sp, #8 * 16]
	stp x18, x19, [sp, #8 * 18]
	stp x20, x21, [sp, #8 * 20]
	stp x22, x23, [sp, #8 * 22]
	stp x24, x25, [sp, #8 * 24]
	stp x26, x27, [sp, #8 * 26]
	stp x28, x29, [sp, #8 * 28]

	mrs x0, spsr_el1
	stp x30, x0, [sp, #8 * 30]

	mrs x0, elr_el1
	mrs x1, esr_el1
	stp x0, x1, [sp, #8 * 32]
.endmacro

// NOTE: This code must not exceed 0x80 bytes
.macro exception_handler, handler
	save_context
	// The SP now points to the ExceptionFrame.
	// Pass it as an argument to the handler.
	mov x0, sp
	bl \handler
	b restore_context_and_eret
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

// This should reverse save_context.
// It's not defined as a macro because then the exception handler would exceed 0x80 bytes.
restore_context_and_eret:
	ldp x0, x1, [sp, #8 * 32]
	msr elr_el1, x0
	msr esr_el1, x1

	ldp x30, x0, [sp, #8 * 30]
	msr spsr_el1, x0

	ldp x0,  x1,  [sp, #8 * 0]
	ldp x2,  x3,  [sp, #8 * 2]
	ldp x4,  x5,  [sp, #8 * 4]
	ldp x6,  x7,  [sp, #8 * 6]
	ldp x8,  x9,  [sp, #8 * 8]
	ldp x10, x11, [sp, #8 * 10]
	ldp x12, x13, [sp, #8 * 12]
	ldp x14, x15, [sp, #8 * 14]
	ldp x16, x17, [sp, #8 * 16]
	ldp x18, x19, [sp, #8 * 18]
	ldp x20, x21, [sp, #8 * 20]
	ldp x22, x23, [sp, #8 * 22]
	ldp x24, x25, [sp, #8 * 24]
	ldp x26, x27, [sp, #8 * 26]
	ldp x28, x29, [sp, #8 * 28]

	add sp, sp, #8 * 34

	eret

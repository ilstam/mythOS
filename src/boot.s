.section .text.boot

.global _start

_start:
	// Read the CPU ID. For RPi3 we just need to check Aff0.
	mrs    x0, mpidr_el1
	and    x0, x0, #7
	// CPU0 can proceed, secondary CPUs must wait forever
	cbz    x0, .L_primary_cpu

.L_secondary_loop:
	wfe
	b      .L_secondary_loop

.L_primary_cpu:
	// Set the top of the stack at _start (stack grows downwards)
	adr    x0, _start
	mov    sp, x0

	// Clear the BSS section
	adr    x0, __bss_start
	adr    x1, __bss_end
.L_clear_bss:
	// If start == end we are done
	cmp    x0, x1
	b.eq   .L_jump_to_rust
	// Store 0s to [x0], then increment x0 by 8
	str    xzr, [x0], #8
	b      .L_clear_bss

.L_jump_to_rust:
	b      main

	// main shouldn't return, but just in case...
	b      .L_secondary_loop

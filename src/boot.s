.section .text.boot

.global _start

_start:
.L_loop:
	wfe
	b      .L_loop

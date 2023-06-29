# os/src/entry.asm
    .section .text.entry
    .globl _start
_start:
	#stacktop- id*64KB

    # la sp, boot_stack_top
	# slli a0,a0,16
	# sub sp,sp,a0

	add     t0, a0, 1
    slli    t0, t0, 16 # 64KB, max stack size
	la 		sp, boot_stack
    add     sp, sp, t0

    call rust_main

    .section .bss.stack
    .globl boot_stack
boot_stack:
    .space 4096 * 16 *4
    .globl boot_stack_top
boot_stack_top:
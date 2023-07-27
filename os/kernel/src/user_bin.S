	.align 3
	.section .data
	.globl _app_num
_app_num:
	.quad 2
	.quad init_start
	.quad shell_start

	.globl _app_names
_app_names:
	.string "init"
	.string "shell"

	.section .data
	.global init_start
	.global init_end
	.align 3
init_start:
	.incbin "../user_c/build/init"
init_end:

	.section .data
	.global shell_start
	.global shell_end
	.align 3
shell_start:
	.incbin "../user_c/build/shell"
shell_end:


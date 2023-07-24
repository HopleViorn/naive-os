.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
	.globl __kernel_trap_entry
    .align 2
__alltraps:
    csrrw sp, sscratch, sp
    # now sp->*TrapFrame in user space, sscratch->user stack
    # save other general purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they have been saved in TrapFrame
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it in TrapFrame
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # load kernel_satp into t0
    ld t0, 34*8(sp)
    # load trap_handler into t1
    ld t1, 36*8(sp)
    # move to kernel_sp
    ld sp, 35*8(sp)
    # switch to kernel space
    csrw satp, t0
    sfence.vma
    # jump to trap_handler

	ld ra, 0(t1)
	ld sp, 8(t1)
	ld s0, 16(t1)
	ld s1, 24(t1)
	ld s2, 32(t1)
	ld s3, 40(t1)
	ld s4, 48(t1)
	ld s5, 56(t1)
	ld s6, 64(t1)
	ld s7, 72(t1)
	ld s8, 80(t1)
	ld s9, 88(t1)
	ld s10, 96(t1)
	ld s11, 104(t1)
	ret

__restore:
    # a0: *TrapFrame in user space(Constant); a1: user space token
    # switch to user space
	sd ra, 0(a0)
	sd sp, 8(a0)
	sd s0, 16(a0)
	sd s1, 24(a0)
	sd s2, 32(a0)
	sd s3, 40(a0)
	sd s4, 48(a0)
	sd s5, 56(a0)
	sd s6, 64(a0)
	sd s7, 72(a0)
	sd s8, 80(a0)
	sd s9, 88(a0)
	sd s10, 96(a0)
	sd s11, 104(a0)

    csrw satp, a1
	mv a1,a0

	li a0,0xFFFFFFFFFFFFE000

    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapFrame in user space, start restoring based on it
    # restore sstatus/sepc

	sd a1, 36*8(sp)#save &cx to trap_handler
	csrr a1,satp

    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # restore general purpose registers except x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # back to user stack
    ld sp, 2*8(sp)
    sret
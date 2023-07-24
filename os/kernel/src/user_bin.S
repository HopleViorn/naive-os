	.align 3
	.section .data
	.globl _app_num
_app_num:
	.quad 40
	.quad init_start
	.quad shell_start
	.quad forktest_start
	.quad argtest_start
	.quad runtest_start
	.quad getpid_start
	.quad getppid_start
	.quad read_start
	.quad write_start
	.quad openat_start
	.quad open_start
	.quad close_start
	.quad gettimeofday_start
	.quad sleep_start
	.quad wait_start
	.quad waitpid_start
	.quad times_start
	.quad exit_start
	.quad brk_start
	.quad execve_start
	.quad test_echo_start
	.quad fork_start
	.quad yield_start
	.quad clone_start
	.quad mount_start
	.quad umount_start
	.quad getcwd_start
	.quad getdents_start
	.quad dup_start
	.quad dup2_start
	.quad mkdir__start
	.quad chdir_start
	.quad fstat_start
	.quad uname_start
	.quad unlink_start
	.quad pipe_start
	.quad mmap_start
	.quad munmap_start
	.quad busybox_start
	.quad busybox_debug_start
	.quad busybox_debug_end

	.globl _app_names
_app_names:
	.string "init"
	.string "shell"
	.string "forktest"
	.string "argtest"
	.string "runtest"
	.string "getpid"
	.string "getppid"
	.string "read"
	.string "write"
	.string "openat"
	.string "open"
	.string "close"
	.string "gettimeofday"
	.string "sleep"
	.string "wait"
	.string "waitpid"
	.string "times"
	.string "exit"
	.string "brk"
	.string "execve"
	.string "test_echo"
	.string "fork"
	.string "yield"
	.string "clone"
	.string "mount"
	.string "umount"
	.string "getcwd"
	.string "getdents"
	.string "dup"
	.string "dup2"
	.string "mkdir_"
	.string "chdir"
	.string "fstat"
	.string "uname"
	.string "unlink"
	.string "pipe"
	.string "mmap"
	.string "munmap"
	.string "busybox"
	.string "busybox_debug"

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

	.section .data
	.global forktest_start
	.global forktest_end
	.align 3
forktest_start:
	.incbin "../user_c/build/forktest"
forktest_end:

	.section .data
	.global argtest_start
	.global argtest_end
	.align 3
argtest_start:
	.incbin "../user_c/build/argtest"
argtest_end:

	.section .data
	.global runtest_start
	.global runtest_end
	.align 3
runtest_start:
	.incbin "../user_c/build/runtest"
runtest_end:

	.global getpid_start
	.global getpid_end
	.align 3
getpid_start:
	.incbin "../testbin/getpid"
getpid_end:

	.global getppid_start
	.global getppid_end
	.align 3
getppid_start:
	.incbin "../testbin/getppid"
getppid_end:

	.global read_start
	.global read_end
	.align 3
read_start:
	.incbin "../testbin/read"
read_end:

	.global write_start
	.global write_end
	.align 3
write_start:
	.incbin "../testbin/write"
write_end:

	.global openat_start
	.global openat_end
	.align 3
openat_start:
	.incbin "../testbin/openat"
openat_end:

	.global open_start
	.global open_end
	.align 3
open_start:
	.incbin "../testbin/open"
open_end:

	.global close_start
	.global close_end
	.align 3
close_start:
	.incbin "../testbin/close"
close_end:

	.global gettimeofday_start
	.global gettimeofday_end
	.align 3
gettimeofday_start:
	.incbin "../testbin/gettimeofday"
gettimeofday_end:

	.global sleep_start
	.global sleep_end
	.align 3
sleep_start:
	.incbin "../testbin/sleep"
sleep_end:

	.global wait_start
	.global wait_end
	.align 3
wait_start:
	.incbin "../testbin/wait"
wait_end:

	.global waitpid_start
	.global waitpid_end
	.align 3
waitpid_start:
	.incbin "../testbin/waitpid"
waitpid_end:

	.global times_start
	.global times_end
	.align 3
times_start:
	.incbin "../testbin/times"
times_end:

	.global exit_start
	.global exit_end
	.align 3
exit_start:
	.incbin "../testbin/exit"
exit_end:

	.global brk_start
	.global brk_end
	.align 3
brk_start:
	.incbin "../testbin/brk"
brk_end:

	.global execve_start
	.global execve_end
	.align 3
execve_start:
	.incbin "../testbin/execve"
execve_end:

	.global test_echo_start
	.global test_echo_end
	.align 3
test_echo_start:
	.incbin "../testbin/test_echo"
test_echo_end:

	.global fork_start
	.global fork_end
	.align 3
fork_start:
	.incbin "../testbin/fork"
fork_end:

	.global yield_start
	.global yield_end
	.align 3
yield_start:
	.incbin "../testbin/yield"
yield_end:

	.global clone_start
	.global clone_end
	.align 3
clone_start:
	.incbin "../testbin/clone"
clone_end:

	.global mount_start
	.global mount_end
	.align 3
mount_start:
	.incbin "../testbin/mount"
mount_end:

	.global umount_start
	.global umount_end
	.align 3
umount_start:
	.incbin "../testbin/umount"
umount_end:

	.global getcwd_start
	.global getcwd_end
	.align 3
getcwd_start:
	.incbin "../testbin/getcwd"
getcwd_end:

	.global getdents_start
	.global getdents_end
	.align 3
getdents_start:
	.incbin "../testbin/getdents"
getdents_end:

	.global dup_start
	.global dup_end
	.align 3
dup_start:
	.incbin "../testbin/dup"
dup_end:

	.global dup2_start
	.global dup2_end
	.align 3
dup2_start:
	.incbin "../testbin/dup2"
dup2_end:

	.global mkdir__start
	.global mkdir__end
	.align 3
mkdir__start:
	.incbin "../testbin/mkdir_"
mkdir__end:

	.global chdir_start
	.global chdir_end
	.align 3
chdir_start:
	.incbin "../testbin/chdir"
chdir_end:

	.global fstat_start
	.global fstat_end
	.align 3
fstat_start:
	.incbin "../testbin/fstat"
fstat_end:

	.global uname_start
	.global uname_end
	.align 3
uname_start:
	.incbin "../testbin/uname"
uname_end:

	.global unlink_start
	.global unlink_end
	.align 3
unlink_start:
	.incbin "../testbin/unlink"
unlink_end:

	.global pipe_start
	.global pipe_end
	.align 3
pipe_start:
	.incbin "../testbin/pipe"
pipe_end:

	.global mmap_start
	.global mmap_end
	.align 3
mmap_start:
	.incbin "../testbin/mmap"
mmap_end:

	.global munmap_start
	.global munmap_end
	.align 3
munmap_start:
	.incbin "../testbin/munmap"
munmap_end:

	.global busybox_start
	.global busybox_end
	.align 3
busybox_start:
	.incbin "../testbin/busybox"
busybox_end:

	.global busybox_debug_start
	.global busybox_debug_end
	.align 3
busybox_debug_start:
	.incbin "../testbin/busybox_debug"
busybox_debug_end:


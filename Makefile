run:
	cd os/user_c;make build
	cd os/kernel;make userbin
	cd os/kernel;make run

all:
	rm -r os/kernel/.cargo
	cp -r os/kernel/cargo-judge os/kernel/.cargo
	cd os/user_c;make build
	cd os/kernel;make userbin
	cd os/kernel;make binary
	cp os/kernel/target/riscv64gc-unknown-none-elf/release/os.bin kernel-qemu
	cp bootloader/rustsbi-qemu.bin sbi-qemu

asm:all
	cd os/kernel;make asm

qemu: all
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
	-smp 2 \
	-m 128 \
    -bios sbi-qemu \
	-kernel kernel-qemu
    # -device loader,file=kernel-qemu,addr=0x80200000

local: all
	qemu-system-riscv64 \
	-machine virt \
	-nographic \
	-smp 1 \
	-m 128 \
	-bios sbi-qemu \
	-kernel kernel-qemu \
	-serial file:os_serial_out.txt

debug: all
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
	-m 128 \
    -bios sbi-qemu \
	-kernel kernel-qemu \
	-gdb tcp::12345 -S

gdb:
	riscv64-unknown-elf-gdb \
    -ex 'file $(TARGET_DIR)/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:12345'
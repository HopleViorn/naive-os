TARGET := riscv64gc-unknown-none-elf
MODE := release
APP_DIR := 
TARGET_DIR := target/$(TARGET)/$(MODE)
#target/cd
APPS := $(wildcard $(APP_DIR)/*.rs)
# ELFS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%, $(APPS))
ELF := $(TARGET_DIR)/os
# BINS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%.bin, $(APPS))
BINS := $(patsubst $(APP_DIR)/%.rs, $(TARGET_DIR)/%.bin, $(APPS))

OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

userbin:
	g++ -w ../tools/userbin_gen.cc -o ../tools/userbin_gen
	../tools/userbin_gen > src/user_bin.S

build: 
	cargo build --release 

binary:build 
	$(OBJCOPY)  $(ELF) --strip-all -O binary $(patsubst $(TARGET_DIR)/%, $(TARGET_DIR)/%.bin, $(ELF))
asm:
	$(OBJDUMP) -D $(ELF) > ../../debug/os.asm
	$(OBJDUMP) -D ../testbin/busybox_unstripped > ../../debug/busybox.asm

run: userbin binary
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
	-m 128\
    -bios ../../bootloader/rustsbi-qemu.bin \
    -device loader,file=$(TARGET_DIR)/os.bin,addr=0x80200000\

debug: build binary
	qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../../bootloader/rustsbi-qemu.bin \
    -device loader,file=$(TARGET_DIR)/os.bin,addr=0x80200000 \
    -gdb tcp::12345 -S
	# -s -S

gdb:
	riscv64-unknown-elf-gdb \
    -ex 'file $(TARGET_DIR)/os' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:12345'
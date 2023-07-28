
# APPS := $(wildcard $(APP_DIR)/*.cc)

APPS := $(wildcard src/*.cc)
# TARGETS := $(foreach n,$(APPS),$(n).
NAMES := $(patsubst src/%.cc, %, $(APPS))

clean:
	rm -rf build
build:clean
	mkdir build
	echo $(ELFS)
	$(foreach name, $(NAMES),\
		riscv64-unknown-elf-g++ -w src/$(name).cc src/lib/entry_main.c src/lib/entry.S src/lib/syscall.c -o build/$(name) -Wl,-T,src/lib/linker.ld -nostdlib;\
	)
# riscv64-unknown-elf-objdump -D build/$(name)  > build/$(name).asm;\
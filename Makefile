
ARCH ?= x86_64
LOG ?=

# do not support debug mode
MODE := release

export ARCH

OBJDUMP ?= objdump
OBJCOPY ?= objcopy

build_path := target/$(ARCH)/$(MODE)
target_elf := $(build_path)/rvm1-5
target_img := $(build_path)/jailhouse-intel.bin

build_args := --target $(ARCH).json
ifeq ($(MODE), release)
build_args += --release
endif

.PHONY: build elf hexdump clippy fmt clean

build: $(target_img)

elf:
	cargo build $(build_args)

$(target_img): elf
	$(OBJCOPY) $(target_elf) --strip-all -O binary $@

disasm:
	$(OBJDUMP) -d $(target_elf) -M intel | less

clippy:
	cargo clippy $(build_args) -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean

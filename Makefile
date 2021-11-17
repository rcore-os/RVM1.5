ARCH ?= x86_64
VENDOR ?= intel
LOG ?=

# do not support debug mode
MODE := release

export ARCH

OBJDUMP ?= objdump
OBJCOPY ?= objcopy

build_path := target/$(ARCH)/$(MODE)
target_elf := $(build_path)/rvm
target_img := $(build_path)/rvm-$(VENDOR).bin

features :=
ifeq ($(ARCH), x86_64)
ifeq ($(VENDOR), intel)
features += --features vmx
else ifeq ($(VENDOR), amd)
features += --features svm
else
$(error VENDOR must be either "intel" or "amd" for x86_64 architecture)
endif
endif

build_args := $(features) --target $(ARCH).json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
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
	cargo clippy $(build_args)

test:
	cargo test $(features)

fmt:
	cargo fmt

clean:
	cargo clean

scp:
	scp -P 2333 -r $(target_img) ubuntu@localhost:/home/ubuntu

ssh:
	ssh -p 2333 ubuntu@localhost

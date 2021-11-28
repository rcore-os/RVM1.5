# Commands:
#   make build                  Build
#   make test                   Run `cargo test`
#   make fmt                    Run `cargo fmt`
#   make clippy                 Run `cargo clippy`
#   make disasm                 Open the disassemble file of the last build
#   make clean                  Clean
#
# Options:
#   LOG  = off | error | warn | info | debug | trace
#   ARCH = x86_64
#   VENDOR = intel | amd        [ x86_64 only ] Build for Intel or AMD CPUs.
#   STATS = on | off            Given performance statistics.

ARCH ?= x86_64
VENDOR ?= intel
LOG ?=
STATS ?= off

# do not support debug mode
MODE := release

export MODE
export LOG
export ARCH
export VENDOR
export STATS

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

ifeq ($(STATS), on)
features += --features stats
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
	cargo test $(features) --release -- --nocapture

fmt:
	cargo fmt

clean:
	cargo clean

scp:
	scp -P 2335 -r $(target_img) ubuntu@192.168.50.55:~/rvm-intel-zyr.bin

ssh:
	ssh -p 2335 ubuntu@localhost
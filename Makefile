# Commands:
#   make build                  Build
#   make test                   Run `cargo test`
#   make fmt                    Run `cargo fmt`
#   make clippy                 Run `cargo clippy`
#   make disasm                 Open the disassemble file of the last build
#   make clean                  Clean
#
# Arguments:
#   LOG  = off | error | warn | info | debug | trace
#   ARCH = x86_64
#   VENDOR = intel | amd        [ x86_64 only ] Build for Intel or AMD CPUs.
#   STATS = on | off            Given performance statistics.

ARCH ?= x86_64
VENDOR ?= intel
LOG ?=
STATS ?= off
PORT ?= 2333

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
target_bin := $(build_path)/rvm-$(VENDOR).bin

ifeq ($(ARCH), x86_64)
  features := $(VENDOR)
else
  features :=
endif

ifeq ($(STATS), on)
  features += --features stats
endif

build_args := --features "$(features)" --target $(ARCH).json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem

ifeq ($(MODE), release)
  build_args += --release
endif

.PHONY: all
all: $(target_bin)

.PHONY: elf
elf:
	cargo build $(build_args)

$(target_bin): elf
	$(OBJCOPY) $(target_elf) --strip-all -O binary $@

.PHONY: disasm
disasm:
	$(OBJDUMP) -d $(target_elf) -M intel | less

.PHONY: clippy
clippy:
	cargo clippy $(build_args)

.PHONY: test
test:
	cargo test --features "$(features)" --release -- --nocapture

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: clean
clean:
	cargo clean

.PHONY: install
install:
	sudo cp $(target_bin) /lib/firmware

.PHONY: scp
scp:
	scp -P $(PORT) -r $(target_bin) ubuntu@localhost:/home/ubuntu

.PHONY: ssh
ssh:
	ssh -p $(PORT) ubuntu@localhost

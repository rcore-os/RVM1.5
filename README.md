# RVM 1.5

[![CI](https://github.com/rcore-os/RVM1.5/workflows/CI/badge.svg?branch=main)](https://github.com/rcore-os/RVM1.5/actions)

A Type-1.5 hypervisor written in Rust.

Drived by the driver from [Jailhouse](https://github.com/siemens/jailhouse).

Supported architectures: x86_64 (Intel VMX, AMD SVM).

[![Enable and disable hypervisor in RVM1.5](demo/enable-disable-hypervisor.gif)](https://asciinema.org/a/381240?autoplay=1)

## Getting Started

### Build

```
make [VENDOR=intel|amd] [LOG=warn|info|debug|trace]
```

TODO...

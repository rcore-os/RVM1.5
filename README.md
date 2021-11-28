# RVM 1.5

[![CI](https://github.com/rcore-os/RVM1.5/workflows/CI/badge.svg?branch=main)](https://github.com/rcore-os/RVM1.5/actions)

A Type-1.5 hypervisor written in Rust.

Drived by the driver from [Jailhouse](https://github.com/DeathWish5/jailhouse).

Supported architectures: x86_64 (Intel VMX, AMD SVM).

[![Enable and disable hypervisor in RVM1.5](demo/enable-disable-hypervisor.gif)](https://asciinema.org/a/381240?autoplay=1)

## Getting Started

### Build

```
make [VENDOR=intel|amd] [LOG=warn|info|debug|trace]
```

### Test in QEMU (ubuntu as the guest OS)

1. Download the guest image and run in QEMU:

    ```bash
    cd scripts/host
    make image          # download image and configure for the first time
    make qemu           # execute this command only for subsequent runs
    ```

    You can login the guest OS via SSH. The default username and password is `ubuntu` and `guest`. The default port is `2333` and can be changed by QEMU arguments.

2. Copy helpful scripts into the guest OS:

    ```bash
    scp -P 2333 scripts/guest/* ubuntu@localhost:/home/ubuntu
    ```

3. Setup in the guest OS:

    ```bash
    ssh -p 2333 ubuntu@localhost    # in host
    ./setup.sh                      # in guest
    ```

4. Copy RVM image into the guest OS:

    ```bash
    make scp                        # in host
    ```

5. Enable RVM:

    ```bash
    ./enable-rvm.sh                 # in guest
    ```

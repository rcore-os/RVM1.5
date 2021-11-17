JH=~/jailhouse/tools/jailhouse
sudo $JH disable
sudo rmmod jailhouse
sudo insmod ~/jailhouse/driver/jailhouse.ko
sudo chown $(whoami) /dev/jailhouse
sudo $JH enable ~/jailhouse/configs/x86/qemu-ubuntu.cell

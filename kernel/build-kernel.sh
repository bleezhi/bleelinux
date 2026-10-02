#!/bin/sh
# Fetch + configure kernel per arch. Borrow from kernel.org, minimal config.
set -eu
KVER=${KVER:-6.10.11}
ARCH=${ARCH:-x86_64}
DEFCONFIG=${DEFCONFIG:-x86_64_defconfig}
JOBS=$(nproc)
mkdir -p /tmp/opencode/bleelinux-kernel
cd /tmp/opencode/bleelinux-kernel
if [ ! -d linux-$KVER ]; then
  curl -L https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-$KVER.tar.xz -o linux.tar.xz
  tar -xf linux.tar.xz
fi
cd linux-$KVER
make $DEFCONFIG
./../../kernel/kconfig-fragment.sh .config ../../kernel/fragments/minimal.config
make -j$JOBS
echo "bzImage: arch/x86/boot/bzImage"

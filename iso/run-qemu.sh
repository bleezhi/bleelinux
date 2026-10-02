#!/bin/sh
# run-qemu.sh – boot bleelinux initramfs with host kernel
set -eu
K=${1:-/usr/lib/modules/$(uname -r)/vmlinuz}
I=/tmp/opencode/initramfs.cpio.zst
if [ ! -f "$I" ]; then echo "missing $I – run ./iso/mkiso.sh first"; exit 1; fi
if [ ! -f "$K" ]; then echo "missing kernel $K"; exit 1; fi
echo "booting $K + $I (Ctrl-a x to quit)"
exec qemu-system-x86_64 -kernel "$K" -initrd "$I" -nographic -m 512 \
  -append 'init=/init console=ttyS0 panic=-1'

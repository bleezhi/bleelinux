#!/bin/sh
# mkdisk.sh – persistent ext4 disk image, no root needed (mkfs.ext4 -d)
# Usage: ./iso/mkdisk.sh   (run ./iso/mkiso.sh first)
set -eu
ROOT=${ROOT:-/tmp/opencode/bleelinux-root}
DISK=${DISK:-/tmp/opencode/bleelinux.img}
SIZE=${SIZE:-512M}

qemu-img create -f raw "$DISK" "$SIZE" 2>&1 | tail -n 1
mkfs.ext4 -q -L bleelinux -d "$ROOT" "$DISK"
echo "DISK: $DISK ($(du -h "$DISK" | cut -f1))"
BZIMAGE=$(ls -t /tmp/opencode/bleelinux-kernel/linux-*/arch/x86/boot/bzImage 2>/dev/null | head -n 1)
echo "test (direct kernel, disk root):"
echo "  qemu-system-x86_64 -kernel $BZIMAGE -drive file=$DISK,format=raw,if=virtio -nographic -m 512 -append 'root=/dev/vda rw init=/sbin/init console=ttyS0 panic=-1'"

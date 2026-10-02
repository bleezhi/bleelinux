#!/bin/sh
# build-our-kernel.sh – build OUR kernel (no host vmlinuz needed)
# Usage: KVER=6.12.9 ./kernel/build-our-kernel.sh
# Output: /tmp/opencode/bleelinux-kernel/linux-$KVER/arch/x86/boot/bzImage
set -eu
KVER=${KVER:-7.2.8}
JOBS=${JOBS:-$(nproc)}
KDIR=${KDIR:-/tmp/opencode/bleelinux-kernel}
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
MAJOR=${KVER%%.*}
KPATH="v$MAJOR.x"

echo "==> need: gcc make flex bison bc openssl perl cpio xz zstd tar curl"
for t in gcc make flex bison perl openssl cpio xz zstd curl; do
  command -v $t >/dev/null 2>&1 || { echo "MISSING: $t"; MISS=1; }
done
if ! command -v bc >/dev/null 2>&1; then
  echo "MISSING: bc (kernel requires it)"
  echo "  Arch: sudo pacman -S --needed bc dwarves base-devel"
  MISS=1
fi
if [ "${MISS:-0}" = 1 ] && [ "${IGNORE_DEPS:-0}" != 1 ]; then
  echo "install missing tools and re-run (or IGNORE_DEPS=1 to try anyway)"
  exit 1
fi

mkdir -p "$KDIR"
cd "$KDIR"
if [ ! -d "linux-$KVER" ]; then
  echo "==> fetch linux-$KVER"
  curl -L "https://cdn.kernel.org/pub/linux/kernel/$KPATH/linux-$KVER.tar.xz" -o "linux-$KVER.tar.xz"
  tar -xf "linux-$KVER.tar.xz"
fi
cd "linux-$KVER"

echo "==> defconfig + bleelinux fragment"
make x86_64_defconfig
# merge our minimal fragment using in-tree script (no external dep)
./scripts/kconfig/merge_config.sh -m .config "$SCRIPT_DIR/fragments/minimal.config" "$SCRIPT_DIR/fragments/bleelinux.config"

# kill BTF/pahole requirement for Phase 1 (re-enable later for prod)
./scripts/config --disable DEBUG_INFO_BTF || true
./scripts/config --disable DEBUG_INFO || true
make olddefconfig

echo "==> build bzImage + modules (JOBS=$JOBS, ~10-25 min first time)"
# GCC 15+ turns -Wunterminated-string-initialization into an error;
# 6.12.x ACPI/PNP code predates that. Demote to warning.
export KCFLAGS="-Wno-error=unterminated-string-initialization"
make -j"$JOBS" bzImage modules

echo "OK bzImage: $KDIR/linux-$KVER/arch/x86/boot/bzImage"
ls -lh arch/x86/boot/bzImage
echo "test:"
echo "  qemu-system-x86_64 -kernel $KDIR/linux-$KVER/arch/x86/boot/bzImage -initrd /tmp/opencode/initramfs.cpio.zst -nographic -append 'init=/init console=ttyS0 panic=-1'"

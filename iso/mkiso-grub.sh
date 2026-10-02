#!/bin/sh
# mkiso-grub.sh – bootable BIOS GRUB ISO from our kernel + initramfs
# Usage: ./iso/mkiso-grub.sh   (run ./iso/mkiso.sh first)
set -eu
ISO_DIR=/tmp/opencode/iso
ISO_OUT=${ISO_OUT:-/tmp/opencode/bleelinux.iso}
BZIMAGE=${BZIMAGE:-$(ls -t /tmp/opencode/bleelinux-kernel/linux-*/arch/x86/boot/bzImage 2>/dev/null | head -n 1)}
INITRAMFS=${INITRAMFS:-/tmp/opencode/initramfs.cpio.zst}

[ -f "$BZIMAGE" ] || { echo "no bzImage (build kernel first)"; exit 1; }
[ -f "$INITRAMFS" ] || { echo "no initramfs (run iso/mkiso.sh first)"; exit 1; }

rm -rf "$ISO_DIR"
mkdir -p "$ISO_DIR/boot/grub"
cp "$BZIMAGE" "$ISO_DIR/boot/vmlinuz"
cp "$INITRAMFS" "$ISO_DIR/boot/initramfs.cpio.zst"

cat > "$ISO_DIR/boot/grub/grub.cfg" <<'EOF'
serial --speed=115200 --unit=0 --word=8 --parity=no --stop=1
terminal_input serial console
terminal_output serial console
set timeout=5
set default=0
menuentry "bleelinux" {
  linux /boot/vmlinuz init=/init console=ttyS0,115200n8 panic=-1
  initrd /boot/initramfs.cpio.zst
}
menuentry "bleelinux (busybox rescue)" {
  linux /boot/vmlinuz init=/bin/sh console=ttyS0,115200n8 panic=-1
  initrd /boot/initramfs.cpio.zst
}
EOF

grub-mkrescue -o "$ISO_OUT" "$ISO_DIR" 2>&1 | tail -n 3 || true
if [ ! -f "$ISO_OUT" ]; then
  echo "grub-mkrescue failed (usually missing mtools) – BIOS-only fallback via xorriso"
  GRUB_PC=/usr/lib/grub/i386-pc
  mkdir -p "$ISO_DIR/boot/grub/i386-pc"
  grub-mkimage -O i386-pc -o "$ISO_DIR/boot/grub/i386-pc/core.img" \
    -p /boot/grub biosdisk iso9660 normal linux serial terminal echo \
    configfile search test all_video 2>&1 | tail -n 2
  cp "$GRUB_PC"/cdboot.img "$GRUB_PC"/biosdisk.mod "$GRUB_PC"/iso9660.mod \
     "$GRUB_PC"/normal.mod "$GRUB_PC"/linux.mod "$GRUB_PC"/serial.mod \
     "$GRUB_PC"/terminal.mod "$GRUB_PC"/echo.mod "$GRUB_PC"/configfile.mod \
     "$GRUB_PC"/search.mod "$GRUB_PC"/test.mod "$GRUB_PC"/all_video.mod \
     "$ISO_DIR/boot/grub/i386-pc/" 2>/dev/null || true
  cat "$GRUB_PC/cdboot.img" "$ISO_DIR/boot/grub/i386-pc/core.img" > "$ISO_DIR/boot/grub/i386-pc/eltorito.img"
  xorriso -as mkisofs -R -J -V BLEELINUX \
    -b boot/grub/i386-pc/eltorito.img -no-emul-boot -boot-load-size 4 -boot-info-table \
    -o "$ISO_OUT" "$ISO_DIR" 2>&1 | tail -n 2
fi
echo "ISO: $ISO_OUT ($(du -h "$ISO_OUT" | cut -f1))"
echo "test: qemu-system-x86_64 -cdrom $ISO_OUT -nographic -m 512"

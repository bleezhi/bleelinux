#!/bin/sh
# mkiso.sh – build bootable minimal image (UEFI UKI + BIOS GRUB)
# Phase 1: x86_64 glibc only, QEMU-tested
set -eu
ROOT=${ROOT:-/tmp/opencode/bleelinux-root}
KVER=${KVER:-6.10.11}
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SRC_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)

mkdir -p "$ROOT"/{proc,sys,dev,run,boot,sbin,bin,lib,lib64,usr/bin,usr/lib,etc/bleepinit,etc/bleelinux,var/lib/bleepkg,var/cache/bleep,repo,root}
mkdir -p "$ROOT"/root/.config/bleelinux

# pick built binaries (release preferred, debug fallback)
pick_bin() {
  if [ -f "$SRC_DIR/target/release/$1" ]; then echo "$SRC_DIR/target/release/$1";
  elif [ -f "$SRC_DIR/target/debug/$1" ]; then echo "$SRC_DIR/target/debug/$1";
  else echo ""; fi
}
BLEEP=$(pick_bin bleep)
BLEEPINIT=$(pick_bin bleepinit)
BLEESH=$(pick_bin bleesh)
if [ -n "$BLEEP" ]; then echo "install $BLEEP -> $ROOT/usr/bin/bleep"; cp "$BLEEP" "$ROOT/usr/bin/bleep"; else echo "WARN: no bleep binary (run cargo build first)"; fi
if [ -n "$BLEEPINIT" ]; then echo "install $BLEEPINIT -> $ROOT/sbin/init"; cp "$BLEEPINIT" "$ROOT/sbin/init"; else echo "WARN: no bleepinit binary"; fi
if [ -n "$BLEESH" ]; then echo "install $BLEESH -> $ROOT/usr/bin/bleesh"; cp "$BLEESH" "$ROOT/usr/bin/bleesh"; fi
# kernel initramfs looks for /init first – symlink it
ln -sf sbin/init "$ROOT/init" 2>/dev/null || true
ln -sf ../../sbin/init "$ROOT/usr/bin/init" 2>/dev/null || true

# shell: prefer host busybox, else fetch static busybox, else host bash
if command -v busybox >/dev/null 2>&1; then
  echo "using host busybox"
  cp "$(command -v busybox)" "$ROOT/usr/bin/busybox"
elif [ ! -x "$ROOT/usr/bin/busybox" ]; then
  echo "fetching static busybox..."
  mkdir -p /tmp/opencode
  if curl -Lf -o /tmp/opencode/busybox https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox; then
    chmod +x /tmp/opencode/busybox
    cp /tmp/opencode/busybox "$ROOT/usr/bin/busybox"
  else
    echo "WARN: busybox download failed, falling back to host bash"
    cp /bin/bash "$ROOT/usr/bin/bash" 2>/dev/null || cp /usr/bin/bash "$ROOT/usr/bin/bash" || true
  fi
fi
# /bin/sh always exists for init scripts
mkdir -p "$ROOT/bin"
ln -sf ../usr/bin/busybox "$ROOT/bin/sh" 2>/dev/null || true
ln -sf busybox "$ROOT/usr/bin/sh" 2>/dev/null || true
# busybox applet symlinks so plain `ls`, `ps`… work with a minimal PATH
if [ -x "$ROOT/usr/bin/busybox" ]; then
  for app in $("$ROOT/usr/bin/busybox" --list 2>/dev/null | tr -d '\r'); do
    case "$app" in
      sh|init|linuxrc) continue ;;
    esac
    [ -e "$ROOT/bin/$app" ] || ln -s /usr/bin/busybox "$ROOT/bin/$app" 2>/dev/null || true
  done
  echo "applets: $(ls "$ROOT/bin" | wc -l) in /bin"
fi

# copy shared-lib deps for dynamic binaries (bleep, bleepinit, bash)
copy_deps() {
  bin="$1"
  [ -f "$bin" ] || return 0
  # ldd prints '=> /path' or '/lib64/ld-... (0x...)'
  ldd "$bin" 2>/dev/null | grep -o '/[^ ()]*' | while read -r lib; do
    [ -f "$lib" ] || continue
    dest="$ROOT$lib"
    if [ ! -f "$dest" ]; then
      mkdir -p "$(dirname "$dest")"
      echo "lib $lib"
      cp -L "$lib" "$dest"
    fi
  done
  # loader symlink target
  for ld in /lib64/ld-linux-x86-64.so.2 /lib/ld-linux-x86-64.so.2; do
    if [ -e "$ld" ] && [ ! -e "$ROOT$ld" ]; then
      mkdir -p "$(dirname "$ROOT$ld")"
      cp -L "$ld" "$ROOT$ld" 2>/dev/null || true
    fi
  done
  mkdir -p "$ROOT/lib64"
  [ -e "$ROOT/lib64/ld-linux-x86-64.so.2" ] || cp -L /lib64/ld-linux-x86-64.so.2 "$ROOT/lib64/" 2>/dev/null || true
}
if [ -n "$BLEEP" ]; then copy_deps "$BLEEP"; fi
if [ -n "$BLEEPINIT" ]; then copy_deps "$BLEEPINIT"; fi
if [ -n "$BLEESH" ]; then copy_deps "$BLEESH"; fi
if [ -f "$ROOT/usr/bin/bash" ]; then copy_deps "$ROOT/usr/bin/bash"; fi

# minimal /etc
[ -f "$ROOT/etc/passwd" ] || echo 'root:x:0:0:root:/root:/bin/sh' > "$ROOT/etc/passwd"
[ -f "$ROOT/etc/group" ] || echo 'root:x:0:' > "$ROOT/etc/group"

cat > "$ROOT/etc/bleepinit/init.toml" <<'EOF'
default_target = "multi-user.target"
EOF
cat > "$ROOT/etc/bleepinit/sh.toml" <<'EOF'
name = "sh"
description = "bleesh console shell"
exec = ["/usr/bin/bleesh"]
type = "simple"
restart = "always"
wanted_by = ["multi-user.target"]
EOF
cat > "$ROOT/etc/bleepinit/rescue.toml" <<'EOF'
name = "rescue"
description = "busybox rescue shell (disabled by default)"
exec = ["/usr/bin/busybox", "sh"]
type = "simple"
restart = "always"
wanted_by = ["rescue.target"]
EOF

# guest package config: local file:// repo seeded below
cat > "$ROOT/etc/bleelinux/config.toml" <<'EOF'
root_dir = "/var/lib/bleep"
db_path = "/var/lib/bleepkg/db.sqlite"
cache_dir = "/var/cache/bleep"
build_dir = "/tmp/bleep-build"
recipes_dir = "/repo/recipes"
hooks_dir = "/etc/bleepkg/hooks"
target_arch = "x86-64"
target_libc = "glibc"
target_triplet = "x86_64-unknown-linux-gnu"
parallel_jobs = 2
verify_signatures = false
trusted_keys = []
colors = false
progress_bars = false
[[repos]]
name = "local"
url = "file:///repo"
enabled = true
arch = "x86-64"
libc = "glibc"
priority = 10
verify_signatures = false
EOF
cp "$ROOT/etc/bleelinux/config.toml" "$ROOT/root/.config/bleelinux/config.toml"

# demo package + repo index (proves bleep end-to-end in the guest)
if [ -n "${BLEEP:-}" ] && [ -x "$BLEEP" ]; then
  mkdir -p /tmp/opencode/demo-pkg/usr/bin "$ROOT/repo"
  printf '#!/bin/sh\necho hello-bleelinux\n' > /tmp/opencode/demo-pkg/usr/bin/hello
  chmod +x /tmp/opencode/demo-pkg/usr/bin/hello
  "$BLEEP" pack /tmp/opencode/demo-pkg --name hello --version 1.0.0 -o "$ROOT/repo" >/dev/null
  "$BLEEP" repo-index "$ROOT/repo" >/dev/null
  echo "repo: $(ls "$ROOT/repo" | tr '\n' ' ')"
fi

# kernel image into /boot if our build exists
OUR_KIMG=$(ls -t /tmp/opencode/bleelinux-kernel/linux-*/arch/x86/boot/bzImage 2>/dev/null | head -n 1 || true)
if [ -n "$OUR_KIMG" ]; then
  echo "kernel $OUR_KIMG -> $ROOT/boot/vmlinuz"
  cp "$OUR_KIMG" "$ROOT/boot/vmlinuz"
fi

# initramfs
if [ -d "$ROOT" ]; then
  (cd "$ROOT" && find . -print0 | cpio --format=newc --create --null | zstd -19 > /tmp/opencode/initramfs.cpio.zst)
  echo "initramfs: /tmp/opencode/initramfs.cpio.zst ($(du -h /tmp/opencode/initramfs.cpio.zst | cut -f1))"
  echo "root size: $(du -sh "$ROOT" | cut -f1), files: $(cd "$ROOT" && find . | wc -l)"
fi

# kernel for QEMU: prefer /usr/lib/modules/$(uname -r)/vmlinuz (Arch), else custom build
HOST_KVER=$(uname -r)
if [ -f "/usr/lib/modules/$HOST_KVER/vmlinuz" ]; then
  echo "host kernel: /usr/lib/modules/$HOST_KVER/vmlinuz"
  echo "test boot:"
  echo "  qemu-system-x86_64 -kernel /usr/lib/modules/$HOST_KVER/vmlinuz -initrd /tmp/opencode/initramfs.cpio.zst -nographic -append 'init=/init console=ttyS0 panic=-1'"
elif [ -f "/tmp/opencode/bleelinux-kernel/linux-$KVER/arch/x86/boot/bzImage" ]; then
  echo "custom kernel: /tmp/opencode/bleelinux-kernel/linux-$KVER/arch/x86/boot/bzImage"
fi

# UKI via ukify if available
if command -v ukify >/dev/null 2>&1 && [ -f "/tmp/opencode/bleelinux-kernel/linux-$KVER/arch/x86/boot/bzImage" ]; then
  ukify build --linux="/tmp/opencode/bleelinux-kernel/linux-$KVER/arch/x86/boot/bzImage" \
    --initrd=/tmp/opencode/initramfs.cpio.zst \
    --cmdline="init=/init console=ttyS0" \
    --output=/tmp/opencode/bleelinux.efi
  echo "UKI: /tmp/opencode/bleelinux.efi"
fi
echo "done. root=$ROOT"

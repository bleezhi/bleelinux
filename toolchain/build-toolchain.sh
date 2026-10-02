#!/bin/sh
# Build cross toolchain with crosstool-ng or system clang
# Usage: ./build-toolchain.sh <arch> <libc>
#  arch: x86_64 | aarch64 | riscv64 | ppc64le
#  libc: gnu | musl
set -eu
ARCH=${1:-x86_64}
LIBC=${2:-gnu}
TRIPLET=""
case "$ARCH-$LIBC" in
  x86_64-gnu) TRIPLET="x86_64-unknown-linux-gnu" ;;
  x86_64-musl) TRIPLET="x86_64-unknown-linux-musl" ;;
  aarch64-gnu) TRIPLET="aarch64-unknown-linux-gnu" ;;
  aarch64-musl) TRIPLET="aarch64-unknown-linux-musl" ;;
  riscv64-gnu) TRIPLET="riscv64-unknown-linux-gnu" ;;
  riscv64-musl) TRIPLET="riscv64-unknown-linux-musl" ;;
  ppc64le-gnu) TRIPLET="powerpc64le-unknown-linux-gnu" ;;
  *) echo "unknown $ARCH-$LIBC"; exit 1 ;;
esac
echo "==> target $TRIPLET"
echo "1) host tools: clang, lld, rust, go, meson, cmake, ninja, bison, flex"
echo "2) musl: use system musl-gcc or build musl-cross-make"
echo "   git clone https://github.com/richfelker/musl-cross-make && make TARGET=$TRIPLET -j$(nproc)"
echo "3) glibc: use crosstool-ng sample $ARCH-unknown-linux-gnu"
echo "4) rust: rustup target add $ARCH-unknown-linux-${LIBC/gnu/gnu}"
echo "Toolchain prefix: /opt/bleelinux/$TRIPLET"

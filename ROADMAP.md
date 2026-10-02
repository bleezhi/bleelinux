# Bleelinux - Independent Multi-Arch Linux Distro

## Vision
- Independent distro with multi-arch support (x86_64, aarch64, riscv64, ppc64le)
- Minimal base + custom package manager with Nix-like features
- Custom init system with universal compat layer (systemd, OpenRC, runit, s6)
- Dual libc support (glibc + musl)
- Hybrid package model: binary by default, source-from-git on demand

## Architecture Overview

```
bleelinux/
├── toolchain/          # Cross-compilation toolchains per arch
├── kernel/             # Kernel config + patches per arch
├── init/               # Custom PID 1 + service manager + compat shims
├── pkgmgr/             # Package manager (binary + source-from-git)
├── buildsys/           # Package build system (xbps-src style + Nix derivations)
├── repos/              # Package repositories per arch + libc
├── iso/                # Installer / live image builder
├── docs/               # Documentation
└── ci/                 # CI/CD pipelines
```

## Phase 1: Bootable Minimal Image (Target: 1 arch, 1 libc)

### Week 1-2: Toolchain & Kernel
- [ ] Set up cross-compilation toolchain (musl + glibc) for x86_64
- [ ] Kernel config: minimal, modular, common hardware support
- [ ] Build kernel + modules + firmware blobs

### Week 3: Custom Init System (PID 1)
- [ ] Minimal PID 1 in Rust: mount /proc, /sys, /dev, start service manager
- [ ] Service manager: supervision, dependency graph, socket activation
- [ ] Compat layer: systemd unit parser → native service defs

### Week 4: Package Manager Core
- [ ] Binary package format: `.bleepkg` (tar.zst + metadata)
- [ ] Repository index format (SQLite + signed)
- [ ] Install/remove/upgrade with transaction safety
- [ ] Dependency resolver (SAT solver, Nix-like)

### Week 5: Build System
- [ ] Template format (shell + declarative metadata)
- [ ] Hermetic build containers (bubblewrap/namespaces)
- [ ] Cross-compile support built-in
- [ ] Binary cache (local + remote)

### Week 6: Base Packages
- [ ] musl/glibc, busybox/toybox, kernel, init, pkgmgr, buildsys
- [ ] Bootstrap: build packages with host tools → rebuild with self

### Week 7: Bootable Image
- [ ] Initramfs with pkgmgr + network
- [ ] UKI (Unified Kernel Image) for UEFI
-   GRUB for BIOS
- [ ] Test boot in QEMU + real hardware

## Phase 2: Multi-Arch + Dual Libc

- [ ] Add aarch64, riscv64, ppc64le toolchains
- [ ] Per-arch kernel configs
- [ ] Multi-libc package variants (glibc/musl suffixes)
- [ ] Cross-build CI pipeline

## Phase 3: Source-from-Git Feature

- [ ] Git source fetcher (shallow clone, pinned commits)
- [ ] Build-from-source flow integrated in pkgmgr
- [ ] User prompt: "Binary available. Compile from git? [Y/n]"
- [ ] Local overlay for custom compile flags

## Phase 4: Universal Init Compat

- [ ] systemd unit → native service compiler
- [ ] OpenRC/runit/s6 service file parsers
- [ ] Socket activation compat (systemd-compatible API)
- [ ] journald/logind compatible APIs

## Phase 5: Installer & Desktop

- [ ] TUI installer (partitioning, encryption, network, users)
- [ ] Live ISO with Calamares-like flow
- [ ] Desktop metapackages (KDE, GNOME, sway, etc.)

## Technical Decisions

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language (core) | Rust | Safety, cross-compile, modern tooling |
| Package format | tar.zst + JSON metadata | Simple, streaming, reproducible |
| Repo index | SQLite + Minisign signatures | Queryable, verifiable, single file |
| Build isolation | bubblewrap + overlayfs | Rootless, fast, hermetic |
| Init IPC | D-Bus + custom Unix sockets | Standard + lightweight |
| Config format | TOML (packages), YAML (services) | Human-readable, typed |

## Borrowed Components (Upstream)

| Component | Source | Integration |
|-----------|--------|-------------|
| Kernel | kernel.org + distro patches | Config + patches only |
| Compiler | LLVM/Clang + GCC | Full toolchain per arch |
| Binutils | GNU binutils | Part of toolchain |
| Firmware | linux-firmware.git | Packaged as-is |
| Crypto | RustCrypto / OpenSSL | Via Cargo / system |

## Package Manager UX (Source-from-Git)

```
$ bleep install firefox
Binary package available (firefox-128.0-1-x86_64.glibc.bleepkg)
Compile from git instead? [y/N]: y
→ Cloning https://github.com/mozilla/gecko-dev (pinned to 128.0)
→ Building with your CFLAGS: -march=native -O3 -pipe
→ Installing firefox-128.0-1-x86_64.glibc.bleepkg (local build)
```

## Next Steps

1. **Initialize repo structure** with Cargo workspaces
2. **Set up cross-toolchain** (crosstool-ng or manual)
3. **Prototype PID 1** in Rust (100 lines → mount → exec)
4. **Design package metadata schema** (TOML)
5. **Write first package templates** (musl, busybox, kernel, init)
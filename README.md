# Bleelinux

Independent multi-arch Linux distro. Minimal base + custom package manager + custom init with universal compat.

## Status: Phase 1 scaffold (compiles, not yet bootable)

- `crates/bleepkg` – `.bleepkg` format, metadata, SQLite DB, verify
- `crates/bleepm` – `bleep` CLI: install/remove/upgrade/search/info/build/repo/verify
- `crates/bleepinit` – `bleepinit` PID 1 + supervisor + systemd/OpenRC/runit/s6 parsers
- `toolchain/` – cross toolchains per arch/libc
- `kernel/` – kernel configs
- `repos/core/` – package recipes
- `iso/` – live/installer image builder
- `buildsys/` – hermetic build docs

## Quick start

```sh
cargo check
cargo build -p bleepm --bin bleep
cargo build -p bleepinit --bin bleepinit
./target/debug/bleep --help
```

## Roadmap
See `ROADMAP.md`.

## Layout
```
toolchain/  crosstool configs + triplets
kernel/     defconfigs per arch
repos/core/<pkg>/recipe.toml
iso/        mkiso.sh, initramfs
buildsys/   BUILD.md
```

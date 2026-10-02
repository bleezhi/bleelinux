# Build system: hermetic, reproducible, cross-aware

Template = `repos/core/<pkg>/recipe.toml` + optional `build.sh` + `patches/*.patch`.

Flow (`bleep build <pkg>`):
1. fetch source (Url sha256 / Git pinned rev / Local)
2. verify sha256
3. enter bubblewrap: `--unshare-all --ro-bind /usr --tmpfs /tmp --dir $work`
   env: CFLAGS, LDFLAGS (PIE/RELRO/FORTIFY/LTO), CC=$TRIPLET-gcc if cross
4. detect: configure / meson / cmake / cargo / make
5. `DESTDIR=$staging install`
6. strip (unless no_strip), split -doc/-locale/-dev (unless disabled)
7. pack `.bleepkg` (tar.zst + .METADATA.toml + .FILES.toml), sign with minisign
8. upload to binary cache

Reproducibility: SOURCE_DATE_EPOCH, zero uid/gid, sorted tar, `-Wdate-time -D__DATE__`.

Cross: `target_triplet` from toolchain/triplets.txt. All recipes must support
`cross_compile=true` or be marked host-only.

Binary vs git prompt (UX):
```
$ bleep install firefox
Binary available. Compile from git? [y/N]:
```
Implemented in bleepm: `InstallArgs --from-git --git-url`, `BuildArgs --git`.

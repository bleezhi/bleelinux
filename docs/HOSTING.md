# Hosting packages for $0 (no VPS)

`bleep` fetches over plain HTTPS, so any static file host works.
A repo is just a directory served over HTTP(S):

```
https://<host>/<tag>/index.json.zst
https://<host>/<tag>/<name>-<ver>-<rel>.<arch>.<libc>.bin.bleepkg
```

`bleep` builds both URLs as `{repo_url}/…`, so a GitHub Release works
directly — release assets live at
`https://github.com/<org>/<repo>/releases/download/<tag>/…`.

## Option A: GitHub repo + Releases (easiest, $0)

1. Create repo `bleezhi/packages`.
2. Build + index + sign locally or in Actions:
```sh
bleep pack ./stage-hello --name hello --version 1.0.0 -o ./out
OUT=./out ./repos/publish.sh
```
3. Publish as release assets:
```sh
gh release create core-x86_64-glibc --repo bleezhi/packages --title "core x86_64 glibc" --notes "nightly"
OUT=./out TAG=core-x86_64-glibc ./repos/publish.sh --upload
```
4. Point bleep at it:
```sh
bleep repo remove core
bleep repo add core https://github.com/bleezhi/packages/releases/download/core-x86_64-glibc
bleep repo refresh
bleep search hello
```
One tag per `<repo>-<arch>-<libc>` lane (e.g. `core-aarch64-glibc`).
CI (`.github/workflows/packages.yml`) rebuilds on push, uploads with
`--clobber`. It writes to `bleezhi/packages`, so it needs a classic PAT
with `repo` scope stored as the `PACKAGES_TOKEN` secret (`GITHUB_TOKEN`
can't cross repos) — same PAT the upload page asks for. Signing: minisign
private key in Actions secrets (`MINISIGN_KEY`), public key shipped in
`docs/keys/`.

## Option B: Cloudflare R2 ($0, no bandwidth bills)

Same layout, `rclone copy ./out r2:$BUCKET/<tag>/`, optional CDN +
custom domain (`packages.bleelinux.org`). 10 GB free, zero egress fees.
Fall back to GitHub Releases as mirror — `bleep` tries repos by priority.

## User repos (like AUR/PPA, no gatekeeping)

Anyone hosts a directory the same way and shares one line:
```sh
bleep repo add bleez-stuff https://github.com/bleez/stuff/releases/download/main
```
`bleep repo {list,add,remove,enable,disable}` persists to
`~/.config/bleelinux/config.toml` (or `--config` path, or
`/etc/bleelinux/config.toml` system-wide). Priority decides ties;
`suspend` a repo with `disable` instead of deleting.

## Trust

`verify_signatures = true` + `public_key` per repo is the goal;
today downloads are checksum-recorded but signature enforcement on the
install path is still TODO (see `crates/bleepkg/src/verify.rs`).
Until then: HTTPS origin + pinned release tags.

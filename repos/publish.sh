#!/bin/sh
# publish.sh – sign + index + upload to GitHub Releases or R2 ($0 infra)
# Usage:
#   ./repos/publish.sh                          # build index for ./out, print upload cmds
#   OUT=./out TAG=core-x86_64-glibc ./repos/publish.sh --upload
set -eu
OUT=${OUT:-./out}
TAG=${TAG:-nightly}
REPO_DIR=${REPO_DIR:-"$OUT"}
PKG_REPO=${PKG_REPO:-bleezhi/packages}
BLEEP=${BLEEP:-./target/release/bleep}

if [ ! -x "$BLEEP" ]; then BLEEP=./target/debug/bleep; fi

echo "==> index"
"$BLEEP" repo-index "$REPO_DIR" 2>/dev/null || {
  echo "bleep binary missing – run: cargo build --release -p bleepm"; exit 1;
}

echo "==> sign (skipped unless MINISIGN_KEY is set)"
if [ -n "${MINISIGN_KEY:-}" ] && command -v minisign >/dev/null 2>&1; then
  for p in "$REPO_DIR"/*.bleepkg; do
    [ -f "$p" ] || continue
    minisign -Sm "$p" -s "$MINISIGN_KEY"
  done
else
  echo "  (no MINISIGN_KEY – assets unsigned; set it in CI secrets)"
fi

ls -lh "$REPO_DIR"
if [ "${1:-}" = "--upload" ]; then
  if [ -n "${R2_BUCKET:-}" ] && command -v rclone >/dev/null 2>&1; then
   DST="${R2_PREFIX:-}${TAG}"
    echo "==> rclone copy $REPO_DIR -> r2:$R2_BUCKET/$DST/"
    rclone copy "$REPO_DIR" "r2:$R2_BUCKET/$DST/"
    if [ -n "${R2_PUBLIC:-}" ]; then
      echo "live at: ${R2_PUBLIC%/}/$DST/"
      echo "wire up bleep with:"
      echo "  bleep repo add core ${R2_PUBLIC%/}/$DST"
    fi
  else
    echo "==> gh release upload $TAG -> $PKG_REPO"
    gh release upload "$TAG" --repo "$PKG_REPO" \
      "$REPO_DIR"/*.bleepkg "$REPO_DIR"/index.json.zst "$REPO_DIR"/packages.json \
      --clobber
    echo "live at: https://github.com/$PKG_REPO/releases/download/$TAG/"
    echo "  bleep repo add ${TAG%%-*} https://github.com/$PKG_REPO/releases/download/$TAG"
  fi
else
  echo "dry run – re-run with --upload to publish"
  echo "  gh release upload $TAG --repo $PKG_REPO $REPO_DIR/*.bleepkg $REPO_DIR/index.json.zst $REPO_DIR/packages.json --clobber"
fi

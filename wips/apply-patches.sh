#!/usr/bin/env bash
# Primjenjuje WiPS patcheve na upstream RustDesk stablo (pokreće se u CI-ju prije builda).
#   wips/patches/rustdesk/*.patch    -> korijen repozitorija
#   wips/patches/hbb_common/*.patch  -> libs/hbb_common (submodul)
#   wips/branding/                   -> ikone (kad postoje)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

apply_dir() {
  local dir="$1" target="$2"
  shopt -s nullglob
  local patches=("$dir"/*.patch)
  shopt -u nullglob
  if [ ${#patches[@]} -eq 0 ]; then
    echo "nema patcheva u $dir"
    return
  fi
  for p in "${patches[@]}"; do
    echo "primjenjujem $p -> $target"
    git -C "$target" apply --check "$ROOT/$p"
    git -C "$target" apply "$ROOT/$p"
  done
}

apply_dir wips/patches/rustdesk .
apply_dir wips/patches/hbb_common libs/hbb_common

if [ -x wips/branding/apply-branding.sh ]; then
  wips/branding/apply-branding.sh
fi
echo "patchevi primijenjeni"

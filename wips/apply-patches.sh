#!/usr/bin/env bash
# Primjenjuje WiPS patcheve na upstream RustDesk stablo (pokreće se u CI-ju prije builda).
#   upotreba: wips/apply-patches.sh <varijanta>      (wips/varijante/<varijanta>.env: korisnik, tehnicar)
#   wips/patches/app/*.patch         -> korijen repozitorija (ne 'rustdesk/': upstream .gitignore ignorira tu riječ)
#   wips/patches/hbb_common/*.patch  -> libs/hbb_common (submodul)
#   wips/branding/                   -> ikone (kad postoje)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VARIJANTA="${1:-}"
if [ -z "$VARIJANTA" ] || [ ! -f "wips/varijante/$VARIJANTA.env" ]; then
  echo "upotreba: wips/apply-patches.sh <varijanta>  (varijante: $(cd wips/varijante && ls *.env | sed 's/\.env$//' | tr '\n' ' '))" >&2
  exit 2
fi
echo "verzija: $VARIJANTA"

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

apply_dir wips/patches/app .
apply_dir wips/patches/hbb_common libs/hbb_common

# zamjena @@WIPS_*@@ oznaka vrijednostima iz wips/server.env i wips/varijante/<varijanta>.env
python3 - "wips/varijante/$VARIJANTA.env" <<'EOF'
import pathlib, re, subprocess, sys
env = {}
for path in ["wips/server.env", sys.argv[1]]:
    for line in pathlib.Path(path).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            k, v = line.split("=", 1)
            env[k.strip()] = v.strip()
files = subprocess.run(["git", "grep", "-l", "@@WIPS_"], capture_output=True, text=True).stdout.split()
files += ["libs/hbb_common/" + f for f in subprocess.run(
    ["git", "-C", "libs/hbb_common", "grep", "-l", "@@WIPS_"], capture_output=True, text=True).stdout.split()]
files = [f for f in files if not f.startswith("wips/")]
for f in files:
    p = pathlib.Path(f)
    s = p.read_text(encoding="utf-8")
    for k, v in env.items():
        s = s.replace(f"@@{k}@@", v)
    left = re.findall(r"@@WIPS_[A-Z_]+@@", s)
    if left:
        sys.exit(f"nezamijenjene oznake u {f}: {left}")
    p.write_text(s, encoding="utf-8")
    print(f"zamijenjeno: {f}")
if not files:
    sys.exit("nijedna @@WIPS_ oznaka nije pronađena - jesu li patchevi primijenjeni?")
EOF

if [ -x wips/branding/apply-branding.sh ]; then
  wips/branding/apply-branding.sh
fi
echo "patchevi primijenjeni"

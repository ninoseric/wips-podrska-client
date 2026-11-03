#!/usr/bin/env bash
# Kopira ikone iz wips/branding/generirano/ (generiraj-ikone.py) preko upstream ikona. Poziva ga apply-patches.sh.
set -euo pipefail
cd "$(dirname "$0")/../.."
G=wips/branding/generirano
cp "$G/app.ico"       res/icon.ico                                   # exe koji korisnik preuzme (libs/portable)
cp "$G/app.ico"       flutter/windows/runner/resources/app_icon.ico  # glavni program, prozori, prečaci
cp "$G/tray.ico"      res/tray-icon.ico                              # ikona u traci (src/tray.rs)
cp "$G/icon.png"      flutter/assets/icon.png                        # mala ikona u aplikaciji (loadIcon)
cp "$G/logo.png"      flutter/assets/logo.png                        # logo na početnom ekranu, svijetla tema
cp "$G/logo_dark.png" flutter/assets/logo_dark.png                   # logo na početnom ekranu, tamna tema
echo "ikone kopirane"
# Font Manrope (OFL, wips/branding/fonts/OFL.txt) za početni prozor korisničke verzije (patch app/0004, pubspec.yaml)
mkdir -p flutter/assets/fonts
cp wips/branding/fonts/Manrope-*.ttf flutter/assets/fonts/
echo "fontovi kopirani"

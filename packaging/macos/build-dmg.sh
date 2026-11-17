#!/usr/bin/env bash
# Builds Lattice.app and a disk image from a release binary.
#
#   packaging/macos/build-dmg.sh <binary> <version> <arch> [output-dir]
#
# arch is arm64 or x86_64. The bundle is signed ad hoc, which is what an Apple
# Silicon binary needs to run at all. It is not notarised, so Gatekeeper will
# still ask the user to confirm the first launch.
set -euo pipefail

binary=${1:?usage: build-dmg.sh <binary> <version> <arch> [output-dir]}
version=${2:?usage: build-dmg.sh <binary> <version> <arch> [output-dir]}
arch=${3:?usage: build-dmg.sh <binary> <version> <arch> [output-dir]}
out=${4:-dist}

root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$out"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

app="$work/Lattice.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

install -m755 "$binary" "$app/Contents/MacOS/Lattice"
install -m644 "$root/packaging/icons/lattice.icns" "$app/Contents/Resources/lattice.icns"
sed "s/@VERSION@/$version/g" "$root/packaging/macos/Info.plist" > "$app/Contents/Info.plist"
install -m644 "$root/LICENSE" "$app/Contents/Resources/LICENSE"

codesign --force --deep --sign - "$app"
codesign --verify --verbose=2 "$app"

dmg_root="$work/dmg"
mkdir -p "$dmg_root"
cp -R "$app" "$dmg_root/"
ln -s /Applications "$dmg_root/Applications"

dmg="$out/lattice-$version-macos-$arch.dmg"
hdiutil create -volname "Lattice $version" -srcfolder "$dmg_root" -ov -format UDZO "$dmg" >/dev/null
echo "wrote $dmg"

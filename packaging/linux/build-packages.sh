#!/usr/bin/env bash
# Builds the Debian package and a portable tarball from a release binary.
#
#   packaging/linux/build-packages.sh <binary> <version> [output-dir]
#
# Produces lattice_<version>_amd64.deb and lattice-<version>-linux-x86_64.tar.gz.
set -euo pipefail

binary=${1:?usage: build-packages.sh <binary> <version> [output-dir]}
version=${2:?usage: build-packages.sh <binary> <version> [output-dir]}
out=${3:-dist}

root=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$out"

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

# Layout inside the package.
install -Dm755 "$binary" "$stage/usr/bin/lattice"
install -Dm644 "$root/packaging/linux/lattice.desktop" \
  "$stage/usr/share/applications/lattice.desktop"
install -Dm644 "$root/packaging/icons/lattice-256.png" \
  "$stage/usr/share/icons/hicolor/256x256/apps/lattice.png"
install -Dm644 "$root/LICENSE" \
  "$stage/usr/share/doc/lattice/copyright"

mkdir -p "$stage/DEBIAN"
cat > "$stage/DEBIAN/control" <<EOF
Package: lattice
Version: $version
Section: graphics
Priority: optional
Architecture: amd64
Installed-Size: $(du -sk "$stage/usr" | cut -f1)
Maintainer: bimoalfarrabi <bimoalfarrabi24@gmail.com>
Homepage: https://github.com/bimoalfarrabi/lattice
Depends: libxkbcommon-x11-0, libxcb1, libx11-6, libfontconfig1
Description: Grid logo maker
 Paint cells on a generated lattice and export SVG or PNG.
 31 lattices, an infinite mirrored canvas, undo, preview, zoom
 and two themes.
EOF

deb="$out/lattice_${version}_amd64.deb"
dpkg-deb --root-owner-group --build "$stage" "$deb" >/dev/null
echo "wrote $deb"

# Portable tarball: binary, desktop entry, icon.
portable=$(mktemp -d)
trap 'rm -rf "$stage" "$portable"' EXIT
mkdir -p "$portable/lattice-$version"
install -m755 "$binary" "$portable/lattice-$version/lattice"
install -m644 "$root/packaging/linux/lattice.desktop" \
  "$portable/lattice-$version/lattice.desktop"
install -m644 "$root/packaging/icons/lattice-256.png" \
  "$portable/lattice-$version/lattice.png"
install -m644 "$root/LICENSE" "$portable/lattice-$version/LICENSE"
tar -C "$portable" -czf "$out/lattice-$version-linux-x86_64.tar.gz" "lattice-$version"
echo "wrote $out/lattice-$version-linux-x86_64.tar.gz"

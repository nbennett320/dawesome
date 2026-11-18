#!/bin/sh
# build the project files' icons (dawesome-document.icns and
# sharedawesome-document.icns, in engine/icons) from the SVGs here. macOS
# only: it needs swiftc, iconutil and ImageMagick (magick)
#
# 16 and 32 pixels are drawn from the -small SVGs, which stay crisp that
# small. 128 pixels and up get a soft shadow, as Finder's icons have
set -eu
here=$(cd "$(dirname "$0")" && pwd)
out=$(cd "$here/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

swiftc -O "$here/render-svg.swift" -o "$work/render-svg"

# draw `svg` at `px` pixels into `png`, with a shadow when it's big enough
draw() {
  svg=$1 png=$2 px=$3
  "$work/render-svg" "$svg" "$work/plain.png" "$px"
  if [ "$px" -ge 128 ]; then
    blur=$((px / 64)) drop=$((px / 96))
    magick "$work/plain.png" \
      \( +clone -background black -shadow "35x${blur}+0+${drop}" \) +swap \
      -background none -layers merge +repage \
      -gravity center -crop "${px}x${px}+0+0" +repage "$png"
  else
    cp "$work/plain.png" "$png"
  fi
}

for name in dawesome-document sharedawesome-document; do
  set="$work/$name.iconset"
  mkdir -p "$set"
  for px in 16 32 64 128 256 512 1024; do
    if [ "$px" -le 32 ]; then svg="$here/$name-small.svg"; else svg="$here/$name.svg"; fi
    draw "$svg" "$work/$name-$px.png" "$px"
  done
  cp "$work/$name-16.png" "$set/icon_16x16.png"
  cp "$work/$name-32.png" "$set/icon_16x16@2x.png"
  cp "$work/$name-32.png" "$set/icon_32x32.png"
  cp "$work/$name-64.png" "$set/icon_32x32@2x.png"
  cp "$work/$name-128.png" "$set/icon_128x128.png"
  cp "$work/$name-256.png" "$set/icon_128x128@2x.png"
  cp "$work/$name-256.png" "$set/icon_256x256.png"
  cp "$work/$name-512.png" "$set/icon_256x256@2x.png"
  cp "$work/$name-512.png" "$set/icon_512x512.png"
  cp "$work/$name-1024.png" "$set/icon_512x512@2x.png"
  iconutil -c icns "$set" -o "$out/$name.icns"
  echo "made $out/$name.icns"
done

#!/bin/bash
# Build ScopeView-<version>-macos-universal.dmg from the two release binaries.
# Usage: packaging/macos/build-dmg.sh <version> <aarch64-binary> <x86_64-binary>
# Run from the repository root on macOS. Output goes to dist/.
set -euo pipefail

version=$1
arm=$2
intel=$3
app=build/ScopeView.app
dist=dist

rm -rf build "$dist"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$dist"

lipo -create "$arm" "$intel" -output "$app/Contents/MacOS/scopeview"
chmod +x "$app/Contents/MacOS/scopeview"
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"

# App icon: build an .icns from the PNGs in assets/icon.
iconset=build/ScopeView.iconset
mkdir -p "$iconset"
cp assets/icon/scopeview-16.png   "$iconset/icon_16x16.png"
cp assets/icon/scopeview-32.png   "$iconset/icon_16x16@2x.png"
cp assets/icon/scopeview-32.png   "$iconset/icon_32x32.png"
cp assets/icon/scopeview-64.png   "$iconset/icon_32x32@2x.png"
cp assets/icon/scopeview-128.png  "$iconset/icon_128x128.png"
cp assets/icon/scopeview-256.png  "$iconset/icon_128x128@2x.png"
cp assets/icon/scopeview-256.png  "$iconset/icon_256x256.png"
cp assets/icon/scopeview-512.png  "$iconset/icon_256x256@2x.png"
cp assets/icon/scopeview-512.png  "$iconset/icon_512x512.png"
iconutil -c icns "$iconset" -o "$app/Contents/Resources/ScopeView.icns"

# Ad-hoc signature: lets the app run on Apple silicon. It is not a Developer ID signature.
codesign --force --deep --sign - "$app"

stage=build/dmg
mkdir -p "$stage"
cp -R "$app" "$stage/"
cp LICENSE "$stage/LICENSE.txt"
ln -s /Applications "$stage/Applications"
hdiutil create -volname "ScopeView $version" -srcfolder "$stage" -ov -format UDZO \
    "$dist/ScopeView-$version-macos-universal.dmg"

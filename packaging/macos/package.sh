#!/usr/bin/env bash
# Packages the macOS launcher: Nexora.app (the binary, Info.plist, the icon), signed, then the
# two release assets, nexora-launcher-macos-arm64.zip (what the launcher updates itself from;
# older launchers find the binary inside it) and nexora-launcher-macos-arm64.dmg (the installer:
# the app and an Applications link over a background, to drag one onto the other).
#
#   packaging/macos/package.sh BINARY VERSION OUTDIR
#
# Needs macOS (ditto, codesign, hdiutil) and dmgbuild (pip install dmgbuild==1.6.5; $DMGBUILD
# names it if it isn't on PATH). Signing: ad hoc by default, which needs no Apple account. With
# MACOS_SIGN_IDENTITY set to a "Developer ID Application: …" identity (in $MACOS_SIGN_KEYCHAIN
# when set), the app and the .dmg are signed with the hardened runtime and a timestamp, and with
# MACOS_NOTARY_PROFILE too, both are notarized and stapled (keychain.sh unlock sets the two;
# see packaging/macos/README.md).
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
binary=$1
version=$2
out=$(mkdir -p "$3" && cd "$3" && pwd)
app=$out/Nexora.app

rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/nexora-launcher"
chmod 755 "$app/Contents/MacOS/nexora-launcher"
cp "$here/../icons/nexora.icns" "$app/Contents/Resources/nexora.icns"
sed "s/@VERSION@/$version/g" "$here/Info.plist" > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"
plutil -lint "$app/Contents/Info.plist"

identity=${MACOS_SIGN_IDENTITY:--}
keychain=()
[[ -n "${MACOS_SIGN_KEYCHAIN:-}" ]] && keychain=(--keychain "$MACOS_SIGN_KEYCHAIN")
if [[ "$identity" == "-" ]]; then
  codesign --force --sign - "$app"
else
  codesign --force --options runtime --timestamp ${keychain[@]+"${keychain[@]}"} --sign "$identity" "$app"
fi
codesign --verify --strict --verbose=2 "$app"
# Still starts under the hardened runtime.
"$app/Contents/MacOS/nexora-launcher" --version
notarize=$([[ "$identity" != "-" && -n "${MACOS_NOTARY_PROFILE:-}" ]] && echo yes || true)
# The app first, so the zip and the .dmg both carry it stapled.
[[ -z "$notarize" ]] || "$here/notarize.sh" "$app"

rm -f "$out/nexora-launcher-macos-arm64.zip" "$out/nexora-launcher-macos-arm64.dmg"
# No resource forks or extended attributes: they would land as ._ files inside the bundle.
(cd "$out" && ditto -c -k --norsrc --noextattr --noqtn --keepParent Nexora.app nexora-launcher-macos-arm64.zip)

cat > "$out/dmg.json" <<JSON
{
  "title": "Nexora",
  "icon-size": 128,
  "background": "$here/background.png",
  "window": { "position": { "x": 200, "y": 160 }, "size": { "width": 660, "height": 420 } },
  "format": "UDZO",
  "contents": [
    { "x": 180, "y": 200, "type": "file", "path": "$app" },
    { "x": 480, "y": 200, "type": "link", "path": "/Applications" }
  ]
}
JSON
"${DMGBUILD:-dmgbuild}" -s "$out/dmg.json" Nexora "$out/nexora-launcher-macos-arm64.dmg"
rm "$out/dmg.json"
dmg=$out/nexora-launcher-macos-arm64.dmg
if [[ "$identity" != "-" ]]; then
  codesign --force --timestamp ${keychain[@]+"${keychain[@]}"} --sign "$identity" "$dmg"
  codesign --verify --strict --verbose=2 "$dmg"
fi
hdiutil verify "$dmg" >/dev/null
if [[ -n "$notarize" ]]; then
  "$here/notarize.sh" "$dmg"
  # What Gatekeeper says of a download: both must be "source=Notarized Developer ID".
  spctl --assess --verbose=2 "$app"
  spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
fi
echo "Packaged $out/nexora-launcher-macos-arm64.zip and .dmg"

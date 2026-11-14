#!/usr/bin/env bash
# Build Sheaf.app, a macOS application bundle with Sheaf's icon, so the app can be found and started
# like any other: from Spotlight, Launchpad, Finder or the Dock.
#
#   tools/make-macos-app.sh             build <target>/release/Sheaf.app
#   tools/make-macos-app.sh --install   also copy it to ~/Applications (no password needed)
#
# Honours CARGO_TARGET_DIR. The icon comes from crates/app/assets/icon/Sheaf.icns
# (made by tools/make-icon.py). Opening a file by double-clicking it in Finder is not supported yet:
# start Sheaf and use File > Open, or drag the file onto the window.
set -euo pipefail

install=0
case "${1:-}" in
  "") ;;
  --install) install=1 ;;
  *) echo "usage: $0 [--install]" >&2; exit 2 ;;
esac

if [[ "$(uname)" != "Darwin" ]]; then
  echo "Sheaf.app is a macOS bundle; run this on a Mac." >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
target="${CARGO_TARGET_DIR:-$root/target}"
version="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"

cargo build --release -p sheaf-app -p sheaf-mcp --manifest-path "$root/Cargo.toml"

app="$target/release/Sheaf.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$target/release/sheaf-app" "$app/Contents/MacOS/sheaf-app"
cp "$target/release/sheaf-mcp" "$app/Contents/MacOS/sheaf-mcp"
cp "$root/crates/app/assets/icon/Sheaf.icns" "$app/Contents/Resources/Sheaf.icns"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Sheaf</string>
  <key>CFBundleDisplayName</key><string>Sheaf</string>
  <key>CFBundleIdentifier</key><string>com.electrochemistry-undergrads.sheaf</string>
  <key>CFBundleExecutable</key><string>sheaf-app</string>
  <key>CFBundleIconFile</key><string>Sheaf</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version}</string>
  <key>CFBundleVersion</key><string>${version}</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.education</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# A local ad-hoc signature, so macOS treats the bundle as one signed whole.
if command -v codesign >/dev/null; then
  codesign --force --deep --sign - "$app" >/dev/null 2>&1 || echo "codesign failed; the app still runs" >&2
fi

lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
dest="$app"
if (( install )); then
  mkdir -p "$HOME/Applications"
  dest="$HOME/Applications/Sheaf.app"
  rm -rf "$dest"
  cp -R "$app" "$dest"
fi
# Tell Launch Services (Spotlight, Launchpad, the Dock) about the bundle and its icon now.
[[ -x "$lsregister" ]] && "$lsregister" -f "$dest" || true
touch "$dest"

echo "Built $dest"
if (( install )); then
  echo "Find it with Spotlight (Cmd+Space, type Sheaf) or in Launchpad; drag it to the Dock to keep it there."
else
  echo "Run with --install to copy it to ~/Applications."
fi

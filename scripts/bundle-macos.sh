#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile=${SUGARSCAPE_PROFILE:-release}
cargo build --profile "$profile" --locked
output=$profile
if [ "$profile" = dev ]; then output=debug; fi
bundle="target/$output/Sugarscape.app"
mkdir -p "$bundle/Contents/MacOS"
cp "target/$output/sugarscape" "$bundle/Contents/MacOS/Sugarscape"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Sugarscape</string>
<key>CFBundleDisplayName</key><string>Sugarscape</string>
<key>CFBundleIdentifier</key><string>org.growing-artificial-societies.sugarscape</string>
<key>CFBundleVersion</key><string>0.1.0</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleExecutable</key><string>Sugarscape</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
printf 'Built %s\n' "$bundle"

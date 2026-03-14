#!/bin/bash
# Package the Aztibase Wallet extension into a release-ready zip.
# Builds WASM if needed, then creates aztibase-wallet-vX.Y.Z.zip
#
# Usage:
#   bash wallet-extension/package.sh            # package with version from manifest.json
#   bash wallet-extension/package.sh --upload    # package + attach to latest GitHub release

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

MANIFEST="$SCRIPT_DIR/manifest.json"
VERSION=$(node -e "console.log(JSON.parse(require('fs').readFileSync(process.argv[1],'utf8')).version)" "$(cygpath -w "$MANIFEST" 2>/dev/null || echo "$MANIFEST")")
ZIP_NAME="aztibase-wallet-v${VERSION}.zip"
OUT_PATH="$PROJECT_ROOT/dist/$ZIP_NAME"

echo "Packaging Aztibase Wallet v${VERSION}..."

# Build WASM if missing
if [ ! -f "$SCRIPT_DIR/wasm/aztibase_wasm_bg.wasm" ]; then
  echo "WASM not found — building..."
  bash "$SCRIPT_DIR/build.sh"
fi

# Ensure icons exist
if [ ! -f "$SCRIPT_DIR/icons/icon16.png" ]; then
  echo "Icons not found — generating..."
  bash "$SCRIPT_DIR/build.sh" --icons
fi

mkdir -p "$PROJECT_ROOT/dist"

# Create zip using PowerShell (Windows) or zip (Linux/Mac)
if command -v powershell.exe &>/dev/null; then
  # Remove old zip if exists
  rm -f "$OUT_PATH"
  powershell.exe -Command "
    \$src = '$(cygpath -w "$SCRIPT_DIR")'
    \$dst = '$(cygpath -w "$OUT_PATH")'
    \$tmp = Join-Path \$env:TEMP 'aztibase-wallet'
    if (Test-Path \$tmp) { Remove-Item \$tmp -Recurse -Force }
    Copy-Item \$src \$tmp -Recurse
    # Remove dev-only files
    Remove-Item \$tmp\build.sh -ErrorAction SilentlyContinue
    Remove-Item \$tmp\package.sh -ErrorAction SilentlyContinue
    Remove-Item \$tmp\generate-icons.html -ErrorAction SilentlyContinue
    Compress-Archive -Path \$tmp\* -DestinationPath \$dst -Force
    Remove-Item \$tmp -Recurse -Force
  "
elif command -v zip &>/dev/null; then
  cd "$SCRIPT_DIR"
  rm -f "$OUT_PATH"
  zip -r "$OUT_PATH" . \
    -x "./build.sh" \
    -x "./package.sh" \
    -x "./generate-icons.html"
else
  echo "Error: no zip tool available (need PowerShell or zip)"
  exit 1
fi

SIZE=$(stat -c%s "$OUT_PATH" 2>/dev/null || stat -f%z "$OUT_PATH" 2>/dev/null || echo "?")
echo ""
echo "=== Package ready ==="
echo "  $OUT_PATH ($SIZE bytes)"
echo ""
echo "Install instructions (include in release notes):"
echo "  1. Download $ZIP_NAME"
echo "  2. Extract the zip to a folder"
echo "  3. Open chrome://extensions"
echo "  4. Enable 'Developer mode' (top-right toggle)"
echo "  5. Click 'Load unpacked' and select the extracted folder"
echo "  6. Pin the Aztibase Wallet from the puzzle-piece icon"

if [ "$1" = "--upload" ]; then
  echo ""
  echo "Attaching to latest GitHub release..."
  LATEST_TAG=$(gh release list --limit 1 --json tagName -q '.[0].tagName' 2>/dev/null)
  if [ -z "$LATEST_TAG" ]; then
    echo "No release found. Create one first: gh release create v0.1.0"
    exit 1
  fi
  gh release upload "$LATEST_TAG" "$OUT_PATH" --clobber
  echo "Uploaded $ZIP_NAME to release $LATEST_TAG"
fi

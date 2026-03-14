#!/bin/bash
# Build and prepare the Aztibase Wallet Chrome extension.
# Prerequisites: wasm-pack (cargo install wasm-pack)
#
# Usage:
#   bash wallet-extension/build.sh          # build WASM + generate icons
#   bash wallet-extension/build.sh --icons  # regenerate icons only

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
WASM_CRATE="$PROJECT_ROOT/crates/aztibase-wasm"
OUT_DIR="$SCRIPT_DIR/wasm"
ICON_DIR="$SCRIPT_DIR/icons"

generate_icons() {
  echo "Generating extension icons..."
  node -e "
const fs = require('fs'), zlib = require('zlib'), path = require('path');
function icon(size) {
  const px = Buffer.alloc(size*size*4);
  const cx=size/2, r=size/2;
  for(let y=0;y<size;y++) for(let x=0;x<size;x++){
    const dx=x-cx+.5, dy=y-cx+.5, d=Math.sqrt(dx*dx+dy*dy), i=(y*size+x)*4;
    if(d<=r){
      const t=(x+y)/(2*size);
      px[i]=Math.round(0x58+(0x1f-0x58)*t);
      px[i+1]=Math.round(0xa6+(0x6f-0xa6)*t);
      px[i+2]=Math.round(0xff+(0xeb-0xff)*t);
      px[i+3]=255;
      const nx=(x-cx)/r, ny=(y-cx)/r;
      if((ny>-.15&&ny<.45&&Math.abs(nx)>.08&&Math.abs(nx)<.25-.15*(ny+.15))||
         (ny>=-.45&&ny<=-.15&&Math.abs(nx)<(ny+.45)*.55)||
         (ny>0&&ny<.12&&Math.abs(nx)<.22)){px[i]=px[i+1]=px[i+2]=px[i+3]=255;}
    } else px[i+3]=0;
  }
  const raw=Buffer.alloc(size*(1+size*4));
  for(let y=0;y<size;y++){raw[y*(1+size*4)]=0;px.copy(raw,y*(1+size*4)+1,y*size*4,(y+1)*size*4);}
  const c=zlib.deflateSync(raw);
  const ihdr=Buffer.alloc(13);ihdr.writeUInt32BE(size,0);ihdr.writeUInt32BE(size,4);ihdr[8]=8;ihdr[9]=6;
  function crc32(b){let c=0xFFFFFFFF;for(let i=0;i<b.length;i++){c^=b[i];for(let j=0;j<8;j++)c=(c>>>1)^(c&1?0xEDB88320:0);}return(c^0xFFFFFFFF)>>>0;}
  function chunk(t,d){const l=Buffer.alloc(4);l.writeUInt32BE(d.length);const tb=Buffer.from(t);const cr=Buffer.alloc(4);cr.writeUInt32BE(crc32(Buffer.concat([tb,d])));return Buffer.concat([l,tb,d,cr]);}
  return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',ihdr),chunk('IDAT',c),chunk('IEND',Buffer.alloc(0))]);
}
const dir='$ICON_DIR'.replace(/\\\\\\\\/g,'/');
[16,48,128].forEach(s=>fs.writeFileSync(path.join(dir,'icon'+s+'.png'),icon(s)));
console.log('  icons/icon16.png, icon48.png, icon128.png');
"
}

if [ "$1" = "--icons" ]; then
  generate_icons
  exit 0
fi

# Ensure icons exist
if [ ! -f "$ICON_DIR/icon16.png" ]; then
  generate_icons
fi

echo "Building aztibase-wasm for browser..."
cd "$WASM_CRATE"
wasm-pack build --target web --out-dir "$OUT_DIR" --out-name aztibase_wasm --no-opt

rm -f "$OUT_DIR/.gitignore" "$OUT_DIR/package.json" "$OUT_DIR/README.md"

echo ""
echo "=== Aztibase Wallet — ready to install ==="
echo ""
echo "WASM module: wallet-extension/wasm/"
echo "Icons:       wallet-extension/icons/"
echo ""
echo "To install in Chrome:"
echo "  1. Open chrome://extensions"
echo "  2. Enable 'Developer mode' (top-right toggle)"
echo "  3. Click 'Load unpacked'"
echo "  4. Select the wallet-extension/ folder"
echo "  5. Pin the extension from the puzzle-piece icon in the toolbar"
echo ""
echo "To update after code changes:"
echo "  - Re-run this script, then click the refresh icon on chrome://extensions"

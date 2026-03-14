// --- TOTP (RFC 6238) using Web Crypto API ---

const TOTP_DIGITS = 6;
const TOTP_PERIOD = 30;
const TOTP_WINDOW = 1; // ±1 period for clock skew

const BASE32_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

function base32Encode(buf) {
  let bits = "";
  for (const b of buf) bits += b.toString(2).padStart(8, "0");
  let out = "";
  for (let i = 0; i < bits.length; i += 5) {
    const chunk = bits.slice(i, i + 5).padEnd(5, "0");
    out += BASE32_CHARS[parseInt(chunk, 2)];
  }
  return out;
}

function base32Decode(str) {
  let bits = "";
  for (const c of str.toUpperCase()) {
    const idx = BASE32_CHARS.indexOf(c);
    if (idx === -1) continue;
    bits += idx.toString(2).padStart(5, "0");
  }
  const bytes = new Uint8Array(Math.floor(bits.length / 8));
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(bits.slice(i * 8, i * 8 + 8), 2);
  }
  return bytes;
}

function generateTotpSecret() {
  const buf = crypto.getRandomValues(new Uint8Array(20));
  return base32Encode(buf);
}

async function hmacSha1(key, message) {
  const cryptoKey = await crypto.subtle.importKey(
    "raw", key, { name: "HMAC", hash: "SHA-1" }, false, ["sign"]
  );
  const sig = await crypto.subtle.sign("HMAC", cryptoKey, message);
  return new Uint8Array(sig);
}

async function generateTotpCode(secret, timeStep) {
  const key = base32Decode(secret);
  const msg = new ArrayBuffer(8);
  const view = new DataView(msg);
  view.setUint32(0, Math.floor(timeStep / 0x100000000));
  view.setUint32(4, timeStep & 0xffffffff);

  const hash = await hmacSha1(key, new Uint8Array(msg));
  const offset = hash[hash.length - 1] & 0x0f;
  const code =
    ((hash[offset] & 0x7f) << 24) |
    ((hash[offset + 1] & 0xff) << 16) |
    ((hash[offset + 2] & 0xff) << 8) |
    (hash[offset + 3] & 0xff);

  return (code % 10 ** TOTP_DIGITS).toString().padStart(TOTP_DIGITS, "0");
}

async function verifyTotpCode(secret, inputCode) {
  const now = Math.floor(Date.now() / 1000 / TOTP_PERIOD);
  for (let i = -TOTP_WINDOW; i <= TOTP_WINDOW; i++) {
    const expected = await generateTotpCode(secret, now + i);
    if (expected === inputCode) return true;
  }
  return false;
}

function buildTotpUri(secret, account) {
  const issuer = "Aztibase";
  const label = `${issuer}:${account}`;
  return `otpauth://totp/${encodeURIComponent(label)}?secret=${secret}&issuer=${issuer}&digits=${TOTP_DIGITS}&period=${TOTP_PERIOD}`;
}

// --- WebAuthn / Passkey ---

const WEBAUTHN_RP = { name: "Aztibase Wallet", id: "localhost" };

async function isWebAuthnAvailable() {
  if (!window.PublicKeyCredential) return false;
  try {
    return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return false;
  }
}

async function registerWebAuthn(userId, userName) {
  const challenge = crypto.getRandomValues(new Uint8Array(32));

  const credential = await navigator.credentials.create({
    publicKey: {
      rp: WEBAUTHN_RP,
      user: {
        id: new TextEncoder().encode(userId),
        name: userName,
        displayName: userName,
      },
      challenge,
      pubKeyCredParams: [
        { type: "public-key", alg: -7 },   // ES256
        { type: "public-key", alg: -257 },  // RS256
      ],
      authenticatorSelection: {
        authenticatorAttachment: "platform",
        userVerification: "required",
        residentKey: "preferred",
      },
      timeout: 60000,
    },
  });

  return {
    credentialId: bufToB64(credential.rawId),
    publicKey: bufToB64(credential.response.getPublicKey?.() || new ArrayBuffer(0)),
    type: credential.type,
  };
}

async function verifyWebAuthn(credentialId) {
  const challenge = crypto.getRandomValues(new Uint8Array(32));

  const assertion = await navigator.credentials.get({
    publicKey: {
      challenge,
      allowCredentials: [{
        type: "public-key",
        id: b64ToBuf(credentialId),
      }],
      userVerification: "required",
      timeout: 60000,
    },
  });

  return !!assertion?.response?.authenticatorData;
}

function bufToB64(buf) {
  return btoa(String.fromCharCode(...new Uint8Array(buf)));
}

function b64ToBuf(b64) {
  const bin = atob(b64);
  const buf = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
  return buf.buffer;
}

// --- Minimal QR Code Generator (for TOTP setup) ---
// Generates a simple SVG QR code from a string.
// Implements QR Code Model 2 with byte encoding, error correction level L.

function generateQrSvg(text, moduleSize = 4) {
  const data = new TextEncoder().encode(text);
  const modules = encodeQr(data);
  const size = modules.length;
  const svgSize = size * moduleSize;

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${svgSize} ${svgSize}" width="${svgSize}" height="${svgSize}">`;
  svg += `<rect width="${svgSize}" height="${svgSize}" fill="#fff"/>`;

  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      if (modules[y][x]) {
        svg += `<rect x="${x * moduleSize}" y="${y * moduleSize}" width="${moduleSize}" height="${moduleSize}" fill="#000"/>`;
      }
    }
  }
  svg += "</svg>";
  return svg;
}

function encodeQr(data) {
  // Simplified QR encoder — version auto-selected (1-10), ECC level L, byte mode
  const version = selectVersion(data.length);
  const size = version * 4 + 17;
  const modules = Array.from({ length: size }, () => new Array(size).fill(false));
  const reserved = Array.from({ length: size }, () => new Array(size).fill(false));

  placeFinderPatterns(modules, reserved, size);
  placeAlignmentPatterns(modules, reserved, version, size);
  placeTimingPatterns(modules, reserved, size);
  reserveFormatArea(reserved, size);
  if (version >= 7) reserveVersionArea(reserved, size);

  const bits = encodeData(data, version);
  const eccBits = addErrorCorrection(bits, version);
  placeData(modules, reserved, eccBits, size);
  applyBestMask(modules, reserved, size);
  placeFormatInfo(modules, size, 0); // mask 0

  return modules;
}

function selectVersion(dataLen) {
  // Byte mode capacities for ECC level L (versions 1-10)
  const caps = [17, 32, 53, 78, 106, 134, 154, 192, 230, 271];
  for (let v = 0; v < caps.length; v++) {
    if (dataLen <= caps[v]) return v + 1;
  }
  return 10; // max we support
}

function placeFinderPatterns(m, r, s) {
  const positions = [[0, 0], [s - 7, 0], [0, s - 7]];
  for (const [row, col] of positions) {
    for (let dy = -1; dy <= 7; dy++) {
      for (let dx = -1; dx <= 7; dx++) {
        const y = row + dy, x = col + dx;
        if (y < 0 || y >= s || x < 0 || x >= s) continue;
        const inOuter = dy === 0 || dy === 6 || dx === 0 || dx === 6;
        const inInner = dy >= 2 && dy <= 4 && dx >= 2 && dx <= 4;
        const isSep = dy === -1 || dy === 7 || dx === -1 || dx === 7;
        m[y][x] = !isSep && (inOuter || inInner);
        r[y][x] = true;
      }
    }
  }
}

function placeAlignmentPatterns(m, r, ver, s) {
  if (ver < 2) return;
  const positions = getAlignmentPositions(ver);
  for (const row of positions) {
    for (const col of positions) {
      if (r[row][col]) continue;
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) {
          const y = row + dy, x = col + dx;
          m[y][x] = Math.abs(dy) === 2 || Math.abs(dx) === 2 || (dy === 0 && dx === 0);
          r[y][x] = true;
        }
      }
    }
  }
}

function getAlignmentPositions(ver) {
  if (ver === 1) return [];
  const last = ver * 4 + 10;
  if (ver <= 6) return [6, last];
  const count = Math.floor(ver / 7) + 2;
  const step = Math.ceil((last - 6) / (count - 1) / 2) * 2;
  const positions = [6];
  for (let i = last; positions.length < count; i -= step) positions.splice(1, 0, i);
  return positions;
}

function placeTimingPatterns(m, r, s) {
  for (let i = 8; i < s - 8; i++) {
    m[6][i] = i % 2 === 0;
    r[6][i] = true;
    m[i][6] = i % 2 === 0;
    r[i][6] = true;
  }
}

function reserveFormatArea(r, s) {
  for (let i = 0; i < 9; i++) {
    r[8][i] = true;
    r[i][8] = true;
  }
  for (let i = 0; i < 8; i++) {
    r[8][s - 1 - i] = true;
    r[s - 1 - i][8] = true;
  }
  r[s - 8][8] = true;
}

function reserveVersionArea(r, s) {
  for (let i = 0; i < 6; i++) {
    for (let j = 0; j < 3; j++) {
      r[i][s - 11 + j] = true;
      r[s - 11 + j][i] = true;
    }
  }
}

function encodeData(data, version) {
  const totalBits = getDataCapacity(version) * 8;
  const bits = [];

  // Mode indicator: byte mode = 0100
  pushBits(bits, 0b0100, 4);
  // Character count
  const ccBits = version <= 9 ? 8 : 16;
  pushBits(bits, data.length, ccBits);
  // Data
  for (const b of data) pushBits(bits, b, 8);
  // Terminator
  const remaining = totalBits - bits.length;
  pushBits(bits, 0, Math.min(4, remaining));
  // Byte-align
  while (bits.length % 8 !== 0) bits.push(0);
  // Pad bytes
  const padBytes = [0xec, 0x11];
  let pi = 0;
  while (bits.length < totalBits) {
    pushBits(bits, padBytes[pi % 2], 8);
    pi++;
  }

  return bits;
}

function getDataCapacity(ver) {
  // Total data codewords for ECC level L
  const caps = [19, 34, 55, 80, 108, 136, 156, 194, 232, 274];
  return caps[ver - 1] || caps[9];
}

function getEccInfo(ver) {
  // [total_codewords, ecc_per_block, num_blocks] for level L
  const info = [
    [26, 7, 1], [44, 10, 1], [70, 15, 1], [100, 20, 1], [134, 26, 1],
    [172, 18, 2], [196, 20, 2], [242, 24, 2], [292, 30, 2], [346, 18, 4],
  ];
  return info[ver - 1] || info[9];
}

function addErrorCorrection(bits, version) {
  const [totalCw, eccPerBlock, numBlocks] = getEccInfo(version);
  const dataCw = getDataCapacity(version);
  const dataBytes = [];
  for (let i = 0; i < bits.length; i += 8) {
    let byte = 0;
    for (let j = 0; j < 8 && i + j < bits.length; j++) {
      byte = (byte << 1) | bits[i + j];
    }
    dataBytes.push(byte);
  }

  const blockSize = Math.floor(dataCw / numBlocks);
  const extraBlocks = dataCw % numBlocks;
  const dataBlocks = [];
  const eccBlocks = [];
  let offset = 0;

  for (let b = 0; b < numBlocks; b++) {
    const bSize = blockSize + (b >= numBlocks - extraBlocks ? 1 : 0);
    const block = dataBytes.slice(offset, offset + bSize);
    offset += bSize;
    dataBlocks.push(block);
    eccBlocks.push(rsEncode(block, eccPerBlock));
  }

  // Interleave
  const result = [];
  const maxDataLen = Math.max(...dataBlocks.map(b => b.length));
  for (let i = 0; i < maxDataLen; i++) {
    for (const block of dataBlocks) {
      if (i < block.length) pushBits(result, block[i], 8);
    }
  }
  for (let i = 0; i < eccPerBlock; i++) {
    for (const block of eccBlocks) {
      if (i < block.length) pushBits(result, block[i], 8);
    }
  }

  return result;
}

// Reed-Solomon encoding over GF(256)
const GF_EXP = new Uint8Array(512);
const GF_LOG = new Uint8Array(256);
(() => {
  let x = 1;
  for (let i = 0; i < 255; i++) {
    GF_EXP[i] = x;
    GF_LOG[x] = i;
    x = (x << 1) ^ (x >= 128 ? 0x11d : 0);
  }
  for (let i = 255; i < 512; i++) GF_EXP[i] = GF_EXP[i - 255];
})();

function gfMul(a, b) {
  if (a === 0 || b === 0) return 0;
  return GF_EXP[GF_LOG[a] + GF_LOG[b]];
}

function rsEncode(data, eccCount) {
  // Build generator polynomial
  let gen = [1];
  for (let i = 0; i < eccCount; i++) {
    const newGen = new Array(gen.length + 1).fill(0);
    for (let j = 0; j < gen.length; j++) {
      newGen[j] ^= gen[j];
      newGen[j + 1] ^= gfMul(gen[j], GF_EXP[i]);
    }
    gen = newGen;
  }

  const msg = new Array(data.length + eccCount).fill(0);
  for (let i = 0; i < data.length; i++) msg[i] = data[i];

  for (let i = 0; i < data.length; i++) {
    const coef = msg[i];
    if (coef === 0) continue;
    for (let j = 1; j < gen.length; j++) {
      msg[i + j] ^= gfMul(gen[j], coef);
    }
  }

  return msg.slice(data.length);
}

function placeData(m, r, bits, s) {
  let bi = 0;
  for (let right = s - 1; right >= 1; right -= 2) {
    if (right === 6) right = 5;
    for (let vert = 0; vert < s; vert++) {
      for (let j = 0; j < 2; j++) {
        const x = right - j;
        const upward = ((right + 1) & 2) === 0;
        const y = upward ? s - 1 - vert : vert;
        if (r[y][x]) continue;
        m[y][x] = bi < bits.length ? bits[bi++] === 1 : false;
      }
    }
  }
}

function applyBestMask(m, r, s) {
  // Apply mask 0 (checkerboard) — simplest, works well enough
  for (let y = 0; y < s; y++) {
    for (let x = 0; x < s; x++) {
      if (!r[y][x] && (y + x) % 2 === 0) {
        m[y][x] = !m[y][x];
      }
    }
  }
}

function placeFormatInfo(m, s, mask) {
  const eccLevel = 1; // L
  const formatVal = (eccLevel << 3) | mask;
  let bits = formatVal;
  let rem = formatVal;
  for (let i = 0; i < 10; i++) {
    rem = (rem << 1) ^ ((rem >> 9) * 0b10100110111);
  }
  bits = ((formatVal << 10) | rem) ^ 0b101010000010010;

  const positions1 = [
    [8, 0], [8, 1], [8, 2], [8, 3], [8, 4], [8, 5], [8, 7], [8, 8],
    [7, 8], [5, 8], [4, 8], [3, 8], [2, 8], [1, 8], [0, 8],
  ];
  const positions2 = [];
  for (let i = 0; i < 7; i++) positions2.push([s - 1 - i, 8]);
  positions2.push([s - 8, 8]);
  for (let i = 0; i < 7; i++) positions2.push([8, s - 7 + i]);

  m[s - 8][8] = true; // dark module

  for (let i = 0; i < 15; i++) {
    const bit = ((bits >> (14 - i)) & 1) === 1;
    if (i < positions1.length) {
      const [y, x] = positions1[i];
      m[y][x] = bit;
    }
    if (i < positions2.length) {
      const [y, x] = positions2[i];
      m[y][x] = bit;
    }
  }
}

function pushBits(arr, val, count) {
  for (let i = count - 1; i >= 0; i--) {
    arr.push((val >> i) & 1);
  }
}

// --- 2FA Manager ---

class TwoFactorAuth {
  constructor() {
    this.totpSecret = null;
    this.webauthnCredentialId = null;
    this.enabled = false;
    this.method = null; // "totp", "webauthn", "both"
  }

  async load() {
    return new Promise((resolve) => {
      const get = (keys, cb) => {
        if (chrome?.storage?.local) {
          chrome.storage.local.get(keys, cb);
        } else {
          const result = {};
          for (const k of keys) {
            const v = localStorage.getItem(k);
            if (v) result[k] = JSON.parse(v);
          }
          cb(result);
        }
      };
      get(["twofa"], (result) => {
        if (result.twofa) {
          this.totpSecret = result.twofa.totpSecret || null;
          this.webauthnCredentialId = result.twofa.webauthnCredentialId || null;
          this.enabled = result.twofa.enabled || false;
          this.method = result.twofa.method || null;
        }
        resolve();
      });
    });
  }

  async save() {
    const data = {
      totpSecret: this.totpSecret,
      webauthnCredentialId: this.webauthnCredentialId,
      enabled: this.enabled,
      method: this.method,
    };
    return new Promise((resolve) => {
      if (chrome?.storage?.local) {
        chrome.storage.local.set({ twofa: data }, resolve);
      } else {
        localStorage.setItem("twofa", JSON.stringify(data));
        resolve();
      }
    });
  }

  async setupTotp(account) {
    this.totpSecret = generateTotpSecret();
    const uri = buildTotpUri(this.totpSecret, account);
    const qrSvg = generateQrSvg(uri, 3);
    return { secret: this.totpSecret, uri, qrSvg };
  }

  async confirmTotp(code) {
    if (!this.totpSecret) return false;
    const valid = await verifyTotpCode(this.totpSecret, code);
    if (valid) {
      this.enabled = true;
      this.method = this.webauthnCredentialId ? "both" : "totp";
      await this.save();
    }
    return valid;
  }

  async setupWebAuthn(address) {
    const available = await isWebAuthnAvailable();
    if (!available) throw new Error("WebAuthn not available on this device");

    const shortAddr = address.slice(0, 16);
    const cred = await registerWebAuthn(address, `Aztibase (${shortAddr}...)`);
    this.webauthnCredentialId = cred.credentialId;
    this.enabled = true;
    this.method = this.totpSecret ? "both" : "webauthn";
    await this.save();
    return cred;
  }

  async verify(totpCode) {
    if (!this.enabled) return true;

    if (this.method === "webauthn" || this.method === "both") {
      try {
        const ok = await verifyWebAuthn(this.webauthnCredentialId);
        if (ok) return true;
      } catch {
        if (this.method === "webauthn") return false;
        // Fall through to TOTP if "both"
      }
    }

    if (this.method === "totp" || this.method === "both") {
      if (!totpCode) return false;
      return verifyTotpCode(this.totpSecret, totpCode);
    }

    return false;
  }

  async disable() {
    this.enabled = false;
    this.method = null;
    this.totpSecret = null;
    this.webauthnCredentialId = null;
    await this.save();
  }
}

export {
  TwoFactorAuth,
  generateTotpSecret,
  verifyTotpCode,
  buildTotpUri,
  generateQrSvg,
  isWebAuthnAvailable,
};

#!/usr/bin/env node
// Build-time generator for the default Open Graph card (§11.3).
// Pure Node — no deps. Writes a 1200×630 PNG to website/public/og-default.png
// so Vite copies it into dist/ during the website build.
//
// Design: dark slate background (#0f172a) with a 12px accent bar on the
// left (#3b82f6) and a single rounded-rectangle "card" silhouette in
// the centre. No text — `satori` would let us render the project name,
// but the dependency cost is large for a tile that mostly just needs to
// not render as a blank box on Slack/LinkedIn.
//
// The image is regenerated on every build so the file in `public/` and
// the script that produced it can never drift.

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT = resolve(__dirname, "../public/og-default.png");

const W = 1200;
const H = 630;

// Palette — kept in sync with index.html's theme-color and the website's
// dark mode. RGB triples.
const BG = [0x0f, 0x17, 0x2a];
const ACCENT = [0x3b, 0x82, 0xf6];
const CARD = [0x1e, 0x29, 0x3b];

// Layout
const ACCENT_WIDTH = 12;
const CARD_X = 200;
const CARD_Y = 195;
const CARD_W = 800;
const CARD_H = 240;
const CARD_RADIUS = 24;

function inRoundedRect(x, y, x0, y0, w, h, r) {
  if (x < x0 || x >= x0 + w || y < y0 || y >= y0 + h) return false;
  // Squared distance to the nearest corner centre, where applicable.
  const corners = [
    [x0 + r, y0 + r],
    [x0 + w - r, y0 + r],
    [x0 + r, y0 + h - r],
    [x0 + w - r, y0 + h - r],
  ];
  const inCornerBox =
    (x < x0 + r || x >= x0 + w - r) && (y < y0 + r || y >= y0 + h - r);
  if (!inCornerBox) return true;
  for (const [cx, cy] of corners) {
    const dx = x - cx;
    const dy = y - cy;
    if (
      Math.abs(x - cx) <= r &&
      Math.abs(y - cy) <= r &&
      dx * dx + dy * dy <= r * r
    ) {
      return true;
    }
  }
  return false;
}

// Build raw RGB scanlines with a filter-None byte per row (PNG truecolor).
function buildRaw() {
  const rowSize = 1 + W * 3;
  const raw = Buffer.alloc(H * rowSize);
  for (let y = 0; y < H; y++) {
    const rowStart = y * rowSize;
    raw[rowStart] = 0; // filter None
    for (let x = 0; x < W; x++) {
      let c = BG;
      if (x < ACCENT_WIDTH) {
        c = ACCENT;
      } else if (inRoundedRect(x, y, CARD_X, CARD_Y, CARD_W, CARD_H, CARD_RADIUS)) {
        c = CARD;
      }
      const px = rowStart + 1 + x * 3;
      raw[px] = c[0];
      raw[px + 1] = c[1];
      raw[px + 2] = c[2];
    }
  }
  return raw;
}

// Standard CRC32 over the chunk type + data (PNG spec §5.2).
function crc32(buf) {
  if (!crc32.table) {
    const t = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c >>> 0;
    }
    crc32.table = t;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    crc = (crc32.table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8)) >>> 0;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

function buildPng() {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(W, 0);
  ihdr.writeUInt32BE(H, 4);
  ihdr.writeUInt8(8, 8);  // bit depth
  ihdr.writeUInt8(2, 9);  // color type 2 = truecolor RGB
  ihdr.writeUInt8(0, 10); // compression
  ihdr.writeUInt8(0, 11); // filter
  ihdr.writeUInt8(0, 12); // interlace

  const idat = deflateSync(buildRaw(), { level: 9 });
  const iend = Buffer.alloc(0);

  return Buffer.concat([
    sig,
    chunk("IHDR", ihdr),
    chunk("IDAT", idat),
    chunk("IEND", iend),
  ]);
}

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, buildPng());
console.log(`Wrote ${OUT} (${W}×${H})`);

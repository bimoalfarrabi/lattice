// Golden tile addressing: transforms, mirror inversion and coverage.
import fs from 'node:fs';
import path from 'node:path';
import { tileTransform, tilesCovering } from './engine.mjs';

const out = path.join(import.meta.dirname, '..', '..', 'fixtures', 'tiling.json');

const tiles = [];
for (let y = -2; y <= 2; y++) {
  for (let x = -2; x <= 2; x++) {
    // Local point 20,30 mirrored back out, then back in, must round trip.
    const forward = tileTransform({ x, y });
    const sx = x % 2 !== 0 ? -1 : 1;
    const sy = y % 2 !== 0 ? -1 : 1;
    const originX = (x + (x % 2 !== 0 ? 1 : 0)) * 240;
    const originY = (y + (y % 2 !== 0 ? 1 : 0)) * 240;
    tiles.push({
      x,
      y,
      transform: forward,
      world: { x: originX + sx * 20, y: originY + sy * 30 },
    });
  }
}

const boxes = [
  { x: 0, y: 0, width: 240, height: 240 },
  { x: -100, y: -30, width: 200, height: 90 },
  { x: 200, y: -1, width: 81, height: 500 },
  { x: 479, y: 239, width: 1, height: 1 },
];

fs.writeFileSync(
  out,
  `${JSON.stringify({ tiles, boxes: boxes.map((b) => ({ box: b, tiles: tilesCovering(b) })) })}\n`,
);
console.log(`wrote ${tiles.length} tiles, ${boxes.length} boxes`);

// Emits golden geometry for every grid and both layouts.
// The Rust port must reproduce these cell path strings exactly.
import fs from 'node:fs';
import path from 'node:path';
import { GRID_TABLE, guidesFor, cellsFor, scaleYFor } from './engine.mjs';

const out = path.join(import.meta.dirname, '..', '..', 'fixtures', 'parity.json');

const cases = [];
for (const { id, name } of GRID_TABLE) {
  for (const original of [false, true]) {
    const guides = guidesFor(id, original);
    if (!guides) continue;
    const cells = cellsFor({ id, guides });
    cases.push({ id, name, original, scaleY: scaleYFor(id), cells });
  }
}

fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, `${JSON.stringify(cases)}\n`);

const total = cases.reduce((n, c) => n + c.cells.length, 0);
console.log(`wrote ${cases.length} cases, ${total} cells, ${fs.statSync(out).size} bytes`);

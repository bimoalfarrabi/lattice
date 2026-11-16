# Lattice documentation

Lattice is a desktop grid logo maker written in Rust with GPUI. It is an
independent reimplementation of the CreateX studio at `logofx.app/createx`,
built after reverse engineering the shipped web bundle.

Start with the [architecture](architecture.md) if you are working on the code.
The [geometry](geometry.md) page explains the engine that everything else
depends on.

| Document | Purpose |
|---|---|
| [architecture.md](architecture.md) | Crate layout, data flow, invariants, and the boundaries between layers |
| [geometry.md](geometry.md) | Guide primitives, the planar arrangement, tiling addressing, and why output must stay exact |
| [ui-spec.md](ui-spec.md) | Measured tokens, layout, states, and every deliberate deviation from the reference |
| [reverse-engineering.md](reverse-engineering.md) | What was inspected, how, what was learned, and what remains unknown |
| [decisions.md](decisions.md) | Scope and architecture decisions with their reasons, including open ones |
| [testing.md](testing.md) | What the suite covers, how to run it, and what still needs a human |

## Short version

- `crates/lattice-core` is pure geometry, documents, and export. No windowing.
- `crates/lattice-app` is the GPUI studio: theme, icons, canvas, shell.
- `tools/parity` regenerates golden fixtures from the reference bundle.
- All 31 lattices are unlocked, everything runs locally, and drawings
  autosave to `~/.local/share/lattice/drawing.json`.

## Conventions

- No em dashes in text, comments, or UI copy.
- Rust 2024, `cargo fmt` defaults, doc comments on public items.
- Interior mutability is limited to the canvas bounds cell.
- Geometry output is append only: a change that alters any cell path is a
  regression unless the parity fixtures are regenerated on purpose.

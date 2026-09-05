# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A terminal user interface (TUI), built with `ratatui`, that visualizes every port on the machine (0–65535) as a grid/heatmap — showing which ports are listening or established, and their protocol (TCP/UDP) — using color/position only, without alphanumeric characters.

## Commands

```bash
make run      # cargo fmt + cargo run (debug)
make build    # cargo fmt + cargo build --release
cargo clippy  # style/performance lints (run before considering work done)
cargo fmt     # formatter (also run via make run/build)
```

There is no test suite yet (`cargo test` has nothing to run).

To inspect real port/socket state for reference while implementing `ports/`:

```bash
sudo ss -tunlp   # -t tcp, -u udp, -n numeric, -l listening only, -p show process
```

## Architecture

The project is tiny (~220 lines) but wires modules via explicit `#[path = "..."]` attributes instead of `mod.rs`/directory-name files — when adding a new module under `src/ui/` or `src/ports/`, follow the existing sibling file's `#[path = ...]` declaration rather than assuming Rust's default module resolution.

Render/data flow, top to bottom:

- **`src/main.rs`** — entry point. Enters raw mode + alternate screen, then runs the main loop: `terminal.draw(layout::tui)`, poll for a `q` keypress (50ms poll, 500ms frame sleep), repeat until quit. This is where the polling cadence and terminal setup/teardown live.
- **`src/ui/layout.rs`** (`layout::tui`) — builds the frame layout: an outer vertical split (header row + main body), then a horizontal split of the body into `[left margin, matrix (75%), right margin]`. It pulls the row data from `matrix_populator::ports_matrix()` and hands it to `table::generate_table()` for rendering. `WIDTH` (currently `100 + 2`, i.e. columns plus 2 margin columns) is the shared const generic that both the matrix populator and table module size their arrays by — the +2 margins exist purely to center the matrix visually and carry no port data.
- **`src/ui/matrix/populator.rs`** (`ports_matrix<WIDTH>`) — builds the grid rows as `ratatui` `Row`/`Cell` arrays. Currently a placeholder: each non-margin cell gets a random RGB background color via `rand`. This is the seam where real per-port state (listening/established, TCP/UDP) will replace the random color.
- **`src/ui/table.rs`** (`generate_table<WIDTH>`) — turns the row arrays into a `ratatui::widgets::Table`, with per-column `Constraint::Length(1)` (so each cell is a single-width "pixel") except the two margin columns, which use `Constraint::Fill(1)` to center the grid.
- **`src/ports/iterator.rs`** — not yet implemented; currently just source links for prior art (RustScan's socket iterator, Go `netstat`/`somo` implementations) to draw from when writing the real port-scanning/socket-enumeration logic that will feed `matrix_populator`.

Given the target of 65536 ports, note that `WIDTH` in `layout.rs`/`table.rs` currently governs columns for one dimension only — reconciling the full 256×256 grid against terminal size is an open `TODO.md` item, not yet solved in the code.

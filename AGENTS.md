# Repository Guidelines

## Project Structure & Module Organization

This Rust repository generates SKK dictionaries from Japanese Wiktionary dumps and the Geospatial Information Authority of Japan Gazetteer of Japan.

- `src/` — shared XML parsing, dictionary models, and conversion logic.
  - `src/jion/` — historical-kana mapping, segmentation, and dictionary generation, including Wiktionary template parsing in `wiktionary.rs`.
  - `src/gsi/` — Gazetteer of Japan PDF-text parsing and dictionary generation.
- `src/bin/` — command-line dictionary generators such as `wiktionary_jisyo`, `jion`, and `gsi`.
- `tests/` — integration tests and self-contained fixtures.
- `data/` — locally downloaded Wiktionary dumps; not committed.
- `make.sh` — downloads dumps, extracts relevant pages, and generates dictionaries.
- `header.txt` — CC BY-SA 4.0 header prepended to Wiktionary-derived dictionaries.
- `gsi-header.txt` — CC BY 4.0-compatible header prepended to the GSI-derived dictionary.
- `PLAN.md` — implementation plan for the Wiktionary-derived dictionary.

## Build, Test, and Development Commands

```sh
cargo test                     # Run tests
cargo build --release          # Build optimized binaries
cargo fmt --all -- --check     # Check formatting
cargo clippy --all-targets -- -D warnings
./make.sh                      # Download dumps and generate dictionaries
```

The full workflow may require `wget`, Docker/MySQL, `pdftotext`, Cargo, and `skkdic-sort`. Do not delete local verification data.

## Coding Style & Naming Conventions

- Use standard `rustfmt` formatting.
- Use `snake_case` for functions and variables and `CamelCase` for types.
- Keep template parsing in `src/jion/wiktionary.rs`, conversion rules in `src/jion/rules.rs`, GSI PDF parsing in `src/gsi/`, and CLI wrappers in `src/bin/`.
- Prefer small testable functions and explicit handling for incomplete data.
- Keep shell scripts POSIX-compatible.

## Testing Guidelines

- Add unit tests near the relevant module and integration tests under `tests/`.
- Use only repository-created fixtures in tests; do not test with GPL-derived dictionary data.
- Test dictionary format carefully, including multiple readings, okuri entries, and annotations.
- Before submitting, run `cargo test`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets -- -D warnings`.

## Commit & Pull Request Guidelines

Recent commits use short imperative subjects such as `update README.md`. Continue that style and keep each commit focused.

Pull requests should:

- Explain the implementation and tests, with representative dictionary samples.
- Link related issues when applicable.
- Note changes to generated data, dependencies, or build scripts.

## Data, Licensing, and Safety

- Wiktionary-derived dictionaries must comply with CC BY-SA 4.0.
- The GSI dictionary must keep `gsi-header.txt` and must not be combined into a single CC BY-SA file.
- Do not copy vocabulary lists from GPL dictionaries such as `SKK-JISYO.L` into the repository or exception files.
- Preserve `header.txt` at the start of generated dictionaries.
- Do not delete local dump or verification files.

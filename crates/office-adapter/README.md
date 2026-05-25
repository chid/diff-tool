# office-adapter

Office document format detection, extraction, and structured comparison.

Supports `.docx` (Word), `.pptx` (PowerPoint), and `.xlsx` (Excel) — no
Microsoft Office installation required.

## Status

-   **Phase 1 (current):** Format detection, safe zip reading, module skeleton, data models.
-   **Phase 2 (next):** XML extraction and structured comparison engine.
-   **Phase 3 (future):** CLI integration (`dt office`) and desktop UI.

## Sandbox Setup

This crate is part of the diff-tool Rust workspace. Everything works from the
repository root.

### Prerequisites

-   Rust 1.95+
-   The workspace dependencies are already installed via the root `Cargo.toml`.

### Quick Start

```bash
# From the repository root:

# Check that the crate compiles
cargo check -p office-adapter

# Run all office-adapter tests
cargo test -p office-adapter

# Run only the format detection tests
cargo test -p office-adapter format

# Run only the zip safety tests
cargo test -p office-adapter unpack

# Run only the data model serialization tests
cargo test -p office-adapter models

# Run with verbose output
cargo test -p office-adapter -- --nocapture

# Check test coverage summary
cargo test -p office-adapter -- --list
```

### Running A Single Test

```bash
cargo test -p office-adapter test_detect_docx
cargo test -p office-adapter test_enforces_max_part_count
cargo test -p office-adapter test_office_diff_result_serialization
```

### Running From The CLI Crate (Once Phase 3 Is Wired)

```bash
# Compare two Office files
cargo run -p diff-cli -- office left.docx right.docx --json
```

### Using In Another Crate

Add to your `Cargo.toml`:

```toml
[dependencies]
office-adapter = { path = "crates/office-adapter" }
```

```rust
use office_adapter::{compare_office_files, models::OfficeDiffOptions};

let result = compare_office_files(
    "left.docx".into(),
    "right.docx".into(),
    OfficeDiffOptions::default(),
)?;

println!("{:?}", result.summary);
```

## Development

### Module Structure

```
src/
  lib.rs          Public API, OfficeError
  format.rs       Format detection (extension + zip signature)
  unpack.rs       Safe zip reading with size/part limits
  models.rs       Shared types, enums, document models
  xml_util.rs     XML parsing helpers, namespace constants
  word/
    mod.rs        Word document extraction and comparison
  powerpoint/
    mod.rs        PowerPoint extraction and comparison
  excel/
    mod.rs        Excel workbook extraction and comparison
```

### Adding Tests

Tests are inline in each module using `#[cfg(test)] mod tests`. Use
`tempfile` to create temporary files for integration-style tests.

### Safety Rules

-   Zip reading always goes through `unpack::read_zip_safe()` — never use raw `zip` APIs.
-   XML parsing uses `xml_util::create_reader()` which disables DTD expansion.
-   Image data is hashed, not loaded into memory in full.

### Conventions

-   Follow existing workspace conventions: `edition = "2024"`, use workspace
    dependencies where available.
-   Public types must be `Serialize + Deserialize`.
-   Deterministic output — same inputs always produce identical JSON.
-   `#[allow(dead_code)]` is acceptable on Phase 2 stubs that are wired but
    not yet called.

## Next Steps (Phase 2)

1.  Implement `word::extract` — parse `word/document.xml` into `WordDocument`.
2.  Implement `word::compare` — compare two `WordDocument`s and produce `Vec<OfficeSectionDiff>`.
3.  Repeat for PowerPoint and Excel.
4.  Wire extraction and comparison into `lib::compare_office_files()`.
5.  Add fixture-driven tests with hand-authored `.docx`, `.pptx`, `.xlsx` files.

# projzst

A command-line tool for packing and unpacking `.pjz` files with MessagePack metadata and zstd compression.

## Features

- Pack directories into compressed `.pjz` archives
- Store structured metadata in MessagePack format
- Support for extra custom metadata via JSON files
- Configurable zstd compression levels (1-22)
- Extract archives with automatic metadata export

## File Format

```
┌─────────────────────────────────────────────────────────┐
│ 4 bytes         │ N bytes              │ M bytes        │
│ (big-endian)    │                      │                │
├─────────────────┼──────────────────────┼────────────────┤
│ Metadata Length │ MessagePack Metadata │ tar.zst Data   │
└─────────────────────────────────────────────────────────┘
```

## Installation

### From Source

```bash
git clone https://github.com/0x7f-del/projzst.git
cd projzst
cargo build --release

# Binary located at target/release/projzst
```

### Run Directly

```bash
cargo run -- <command> [options]
```

## Usage

### Pack a Directory

```bash
projzst pack ./my-project \
    --name my-project \
    --auth "Your Name" \
    --fmt package-format \
    --ed 2024 \
    --ver 1.0.0 \
    --desc "Project description" \
    output.pjz
```

With extra metadata:
```bash
projzst pack ./my-project \
    --name my-project \
    --auth "Your Name" \
    --fmt package-format \
    --ed 2024 \
    --ver 1.0.0 \
    --desc "Project description" \
    --extra extra-metadata.json \
    --level 5 \
    output.pjz
```

### Unpack an Archive

```bash
projzst unpack input.pjz ./output-directory
```

This extracts the archive contents to `./output-directory` and creates `metadata.json` in the parent directory.

### Extract Metadata Only

```bash
projzst info input.pjz metadata.json
```

## Metadata Structure

```json
{
  "name": "my-project",
  "auth": "Your Name",
  "fmt": "package-format",
  "ed": "2024",
  "ver": "1.0.0",
  "desc": "A simple project",
  "extra": {
    "custom_field": "value",
    "nested": { "key": 123 }
  }
}
```

## Library Usage

```rust
use projzst::{pack, unpack, info, read_metadata, Metadata};

// Create metadata
let metadata = Metadata::new(
    "my-project",
    "Author",
    "format",
    "edition",
    "1.0.0",
    "Description"
);

// Pack directory
pack("./source", "output.pjz", metadata, None::<&str>, 3)?;

// Read metadata only
let meta = read_metadata("output.pjz")?;

// Unpack archive
unpack("output.pjz", "./extracted")?;

// Export metadata to JSON
info("output.pjz", "metadata.json")?;
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | Command-line argument parsing |
| `thiserror` | Error type definitions |
| `serde` | Serialization framework |
| `serde_json` | JSON processing |
| `rmp-serde` | MessagePack serialization |
| `zstd` | Zstandard compression |
| `tar` | Tar archive handling |

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_pack_and_unpack_full_cycle
```

## License

MIT
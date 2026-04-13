# dart_dec — Dart AOT Headless Decompiler

The fastest, most modular, and pipeline-ready reverse engineering tool for Flutter/Dart AOT-compiled applications.

## Features

- **Headless-first**: Zero GUI dependency, perfect for CI/CD pipelines
- **Multi-architecture**: ARM64, ARM32, x86_64
- **Version-resilient**: JSON profiles for every Dart VM version
- **Library mode**: Embeddable as .so/.dylib via C FFI
- **Pattern recovery**: async/await, Streams, Records, Sealed Classes
- **Deobfuscation**: Handle `--obfuscate`, symbol recovery, heuristic naming
- **Security scanning**: Secrets, weak crypto, SARIF output
- **Blazing fast**: rayon + bumpalo = 100MB binary in <5 seconds

## Quick Start

```bash
# Install from source
cargo install --path crates/dart_dec_cli

# Show binary info
dart_dec info --so libapp.so

# Full decompilation to JSON
dart_dec --so libapp.so --format json -o output.json

# Dump all classes as CSV
dart_dec --so libapp.so --dump classes --format csv

# Security scan with SARIF output
dart_dec --so libapp.so --scan --format sarif -o report.sarif

# Decompile specific method
dart_dec --so libapp.so --method "Auth.login" --format json

# Batch processing
find ./samples/ -name "libapp.so" | parallel dart_dec --so {} --format json -o {}.json
```

## Output Formats

| Format | Description | Use case |
|--------|-------------|----------|
| `json` | Full structured JSON | Python/JS scripts |
| `sqlite` | SQLite database | SQL analytics |
| `dart` | Generated .dart files | Code review |
| `sarif` | SARIF v2.1 | GitHub CodeQL, Semgrep |
| `dot` | Graphviz DOT | CFG visualization |
| `csv` | Simple table | Excel/Sheets |
| `jsonl` | JSON Lines (streaming) | Large binaries |

## Architecture

```
dart_dec/
├── crates/
│   ├── dart_dec_core/       # ELF/Mach-O/PE parsing
│   ├── dart_dec_snapshot/   # AOT Snapshot parsing
│   ├── dart_dec_profiles/   # Version profiles
│   ├── dart_dec_disasm/     # Multi-arch disassembler
│   ├── dart_dec_lifter/     # Assembly → IR
│   ├── dart_dec_graph/      # CFG, SSA, AST
│   ├── dart_dec_patterns/   # Dart pattern recovery
│   ├── dart_dec_deobf/      # Deobfuscation
│   ├── dart_dec_output/     # Output formatters
│   ├── dart_dec_scan/       # Security scanners
│   ├── dart_dec_cli/        # CLI entry point
│   └── dart_dec_lib/        # C FFI library
```

## Library Usage (Python)

```python
import ctypes
lib = ctypes.CDLL("libdart_dec.so")

# Open binary
ctx = lib.dart_dec_open(b"libapp.so")

# Get classes
classes_json = lib.dart_dec_get_classes_json(ctx)

# Clean up
lib.dart_dec_close(ctx)
```

## Building

```bash
# Debug build
cargo build --workspace

# Release build
cargo build --release

# Run tests
cargo test --workspace

# Run benchmarks
cargo bench
```

## Docker

```bash
docker build -t dart_dec .
docker run --rm -v ./samples:/data dart_dec --so /data/libapp.so --format json
```

## Supported Dart Versions

- Dart 2.19.x
- Dart 3.0.x
- Dart 3.2.x
- Dart 3.5.x
- More via `dart_dec profile-gen`

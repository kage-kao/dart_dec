# Ghidra Plugin for dart_dec — Dart AOT Decompiler

## Installation

1. Build libdart_dec:
   ```bash
   cd dart_dec
   cargo build --release -p dart_dec_lib
   ```

2. Copy the library to a known location:
   ```bash
   # Linux
   cp target/release/libdart_dec.so /usr/local/lib/
   # macOS
   cp target/release/libdart_dec.dylib /usr/local/lib/
   ```

3. Copy `DartDecAnalyze.java` to your Ghidra scripts directory:
   ```bash
   cp DartDecAnalyze.java ~/ghidra_scripts/
   ```

4. (Optional) Set environment variable:
   ```bash
   export DART_DEC_LIB=/usr/local/lib/libdart_dec.so
   ```

## Usage

1. Open a Dart AOT binary (libapp.so) in Ghidra
2. Open Script Manager (Window → Script Manager)
3. Search for "DartDecAnalyze" under the "Dart" category
4. Run the script

The script will:
- Parse the binary with dart_dec
- Annotate classes with their Dart names
- Display found URLs and potential secrets
- Add comments to recognized functions

## Requirements

- Ghidra 10.0+
- JNA library (included in Ghidra)
- libdart_dec.so/.dylib built from this project

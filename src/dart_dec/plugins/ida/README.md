# IDA Pro Plugin for dart_dec — Dart AOT Decompiler

## Installation

1. Build libdart_dec:
   ```bash
   cargo build --release -p dart_dec_lib
   ```

2. Set the library path:
   ```bash
   export DART_DEC_LIB=/path/to/libdart_dec.so
   ```

3. Copy `dart_dec_ida.py` to IDA's plugins directory:
   ```bash
   # Linux
   cp dart_dec_ida.py ~/.idapro/plugins/
   # macOS
   cp dart_dec_ida.py ~/Library/Application\ Support/IDA\ Pro/plugins/
   ```

## Usage

### From IDA Pro GUI
- Edit → Plugins → dart_dec Analyzer
- Or press Ctrl+Shift+D

### From IDAPython Console
```python
import dart_dec_ida
dart_dec_ida.run()
```

### Standalone (without IDA)
```bash
python dart_dec_ida.py libapp.so
```

## Features
- Recovers and annotates Dart class names
- Identifies URLs and potential secrets in strings
- Adds comments to recognized functions
- Works with ARM64, ARM32, and x86_64 binaries

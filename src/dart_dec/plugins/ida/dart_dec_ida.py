"""
IDA Pro Plugin for dart_dec — Dart AOT Decompiler

Integrates dart_dec analysis into IDA Pro via the C FFI library.
Annotates functions, classes, and strings recovered from Dart AOT snapshots.

Installation:
    1. Build libdart_dec: cargo build --release -p dart_dec_lib
    2. Copy this script to IDA's plugins/ directory
    3. Set DART_DEC_LIB env var or edit LIB_PATH below

Usage:
    Edit → Plugins → dart_dec Analyzer
    or from IDAPython console: import dart_dec_ida; dart_dec_ida.run()
"""

import ctypes
import json
import os
import sys

try:
    import idaapi
    import idautils
    import idc
    IN_IDA = True
except ImportError:
    IN_IDA = False
    print("[dart_dec] Not running inside IDA Pro, standalone mode")

# Path to libdart_dec shared library
LIB_PATH = os.environ.get("DART_DEC_LIB", None)

# Try common locations
if LIB_PATH is None:
    candidates = [
        "./libdart_dec.so",
        "./libdart_dec.dylib",
        "/usr/local/lib/libdart_dec.so",
        "/usr/local/lib/libdart_dec.dylib",
        os.path.expanduser("~/.local/lib/libdart_dec.so"),
    ]
    for c in candidates:
        if os.path.exists(c):
            LIB_PATH = c
            break


class DartDecFFI:
    """Wrapper around libdart_dec C FFI"""

    def __init__(self, lib_path):
        self.lib = ctypes.CDLL(lib_path)

        # dart_dec_open(path: *const c_char) -> *mut Context
        self.lib.dart_dec_open.argtypes = [ctypes.c_char_p]
        self.lib.dart_dec_open.restype = ctypes.c_void_p

        # dart_dec_get_classes_json(ctx) -> *mut c_char
        self.lib.dart_dec_get_classes_json.argtypes = [ctypes.c_void_p]
        self.lib.dart_dec_get_classes_json.restype = ctypes.c_void_p

        # dart_dec_get_strings_json(ctx) -> *mut c_char
        self.lib.dart_dec_get_strings_json.argtypes = [ctypes.c_void_p]
        self.lib.dart_dec_get_strings_json.restype = ctypes.c_void_p

        # dart_dec_decompile_function(ctx, class, func) -> *mut c_char
        self.lib.dart_dec_decompile_function.argtypes = [
            ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p
        ]
        self.lib.dart_dec_decompile_function.restype = ctypes.c_void_p

        # dart_dec_free_string(ptr)
        self.lib.dart_dec_free_string.argtypes = [ctypes.c_void_p]
        self.lib.dart_dec_free_string.restype = None

        # dart_dec_close(ctx)
        self.lib.dart_dec_close.argtypes = [ctypes.c_void_p]
        self.lib.dart_dec_close.restype = None

        self.ctx = None

    def open(self, path):
        """Open a Dart AOT binary"""
        self.ctx = self.lib.dart_dec_open(path.encode("utf-8"))
        if not self.ctx:
            raise RuntimeError(f"Failed to open {path}")
        return self

    def get_classes(self):
        """Get all classes as list of dicts"""
        ptr = self.lib.dart_dec_get_classes_json(self.ctx)
        if not ptr:
            return []
        try:
            json_str = ctypes.string_at(ptr).decode("utf-8")
            return json.loads(json_str)
        finally:
            self.lib.dart_dec_free_string(ptr)

    def get_strings(self):
        """Get all strings"""
        ptr = self.lib.dart_dec_get_strings_json(self.ctx)
        if not ptr:
            return []
        try:
            json_str = ctypes.string_at(ptr).decode("utf-8")
            return json.loads(json_str)
        finally:
            self.lib.dart_dec_free_string(ptr)

    def decompile(self, class_name, func_name):
        """Decompile a specific function"""
        ptr = self.lib.dart_dec_decompile_function(
            self.ctx,
            class_name.encode("utf-8"),
            func_name.encode("utf-8"),
        )
        if not ptr:
            return None
        try:
            return ctypes.string_at(ptr).decode("utf-8")
        finally:
            self.lib.dart_dec_free_string(ptr)

    def close(self):
        """Close and free resources"""
        if self.ctx:
            self.lib.dart_dec_close(self.ctx)
            self.ctx = None

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


def run(binary_path=None):
    """Main entry point for IDA or standalone usage"""
    if LIB_PATH is None:
        print("[dart_dec] ERROR: libdart_dec not found!")
        print("[dart_dec] Set DART_DEC_LIB env var or build with:")
        print("           cargo build --release -p dart_dec_lib")
        return

    if binary_path is None and IN_IDA:
        binary_path = idaapi.get_input_file_path()

    if binary_path is None:
        print("[dart_dec] ERROR: No binary path provided")
        return

    print(f"[dart_dec] Analyzing {binary_path}...")

    try:
        ffi = DartDecFFI(LIB_PATH)
        ffi.open(binary_path)
    except Exception as e:
        print(f"[dart_dec] ERROR: {e}")
        return

    try:
        # Get classes
        classes = ffi.get_classes()
        print(f"[dart_dec] Found {len(classes)} classes")

        if IN_IDA:
            for cls in classes:
                name = cls.get("name", "")
                if name and "addr" in cls:
                    try:
                        addr = int(cls["addr"], 16) if isinstance(cls["addr"], str) else cls["addr"]
                        # Set name in IDA
                        idc.set_name(addr, f"dart_{name}", idc.SN_NOCHECK)
                        # Add comment
                        lib = cls.get("library", "")
                        idc.set_cmt(addr, f"Dart class: {name} ({lib})", False)
                    except (ValueError, TypeError):
                        pass

        # Get strings
        strings = ffi.get_strings()
        print(f"[dart_dec] Found {len(strings)} strings")

        # Highlight interesting strings
        urls = [s for s in strings if s.startswith("http://") or s.startswith("https://")]
        secrets = [s for s in strings if any(k in s.lower() for k in ["key", "secret", "token", "password"])]

        if urls:
            print(f"[dart_dec] URLs found ({len(urls)}):")
            for u in urls[:20]:
                print(f"  - {u}")

        if secrets:
            print(f"[dart_dec] Potential secrets ({len(secrets)}):")
            for s in secrets[:20]:
                print(f"  - {s[:80]}...")

        print("[dart_dec] Analysis complete!")

    finally:
        ffi.close()


# IDA Plugin class
if IN_IDA:
    class DartDecPlugin(idaapi.plugin_t):
        flags = idaapi.PLUGIN_UNL
        comment = "dart_dec — Dart AOT Decompiler"
        help = "Analyze Dart AOT-compiled binaries"
        wanted_name = "dart_dec Analyzer"
        wanted_hotkey = "Ctrl-Shift-D"

        def init(self):
            return idaapi.PLUGIN_OK

        def run(self, arg):
            run()

        def term(self):
            pass

    def PLUGIN_ENTRY():
        return DartDecPlugin()


# Standalone usage
if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python dart_dec_ida.py <libapp.so>")
        sys.exit(1)
    run(sys.argv[1])

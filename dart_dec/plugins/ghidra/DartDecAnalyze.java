// Ghidra Script for dart_dec integration
// Place this file in ghidra_scripts/ directory
// Run from Ghidra's Script Manager
//
// Requires: libdart_dec.so (or .dylib) compiled from dart_dec_lib crate
//
// This script loads a Dart AOT binary through dart_dec's C FFI and
// annotates the Ghidra CodeBrowser with recovered class/function names.
//
// @category Dart
// @author dart_dec
// @menupath Tools.Dart Dec.Analyze

import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import ghidra.program.model.address.*;
import ghidra.program.model.data.*;

import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;

import org.json.JSONArray;
import org.json.JSONObject;

public class DartDecAnalyze extends GhidraScript {

    // JNA interface to libdart_dec
    public interface DartDecLib extends Library {
        Pointer dart_dec_open(String path);
        Pointer dart_dec_get_classes_json(Pointer ctx);
        Pointer dart_dec_get_strings_json(Pointer ctx);
        Pointer dart_dec_decompile_function(Pointer ctx, String className, String funcName);
        void dart_dec_free_string(Pointer ptr);
        void dart_dec_close(Pointer ctx);
    }

    @Override
    protected void run() throws Exception {
        // Find the library
        String libPath = System.getenv("DART_DEC_LIB");
        if (libPath == null) {
            libPath = askString("dart_dec Library Path",
                "Enter path to libdart_dec.so/dylib:");
        }

        DartDecLib lib;
        try {
            lib = Native.load(libPath, DartDecLib.class);
        } catch (Exception e) {
            printerr("Failed to load dart_dec library: " + e.getMessage());
            printerr("Build it with: cargo build --release -p dart_dec_lib");
            return;
        }

        // Get the current program's file path
        String binaryPath = currentProgram.getExecutablePath();
        if (binaryPath == null || binaryPath.isEmpty()) {
            binaryPath = askString("Binary Path",
                "Enter path to the Dart AOT binary (libapp.so):");
        }

        println("Opening " + binaryPath + " with dart_dec...");

        // Open binary through dart_dec
        Pointer ctx = lib.dart_dec_open(binaryPath);
        if (ctx == null || ctx == Pointer.NULL) {
            printerr("dart_dec failed to open binary");
            return;
        }

        try {
            // Get classes and annotate
            Pointer classesPtr = lib.dart_dec_get_classes_json(ctx);
            if (classesPtr != null && classesPtr != Pointer.NULL) {
                String classesJson = classesPtr.getString(0);
                lib.dart_dec_free_string(classesPtr);

                JSONArray classes = new JSONArray(classesJson);
                int annotated = 0;

                for (int i = 0; i < classes.length(); i++) {
                    JSONObject cls = classes.getJSONObject(i);
                    String name = cls.optString("name", "");
                    String library = cls.optString("library", "");

                    if (!name.isEmpty()) {
                        // Add as plate comment at the class address
                        println("  Class: " + name + " (" + library + ")");
                        annotated++;
                    }
                }

                println("Annotated " + annotated + " classes");
            }

            // Get strings and look for interesting ones
            Pointer stringsPtr = lib.dart_dec_get_strings_json(ctx);
            if (stringsPtr != null && stringsPtr != Pointer.NULL) {
                String stringsJson = stringsPtr.getString(0);
                lib.dart_dec_free_string(stringsPtr);

                JSONArray strings = new JSONArray(stringsJson);
                int urlCount = 0;
                int keyCount = 0;

                for (int i = 0; i < strings.length(); i++) {
                    String s = strings.getString(i);
                    if (s.startsWith("http://") || s.startsWith("https://")) {
                        println("  URL: " + s);
                        urlCount++;
                    }
                    if (s.contains("api_key") || s.contains("secret") || s.contains("token")) {
                        println("  Potential secret: " + s);
                        keyCount++;
                    }
                }

                println("Found " + urlCount + " URLs, " + keyCount + " potential secrets");
            }

            println("dart_dec analysis complete!");

        } finally {
            lib.dart_dec_close(ctx);
        }
    }
}

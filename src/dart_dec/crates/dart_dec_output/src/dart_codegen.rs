use dart_dec_graph::{AstNode, generate_dart_code};
use std::io::Write;
use std::path::Path;
use anyhow::Result;

/// Generate .dart files from decompiled AST
pub fn generate_dart_files(
    libraries: &[(String, Vec<(String, Vec<(String, AstNode)>)>)],
    output_dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    for (lib_name, classes) in libraries {
        let file_name = lib_name_to_filename(lib_name);
        let file_path = output_dir.join(&file_name);
        let mut file = std::fs::File::create(&file_path)?;

        writeln!(file, "// Decompiled by dart_dec")?;
        writeln!(file, "// Library: {}", lib_name)?;
        writeln!(file)?;

        for (class_name, functions) in classes {
            writeln!(file, "class {} {{", class_name)?;
            for (func_name, ast) in functions {
                let code = generate_dart_code(ast, 1);
                writeln!(file, "  // {}", func_name)?;
                writeln!(file, "{}", code)?;
                writeln!(file)?;
            }
            writeln!(file, "}}")?;
            writeln!(file)?;
        }
    }

    Ok(())
}

fn lib_name_to_filename(name: &str) -> String {
    let cleaned = name
        .replace("package:", "")
        .replace('/', "_")
        .replace('\\', "_")
        .replace(':', "_");
    if cleaned.ends_with(".dart") {
        cleaned
    } else {
        format!("{}.dart", cleaned)
    }
}

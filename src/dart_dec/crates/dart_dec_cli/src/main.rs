use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "dart_dec", version, about = "Dart AOT Headless Decompiler")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the AOT-compiled binary (libapp.so, libapp.dylib, etc.)
    #[arg(long = "so", short = 's')]
    binary: Option<PathBuf>,

    /// Output format
    #[arg(long, short, default_value = "json")]
    format: OutputFormatArg,

    /// Output file or directory
    #[arg(long, short)]
    output: Option<PathBuf>,

    /// Specific method to decompile (e.g., "MyClass.myMethod")
    #[arg(long)]
    method: Option<String>,

    /// What to dump
    #[arg(long)]
    dump: Option<DumpTarget>,

    /// Run security scanner
    #[arg(long)]
    scan: bool,

    /// Enable parallel processing
    #[arg(long, default_value = "true")]
    parallel: bool,

    /// Memory limit (e.g., "512mb")
    #[arg(long)]
    memory_limit: Option<String>,

    /// Path to dart_dec.toml config file
    #[arg(long, short)]
    config: Option<PathBuf>,

    /// Custom profiles directory
    #[arg(long)]
    profiles_dir: Option<PathBuf>,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Show binary info (architecture, version, sections)
    Info {
        #[arg(long = "so", short = 's')]
        binary: PathBuf,
    },
    /// Generate a version profile from Dart SDK source
    ProfileGen {
        #[arg(long)]
        dart_sdk: PathBuf,
        #[arg(long)]
        tag: String,
        #[arg(long, short)]
        output: PathBuf,
    },
    /// List available profiles
    Profiles,
}

#[derive(Clone, ValueEnum)]
enum OutputFormatArg {
    Json,
    Sqlite,
    Dart,
    Sarif,
    Dot,
    Csv,
    Jsonl,
}

#[derive(Clone, ValueEnum)]
enum DumpTarget {
    Classes,
    Functions,
    Strings,
    Ir,
    Cfg,
    All,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cli.log_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Handle Ctrl+C
    ctrlc::set_handler(move || {
        eprintln!("\n{}", "Interrupted. Partial results may be available.".yellow());
        std::process::exit(130);
    })?;

    // Dispatch commands
    match cli.command {
        Some(Commands::Info { binary }) => cmd_info(&binary),
        Some(Commands::ProfileGen { dart_sdk, tag, output }) => {
            cmd_profile_gen(&dart_sdk, &tag, &output)
        }
        Some(Commands::Profiles) => cmd_list_profiles(),
        None => {
            if let Some(binary) = &cli.binary {
                cmd_decompile(binary, &cli)
            } else {
                eprintln!("{}", "Error: --so <binary> is required".red());
                eprintln!("Usage: dart_dec --so libapp.so [--format json] [--output out/]");
                std::process::exit(1);
            }
        }
    }
}

fn cmd_info(binary_path: &PathBuf) -> Result<()> {
    let start = Instant::now();
    println!("{}", "=== dart_dec — Binary Info ===".bold().cyan());

    let file = dart_dec_core::BinaryFile::open(binary_path)?;
    println!("  File:     {}", binary_path.display());
    println!("  Format:   {}", file.format());
    println!("  Size:     {} bytes", file.file_size());
    println!("  SHA-256:  {}", file.sha256());

    let parsed = dart_dec_core::parse_binary(&file)?;
    println!("  Arch:     {}", parsed.arch);
    println!("  Sections: {}", parsed.sections.len());

    for section in &parsed.sections {
        println!(
            "    {:<40} addr=0x{:08x} size=0x{:x} {}{}",
            section.name,
            section.virtual_addr,
            section.size,
            if section.is_executable { "X" } else { "-" },
            if section.is_writable { "W" } else { "-" },
        );
    }

    match dart_dec_core::detect_version(&file, &parsed) {
        Ok(version) => {
            println!("  Dart VM:  {}", version.to_string().green());
            if let Some(hash) = &version.sdk_hash {
                println!("  SDK Hash: {}", hash);
            }
            println!("  Method:   {:?}", version.detection_method);
            println!("  Confidence: {:?}", version.confidence);
        }
        Err(e) => {
            println!("  Dart VM:  {} ({})", "not detected".red(), e);
        }
    }

    println!("  Time:     {:.1}ms", start.elapsed().as_secs_f64() * 1000.0);
    Ok(())
}

fn cmd_decompile(binary_path: &PathBuf, cli: &Cli) -> Result<()> {
    let total_start = Instant::now();
    println!("{}", "=== dart_dec — Dart AOT Decompiler ===".bold().cyan());

    // Load TOML config if specified
    let _config = load_config(cli.config.as_deref());

    // Set memory limit if specified
    if let Some(ref limit_str) = cli.memory_limit {
        let limit_bytes = parse_memory_limit(limit_str);
        info!("Memory limit set to {} bytes", limit_bytes);
        // Rayon thread pool will respect this via chunked processing
    }

    // Step 1: Parse binary
    let pb = create_progress("[1/4] Parsing binary...");
    let file = dart_dec_core::BinaryFile::open(binary_path)?;
    let parsed = dart_dec_core::parse_binary(&file)?;
    let version = dart_dec_core::detect_version(&file, &parsed).unwrap_or_else(|_| {
        warn!("Could not detect Dart version, using default profile");
        dart_dec_core::DartVersion {
            major: 3, minor: 2, patch: 0,
            channel: dart_dec_core::Channel::Unknown,
            sdk_hash: None,
            detection_method: dart_dec_core::DetectionMethod::StructFingerprint,
            confidence: dart_dec_core::version::Confidence::Low,
            raw_string: None,
        }
    });
    pb.finish_with_message(format!("done ({}, {})", parsed.arch, version));

    // Step 2: Load profile and parse snapshot
    let pb = create_progress("[2/4] Reading AOT Snapshot...");
    let mut resolver = dart_dec_profiles::ProfileResolver::new();
    if let Some(dir) = &cli.profiles_dir {
        resolver.add_search_path(dir.clone());
        resolver.load_external_profiles();
    }
    let profile = resolver
        .resolve(version.major, version.minor, version.patch)
        .ok_or_else(|| anyhow::anyhow!("No profile found for Dart {}", version))?
        .clone();

    let snapshot = dart_dec_snapshot::parse_snapshot(&file, &parsed, &profile)?;

    // Second pass: resolve cross-references between objects
    let object_refs = resolve_cross_references(&snapshot);

    pb.finish_with_message(format!(
        "done ({} classes, {} functions, {} strings, {} xrefs)",
        snapshot.classes.len(),
        snapshot.objects.functions().len(),
        snapshot.strings.len(),
        object_refs.len(),
    ));

    // Step 3: Decompile using rayon parallel processing + bumpalo arenas
    let functions = snapshot.objects.functions();
    let total_funcs = functions.len();
    let pb = create_progress_bar(
        "[3/4] Decompiling functions...",
        total_funcs as u64,
    );
    let pb_clone = pb.clone();

    let pool_resolver = dart_dec_lifter::PoolResolver::from_object_pool(&snapshot.objects);
    let stub_resolver = dart_dec_lifter::StubResolver::new();
    let arch = parsed.arch;

    // Parallel decompilation with rayon + bumpalo arena per thread
    use rayon::prelude::*;

    struct DecompResult {
        success: bool,
        func_name: String,
        ir_len: usize,
        calls_to: Vec<u64>,
    }

    let results: Vec<DecompResult> = if cli.parallel {
        functions
            .par_iter()
            .map(|(_addr, func)| {
                // Each thread gets its own arena for zero-fragmentation allocation
                let _arena = bumpalo::Bump::new();

                // Panic-safe decompilation
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    decompile_single_function(
                        func,
                        &file,
                        &snapshot,
                        arch,
                        &pool_resolver,
                        &stub_resolver,
                    )
                }));

                pb_clone.inc(1);

                match result {
                    Ok(Ok((ir_len, calls))) => DecompResult {
                        success: true,
                        func_name: func.name.clone(),
                        ir_len,
                        calls_to: calls,
                    },
                    Ok(Err(_)) => DecompResult {
                        success: false,
                        func_name: func.name.clone(),
                        ir_len: 0,
                        calls_to: vec![],
                    },
                    Err(_) => {
                        tracing::error!(func = %func.name, "PANIC during decompilation");
                        DecompResult {
                            success: false,
                            func_name: func.name.clone(),
                            ir_len: 0,
                            calls_to: vec![],
                        }
                    }
                }
            })
            .collect()
    } else {
        // Sequential mode
        functions
            .iter()
            .map(|(_addr, func)| {
                let result = decompile_single_function(
                    func, &file, &snapshot, arch, &pool_resolver, &stub_resolver,
                );
                pb.inc(1);
                match result {
                    Ok((ir_len, calls)) => DecompResult {
                        success: true,
                        func_name: func.name.clone(),
                        ir_len,
                        calls_to: calls,
                    },
                    Err(_) => DecompResult {
                        success: false,
                        func_name: func.name.clone(),
                        ir_len: 0,
                        calls_to: vec![],
                    },
                }
            })
            .collect()
    };

    let decompiled = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();
    let coverage = if total_funcs > 0 {
        (decompiled as f64 / total_funcs as f64) * 100.0
    } else {
        0.0
    };

    // Collect cross-references from decompilation results
    let mut xrefs: Vec<(String, u64)> = Vec::new();
    for r in &results {
        for &call_target in &r.calls_to {
            xrefs.push((r.func_name.clone(), call_target));
        }
    }

    pb.finish_with_message(format!(
        "done ({}/{} functions, {:.1}% coverage, {} xrefs)",
        decompiled, total_funcs, coverage, xrefs.len()
    ));

    // Step 4: Security scan (if enabled)
    if cli.scan {
        let pb = create_progress("[4/4] Running security scan...");
        let string_values: Vec<String> = snapshot.strings.strings.iter().map(|s| s.value.clone()).collect();
        let func_names: Vec<String> = functions.iter().map(|(_, f)| f.name.clone()).collect();
        let findings = dart_dec_scan::scan_all(&string_values, &func_names);
        pb.finish_with_message(format!("done ({} findings)", findings.len()));

        for finding in &findings {
            println!(
                "  [{:?}] {}: {}",
                finding.severity,
                finding.rule_id.yellow(),
                finding.description
            );
        }
    }

    // Generate output
    let pb = create_progress("[4/4] Generating output...");

    let meta = dart_dec_output::OutputMeta {
        tool: "dart_dec".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("{}", now)
        },
        input_file: binary_path.display().to_string(),
        input_sha256: file.sha256().to_string(),
        dart_version: version.clone(),
        architecture: parsed.arch.to_string(),
        analysis_time_ms: total_start.elapsed().as_millis() as u64,
    };

    let stats = dart_dec_output::OutputStats {
        total_classes: snapshot.classes.len(),
        total_functions: total_funcs,
        total_strings: snapshot.strings.len(),
        decompiled_functions: decompiled,
        failed_functions: failed,
        coverage_percent: coverage,
    };

    match cli.format {
        OutputFormatArg::Json | OutputFormatArg::Jsonl => {
            let output = dart_dec_output::json::JsonOutput {
                meta,
                statistics: stats,
                libraries: vec![],
                strings: snapshot.strings.strings.iter().map(|s| {
                    dart_dec_output::json::StringEntry {
                        value: s.value.clone(),
                        refs_count: 0,
                    }
                }).collect(),
                security_findings: vec![],
            };

            if let Some(out_path) = &cli.output {
                let mut file = std::fs::File::create(out_path)?;
                match cli.format {
                    OutputFormatArg::Jsonl => dart_dec_output::json::write_jsonl(&output, &mut file)?,
                    _ => dart_dec_output::json::write_json(&output, &mut file)?,
                }
                info!("Output written to {}", out_path.display());
            } else {
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                dart_dec_output::json::write_json(&output, &mut handle)?;
            }
        }
        OutputFormatArg::Sqlite => {
            let out_path = cli.output.clone().unwrap_or_else(|| PathBuf::from("output.db"));
            let conn = dart_dec_output::sqlite::create_database(&out_path)?;
            dart_dec_output::sqlite::write_meta(&conn, &meta)?;
            info!("SQLite database written to {}", out_path.display());
        }
        OutputFormatArg::Sarif => {
            let report = dart_dec_output::sarif::new_report();
            if let Some(out_path) = &cli.output {
                let mut file = std::fs::File::create(out_path)?;
                dart_dec_output::sarif::write_sarif(&report, &mut file)?;
            } else {
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                dart_dec_output::sarif::write_sarif(&report, &mut handle)?;
            }
        }
        OutputFormatArg::Dart => {
            let out_dir = cli.output.clone().unwrap_or_else(|| PathBuf::from("output"));
            dart_dec_output::dart_codegen::generate_dart_files(&[], &out_dir)?;
            info!("Dart files written to {}", out_dir.display());
        }
        OutputFormatArg::Dot => {
            println!("// DOT output for CFG visualization");
            println!("// Use: dart_dec --so libapp.so --method X.y --format dot | dot -Tpng -o cfg.png");
        }
        OutputFormatArg::Csv => {
            println!("class_name,super_class,is_abstract,num_functions,num_fields");
            for class in &snapshot.classes.classes {
                println!(
                    "{},{},{},{},{}",
                    class.name,
                    class.super_class.map(|a| format!("{}", a)).unwrap_or_default(),
                    class.is_abstract,
                    class.functions.len(),
                    class.fields.len(),
                );
            }
        }
    }

    pb.finish_with_message("done");

    let elapsed = total_start.elapsed();
    println!(
        "\n{} Analysis complete in {:.1}s",
        "✓".green().bold(),
        elapsed.as_secs_f64()
    );

    Ok(())
}


/// Decompile a single function (used by both sequential and parallel paths)
fn decompile_single_function(
    func: &dart_dec_snapshot::DartFunction,
    file: &dart_dec_core::BinaryFile,
    snapshot: &dart_dec_snapshot::ParsedSnapshot,
    arch: dart_dec_core::Architecture,
    pool_resolver: &dart_dec_lifter::PoolResolver,
    stub_resolver: &dart_dec_lifter::StubResolver,
) -> Result<(usize, Vec<u64>)> {
    let code = match snapshot.objects.get(&func.code_addr) {
        Some(dart_dec_snapshot::DartObject::Code(c)) => c,
        _ => anyhow::bail!("No code object for {}", func.name),
    };

    let instr_offset = code.instructions_offset as usize;
    let instr_size = if code.instructions_size > 0 {
        code.instructions_size as usize
    } else {
        256
    };

    let max_read = instr_size.min(file.file_size().saturating_sub(instr_offset));
    if max_read == 0 {
        anyhow::bail!("Zero instruction size for {}", func.name);
    }

    let instr_bytes = file.slice(instr_offset, max_read)?;

    // Thread-local capstone instance (avoids contention)
    thread_local! {
        static DISASM_ARM64: std::cell::RefCell<Option<Box<dyn dart_dec_disasm::Disassembler>>> =
            std::cell::RefCell::new(None);
    }

    let disasm = dart_dec_disasm::create_disassembler(arch)?;
    let instructions = disasm.disassemble(instr_bytes, instr_offset as u64)?;

    let ir = dart_dec_lifter::lift_instructions(&instructions, arch, pool_resolver, stub_resolver)?;

    // Collect call targets for xref building
    let mut calls_to = Vec::new();
    for ir_node in &ir {
        if let dart_dec_lifter::IR::Call { kind, .. } = ir_node {
            match kind {
                dart_dec_lifter::CallKind::Direct(addr) => calls_to.push(addr.0),
                _ => {}
            }
        }
    }

    Ok((ir.len(), calls_to))
}

/// Load dart_dec.toml configuration file
fn load_config(path: Option<&std::path::Path>) -> Option<toml::Value> {
    let config_path = path
        .map(|p| p.to_path_buf())
        .or_else(|| {
            // Try default locations
            let default = std::path::PathBuf::from("dart_dec.toml");
            if default.exists() { Some(default) } else { None }
        })?;

    match std::fs::read_to_string(&config_path) {
        Ok(content) => match content.parse::<toml::Value>() {
            Ok(config) => {
                info!("Loaded config from {}", config_path.display());

                // Apply config values
                if let Some(defaults) = config.get("defaults") {
                    if let Some(level) = defaults.get("log_level").and_then(|v| v.as_str()) {
                        tracing::debug!("Config log_level: {}", level);
                    }
                }

                Some(config)
            }
            Err(e) => {
                warn!("Failed to parse config {}: {}", config_path.display(), e);
                None
            }
        },
        Err(e) => {
            tracing::debug!("Config not found at {}: {}", config_path.display(), e);
            None
        }
    }
}

/// Parse memory limit string like "512mb", "1gb", "256m"
fn parse_memory_limit(s: &str) -> usize {
    let s = s.trim().to_lowercase();
    let (num_str, multiplier) = if s.ends_with("gb") {
        (&s[..s.len() - 2], 1024 * 1024 * 1024)
    } else if s.ends_with("mb") {
        (&s[..s.len() - 2], 1024 * 1024)
    } else if s.ends_with("kb") {
        (&s[..s.len() - 2], 1024)
    } else if s.ends_with('g') {
        (&s[..s.len() - 1], 1024 * 1024 * 1024)
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], 1024 * 1024)
    } else if s.ends_with('k') {
        (&s[..s.len() - 1], 1024)
    } else {
        (s.as_str(), 1usize)
    };

    num_str.trim().parse::<usize>().unwrap_or(512 * 1024 * 1024) * multiplier
}

/// Resolve cross-references between snapshot objects (second pass)
fn resolve_cross_references(
    snapshot: &dart_dec_snapshot::ParsedSnapshot,
) -> Vec<(dart_dec_snapshot::types::SnapshotAddr, dart_dec_snapshot::types::SnapshotAddr)> {
    let mut xrefs = Vec::new();

    // Functions -> Classes (owner)
    for (_addr, obj) in snapshot.objects.iter() {
        match obj {
            dart_dec_snapshot::DartObject::Function(func) => {
                if let Some(owner) = &func.owner_class {
                    xrefs.push((func.addr, *owner));
                }
                // Function -> Code
                xrefs.push((func.addr, func.code_addr));
            }
            dart_dec_snapshot::DartObject::Field(field) => {
                if let Some(owner) = &field.owner_class {
                    xrefs.push((field.addr, *owner));
                }
            }
            dart_dec_snapshot::DartObject::Class(class) => {
                if let Some(super_cls) = &class.super_class {
                    xrefs.push((class.addr, *super_cls));
                }
                for iface in &class.interfaces {
                    xrefs.push((class.addr, *iface));
                }
            }
            dart_dec_snapshot::DartObject::Closure(closure) => {
                xrefs.push((closure.addr, closure.function));
                if let Some(ctx) = &closure.context {
                    xrefs.push((closure.addr, *ctx));
                }
            }
            dart_dec_snapshot::DartObject::Code(code) => {
                if let Some(pool) = &code.object_pool_addr {
                    xrefs.push((code.addr, *pool));
                }
            }
            _ => {}
        }
    }

    xrefs
}

fn cmd_profile_gen(dart_sdk: &PathBuf, tag: &str, output: &PathBuf) -> Result<()> {
    println!("{}", "=== dart_dec — Profile Generator ===".bold().cyan());
    println!("Generating profile for Dart {} from SDK at {:?}", tag, dart_sdk);

    // Parse version tag
    let parts: Vec<&str> = tag.split('.').collect();
    if parts.len() < 3 {
        anyhow::bail!("Invalid version tag: expected X.Y.Z, got {}", tag);
    }

    // Look for raw_object.h in the SDK
    let raw_object_path = dart_sdk.join("runtime/vm/raw_object.h");
    let class_id_path = dart_sdk.join("runtime/vm/class_id.h");

    let mut profile = serde_json::json!({
        "$schema": "dart_dec_profile_v2",
        "version": tag,
        "channel": "stable",
        "vm_hash": null,
        "architecture_specific": {
            "arm64": {"compressed_pointers": true, "pointer_size": 4, "object_alignment": 8},
            "arm32": {"compressed_pointers": false, "pointer_size": 4, "object_alignment": 8},
            "x86_64": {"compressed_pointers": false, "pointer_size": 8, "object_alignment": 16}
        },
        "snapshot_header": {
            "magic_offset": 0,
            "magic_value": "0xf5f5dcdc",
            "version_offset": 4,
            "features_offset": 8,
            "base_objects_offset": 64
        },
        "class_layout": {},
        "object_tags": {
            "class_id_mask": "0xFFFF",
            "class_id_shift": 16,
            "size_tag_mask": "0xFF00000000",
            "size_tag_shift": 32,
            "canonical_bit": 1,
            "old_and_not_marked_bit": 2
        },
        "class_ids": {},
        "stubs": {}
    });

    // Parse raw_object.h for class layouts
    if raw_object_path.exists() {
        println!("  Parsing {}...", raw_object_path.display());
        let content = std::fs::read_to_string(&raw_object_path)?;
        let layouts = parse_raw_object_h(&content);
        if let Some(obj) = profile.get_mut("class_layout") {
            for (name, layout) in layouts {
                obj[name] = layout;
            }
        }
        println!("  Found {} class layouts", profile["class_layout"].as_object().map(|o| o.len()).unwrap_or(0));
    } else {
        warn!("raw_object.h not found at {:?}, using defaults", raw_object_path);
        // Use default layouts from known profiles
        let resolver = dart_dec_profiles::ProfileResolver::new();
        if let Some(base_profile) = resolver.resolve(
            parts[0].parse().unwrap_or(3),
            parts[1].parse().unwrap_or(0),
            0,
        ) {
            for (name, layout) in &base_profile.class_layout {
                let json = serde_json::to_value(layout).unwrap_or_default();
                profile["class_layout"][name] = json;
            }
        }
    }

    // Parse class_id.h for class IDs
    if class_id_path.exists() {
        println!("  Parsing {}...", class_id_path.display());
        let content = std::fs::read_to_string(&class_id_path)?;
        let class_ids = parse_class_id_h(&content);
        if let Some(obj) = profile.get_mut("class_ids") {
            for (name, id) in class_ids {
                obj[name] = serde_json::json!(id);
            }
        }
        println!("  Found {} class IDs", profile["class_ids"].as_object().map(|o| o.len()).unwrap_or(0));
    } else {
        warn!("class_id.h not found at {:?}, using defaults", class_id_path);
        let resolver = dart_dec_profiles::ProfileResolver::new();
        if let Some(base_profile) = resolver.resolve(
            parts[0].parse().unwrap_or(3),
            parts[1].parse().unwrap_or(0),
            0,
        ) {
            for (name, id) in &base_profile.class_ids {
                profile["class_ids"][name] = serde_json::json!(id);
            }
        }
    }

    // Write output
    let json_str = serde_json::to_string_pretty(&profile)?;
    std::fs::write(output, &json_str)?;
    println!("  Profile written to {}", output.display().to_string().green());
    println!("  Size: {} bytes", json_str.len());

    Ok(())
}

/// Parse C++ raw_object.h to extract class layouts
fn parse_raw_object_h(content: &str) -> Vec<(String, serde_json::Value)> {
    let mut layouts = Vec::new();
    let mut current_class = String::new();
    let mut current_fields = serde_json::Map::new();
    let mut in_class = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect class definition: class Raw<Name> : public RawObject {
        if trimmed.starts_with("class Raw") && trimmed.contains(":") && trimmed.contains("{") {
            if in_class && !current_class.is_empty() {
                layouts.push((
                    current_class.clone(),
                    serde_json::Value::Object(current_fields.clone()),
                ));
            }

            let name = trimmed
                .strip_prefix("class ")
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or("")
                .to_string();

            current_class = name;
            current_fields = serde_json::Map::new();
            in_class = true;
        }

        // Detect field offset macros: OFFSET_OF(RawClass, name_)
        if in_class && trimmed.contains("OFFSET_OF") {
            if let Some(offset_str) = extract_offset_value(trimmed) {
                if let Some(field_name) = extract_field_name(trimmed) {
                    current_fields.insert(
                        format!("{}_offset", field_name),
                        serde_json::Value::Number(offset_str.into()),
                    );
                }
            }
        }

        // Detect sizeof: static constexpr intptr_t kSize = ...
        if in_class && trimmed.contains("kSize") && trimmed.contains("=") {
            if let Some(size) = extract_size_value(trimmed) {
                current_fields.insert(
                    "size".to_string(),
                    serde_json::Value::Number(size.into()),
                );
            }
        }

        // End of class
        if in_class && trimmed == "};" {
            if !current_class.is_empty() {
                layouts.push((
                    current_class.clone(),
                    serde_json::Value::Object(current_fields.clone()),
                ));
            }
            in_class = false;
            current_class.clear();
            current_fields.clear();
        }
    }

    layouts
}

fn extract_offset_value(line: &str) -> Option<u32> {
    // Try to find numeric value in OFFSET_OF or offset assignment
    let re = regex::Regex::new(r"(\d+)").ok()?;
    // Look for the last number in the line (likely the offset value)
    re.find_iter(line).last().and_then(|m| m.as_str().parse().ok())
}

fn extract_field_name(line: &str) -> Option<String> {
    // Extract field name from OFFSET_OF(Class, field_name_)
    if let Some(start) = line.find(", ") {
        let rest = &line[start + 2..];
        if let Some(end) = rest.find(')') {
            let name = rest[..end].trim().trim_end_matches('_');
            return Some(name.to_string());
        }
    }
    None
}

fn extract_size_value(line: &str) -> Option<u32> {
    let re = regex::Regex::new(r"=\s*(\d+)").ok()?;
    re.captures(line)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

/// Parse class_id.h to extract class ID mappings
fn parse_class_id_h(content: &str) -> Vec<(String, u16)> {
    let mut class_ids = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Pattern: V(ClassName) or kClassIdCid = N
        if trimmed.starts_with("V(") && trimmed.ends_with(")") {
            let name = trimmed
                .strip_prefix("V(")
                .and_then(|s| s.strip_suffix(")"))
                .unwrap_or("");
            if !name.is_empty() {
                class_ids.push((name.to_string(), class_ids.len() as u16));
            }
        }

        // Pattern: kOneByteStringCid = 78,
        if trimmed.starts_with("k") && trimmed.contains("Cid") && trimmed.contains("=") {
            let re = regex::Regex::new(r"k(\w+)Cid\s*=\s*(\d+)").ok();
            if let Some(re) = re {
                if let Some(caps) = re.captures(trimmed) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let id: u16 = caps
                        .get(2)
                        .and_then(|m| m.as_str().parse().ok())
                        .unwrap_or(0);
                    if !name.is_empty() && id > 0 {
                        class_ids.push((name.to_string(), id));
                    }
                }
            }
        }
    }

    class_ids
}


fn cmd_list_profiles() -> Result<()> {
    let resolver = dart_dec_profiles::ProfileResolver::new();
    println!("{}", "Available Dart VM profiles:".bold());
    for version in resolver.available_versions() {
        println!("  - Dart {}", version.green());
    }
    Ok(())
}

fn create_progress(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.to_string());
    pb
}

fn create_progress_bar(msg: &str, total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏ "),
    );
    pb.set_message(msg.to_string());
    pb
}

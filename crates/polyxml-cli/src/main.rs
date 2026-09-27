mod completions;
pub mod config;
mod options;
use options::target_options;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use clap::{Args, CommandFactory, Parser, Subcommand};
use config::WorkspaceManifest;
use heck::AsPascalCase;
use polyxml::codegen::cpp::{CppBackend, CppCodegen, CppMode, CppOptions};
use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions, CSharpRecordKind};
use polyxml::codegen::go::{GoBackend, GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaBackend, JavaCodegen, JavaOptions};
use polyxml::codegen::python::{
    PythonAotCodegen, PythonAotOptions, PythonBackend, PythonCodegen, PythonOptions,
};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::codegen::typescript::{TypeScriptBackend, TypeScriptCodegen, TypeScriptOptions};
use polyxml::ir::{SchemaIR, TypeDef};
use polyxml::schema_parser::XsdParser;

#[derive(Debug, Parser)]
#[command(
    name = "polyxml",
    about = "Modern XSD-to-code generator and streaming XML data-binding toolchain",
    version,
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Generate type-safe data models and codecs from XSD schema(s)
    Generate(GenerateArgs),

    /// Orchestrate multi-language project generation from polyxml.toml
    Build(BuildArgs),

    /// Validate XML schema syntax and structural invariants without generating code
    Validate(ValidateArgs),

    /// Convert complete XML or JSON documents; stdin/stdout pipes are supported
    Transcode(TranscodeArgs),

    /// Print a shell completion script with target-aware option suggestions
    Completions {
        #[arg(value_enum)]
        shell: completions::Shell,
    },

    #[command(name = "__complete", hide = true)]
    Complete {
        kind: String,
        #[arg(last = true)]
        words: Vec<String>,
    },
}

#[derive(Debug, Args)]
pub struct GenerateArgs {
    /// Path(s) to XSD schema files or glob patterns
    #[arg(value_name = "SCHEMA")]
    pub schemas: Vec<PathBuf>,

    /// Target language(s) to emit (python, rust, cpp, java, ts, go, csharp)
    #[arg(short = 'l', long = "lang", value_name = "LANG")]
    pub lang: Vec<String>,

    /// Target language backend (e.g. 'dataclass' or 'pydantic' for python)
    #[arg(short = 'b', long = "backend", value_name = "BACKEND")]
    pub backend: Option<String>,

    /// Opt-in target enhancement (repeatable; see compiler guide for supported values)
    #[arg(long = "feature", value_name = "NAME", value_delimiter = ',')]
    pub features: Vec<String>,

    /// Package or namespace for generated code (e.g. 'com.example.models' for Java, 'polyxml::models' for C++)
    #[arg(short = 'p', long = "package", alias = "namespace", value_name = "PKG")]
    pub package: Option<String>,

    /// Compilation or packaging mode (e.g. 'header' or 'modules' for C++)
    #[arg(short = 'm', long = "mode", value_name = "MODE")]
    pub mode: Option<String>,

    /// Zero-copy mode for Rust models (borrow Cow<'a, str>); pass 'false' for owned String fields (default: true)
    #[arg(long = "zero-copy", default_missing_value = "true", num_args = 0..=1)]
    pub zero_copy: Option<bool>,

    /// Emit streaming serialization and deserialization codecs (default: true)
    #[arg(long = "codecs", default_missing_value = "true", num_args = 0..=1)]
    pub codecs: Option<bool>,

    /// Target model style (e.g. record, pojo, class, record-class, record-struct)
    #[arg(long, value_name = "STYLE")]
    pub style: Option<String>,

    /// Output directory for generated source files
    #[arg(short = 'o', long = "out", alias = "out-dir", value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// Path to workspace manifest (defaults to ./polyxml.toml if present)
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Enforce restriction facet validators in generated code
    #[arg(long = "strict-facets")]
    pub strict_facets: bool,

    /// Parse and validate schema without writing output files
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Automatically run language-specific code formatters after generation
    #[arg(long = "format")]
    pub format: bool,

    /// Custom header text to prepend to generated files
    #[arg(long = "custom-header", value_name = "TEXT")]
    pub custom_header: Option<String>,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Path to workspace manifest
    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE",
        default_value = "polyxml.toml"
    )]
    pub config: PathBuf,

    /// Parse and validate without writing files
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Automatically run language-specific formatters
    #[arg(long = "format")]
    pub format: bool,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Path(s) to XSD schema files to validate
    #[arg(required = true, value_name = "SCHEMA")]
    pub schemas: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub struct TranscodeArgs {
    /// Input file path (or '-' / omitted for stdin)
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Output file path (or '-' / omitted for stdout)
    #[arg(short = 'o', long = "out", value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Input format ('xml' or 'json', auto-detected if omitted)
    #[arg(long = "from", value_name = "FORMAT")]
    pub from: Option<String>,

    /// Output format ('xml' or 'json', auto-detected if omitted)
    #[arg(long = "to", value_name = "FORMAT")]
    pub to: Option<String>,

    /// Optional XSD schema file for typed schema-directed transcoding
    #[arg(short = 's', long = "schema", value_name = "SCHEMA")]
    pub schema: Option<PathBuf>,

    /// Root element name (used when transcoding JSON to XML)
    #[arg(short = 'r', long = "root", value_name = "ROOT")]
    pub root: Option<String>,

    /// Format output with indentation and newlines
    #[arg(long = "pretty")]
    pub pretty: bool,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Generate(args) => run_generate(args),
        Commands::Build(args) => run_build(args),
        Commands::Validate(args) => run_validate(args),
        Commands::Transcode(args) => run_transcode(args),
        Commands::Completions { shell } => {
            completions::print_script(shell);
            Ok(())
        }
        Commands::Complete { kind, words } => {
            for value in completions::candidates(&kind, &words) {
                println!("{value}");
            }
            Ok(())
        }
    };

    if let Err(err) = result {
        if err
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::InvalidInput)
        {
            Cli::command()
                .error(clap::error::ErrorKind::InvalidValue, err.to_string())
                .exit();
        }
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run_generate(args: GenerateArgs) -> Result<(), Box<dyn std::error::Error>> {
    // If no schemas are passed directly, check for polyxml.toml config
    if args.schemas.is_empty() {
        if !args.lang.is_empty()
            || args.backend.is_some()
            || args.style.is_some()
            || !args.features.is_empty()
            || args.package.is_some()
            || args.mode.is_some()
            || args.zero_copy.is_some()
            || args.codecs.is_some()
            || args.out.is_some()
            || args.custom_header.is_some()
            || args.strict_facets
        {
            return Err("Generation options require explicit schema paths. For manifest builds, set target options in polyxml.toml.".into());
        }
        let config_path = args.config.unwrap_or_else(|| PathBuf::from("polyxml.toml"));
        if config_path.exists() {
            return run_build(BuildArgs {
                config: config_path,
                dry_run: args.dry_run,
                format: args.format,
            });
        } else {
            eprintln!("No schema files specified and polyxml.toml not found.");
            eprintln!("Run 'polyxml generate --help' for usage.");
            process::exit(1);
        }
    }

    let languages = if args.lang.is_empty() {
        vec!["python".to_string()]
    } else {
        args.lang.clone()
    };

    let emit_opts = TargetEmitOptions {
        backend: args.backend.as_deref(),
        features: &args.features,
        slots: None,
        kw_only: None,
        package: args.package.as_deref(),
        mode: args.mode.as_deref(),
        zero_copy: args.zero_copy,
        codecs: args.codecs,
        style: args.style.as_deref(),
        builder: None,
        codec: None,
        rkyv: None,
        phf: None,
        validation: None,
        custom_header: args.custom_header.as_deref(),
    };
    let resolved_options = languages
        .iter()
        .map(|lang| emit_opts.resolve(lang))
        .collect::<std::io::Result<Vec<_>>>()?;

    let mut parser = XsdParser::new();
    let mut compiled_schemas = Vec::new();

    for schema_path in &args.schemas {
        if !schema_path.exists() {
            return Err(format!("Schema file not found: {}", schema_path.display()).into());
        }

        println!("Parsing schema: {}", schema_path.display());
        let ir = parser.parse_file(schema_path)?;
        report_schema_ir(&ir);
        compiled_schemas.push((schema_path.clone(), ir));
    }

    if args.dry_run {
        println!("\nDry run completed successfully. No files written.");
        return Ok(());
    }

    let base_out = args.out.unwrap_or_else(|| PathBuf::from("generated"));

    for (lang, emit_opts) in languages.iter().zip(resolved_options) {
        let lang_out = if languages.len() > 1 {
            base_out.join(lang)
        } else {
            base_out.clone()
        };

        fs::create_dir_all(&lang_out)?;

        for (schema_path, ir) in &compiled_schemas {
            emit_target_code(lang, emit_opts, &lang_out, schema_path, ir)?;
        }

        if args.format {
            run_language_formatter(lang, &lang_out);
        }
    }

    println!("Code generation complete.");
    Ok(())
}

fn run_build(args: BuildArgs) -> Result<(), Box<dyn std::error::Error>> {
    if !args.config.exists() {
        return Err(format!("Manifest not found: {}", args.config.display()).into());
    }

    println!("Loading manifest: {}", args.config.display());
    let manifest = WorkspaceManifest::from_file(&args.config)?;
    let base_dir = args.config.parent().unwrap_or_else(|| Path::new("."));

    let targets = manifest.resolved_targets();
    for target in &targets {
        target_options(target).resolve(&target.target)?;
    }

    let schema_files = manifest.expand_schemas(base_dir)?;
    if schema_files.is_empty() {
        println!("No schema files matched workspace schema patterns.");
        return Ok(());
    }

    let mut parser = XsdParser::new();
    let mut compiled_schemas = Vec::new();

    for schema_path in &schema_files {
        println!("Compiling schema: {}", schema_path.display());
        let ir = parser.parse_file(schema_path)?;
        report_schema_ir(&ir);
        compiled_schemas.push((schema_path.clone(), ir));
    }

    if targets.is_empty() {
        println!("No generation targets configured in manifest.");
        return Ok(());
    }

    if args.dry_run {
        println!("\nDry run completed. Targets configured: {}", targets.len());
        for target in &targets {
            println!(" - Target: {} -> {}", target.target, target.output);
        }
        return Ok(());
    }

    let output_base = manifest
        .workspace
        .as_ref()
        .and_then(|w| w.output_base_dir.as_ref())
        .map(|dir| {
            let p = Path::new(dir);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                base_dir.join(p)
            }
        })
        .unwrap_or_else(|| base_dir.to_path_buf());

    for target in &targets {
        let target_dir = output_base.join(&target.output);
        fs::create_dir_all(&target_dir)?;
        println!(
            "Emitting target [{}] into {}",
            target.target,
            target_dir.display()
        );

        let emit_opts = target_options(target).resolve(&target.target)?;

        for (schema_path, ir) in &compiled_schemas {
            emit_target_code(&target.target, emit_opts, &target_dir, schema_path, ir)?;
        }

        if args.format {
            run_language_formatter(&target.target, &target_dir);
        }
    }

    println!("Build finished successfully.");
    Ok(())
}

fn run_validate(args: ValidateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = XsdParser::new();
    let mut total_types = 0;
    let mut total_elements = 0;

    for schema_path in &args.schemas {
        if !schema_path.exists() {
            return Err(format!("Schema file not found: {}", schema_path.display()).into());
        }

        let ir = parser.parse_file(schema_path)?;
        println!("✓ Valid schema: {}", schema_path.display());
        if let Some(ref ns) = ir.target_namespace {
            println!("  targetNamespace: {}", ns);
        }
        println!(
            "  Components: {} types, {} root elements",
            ir.types.len(),
            ir.elements.len()
        );

        total_types += ir.types.len();
        total_elements += ir.elements.len();
    }

    println!(
        "\nAll schemas valid (Total: {} types, {} elements).",
        total_types, total_elements
    );
    Ok(())
}

fn report_schema_ir(ir: &SchemaIR) {
    let mut structs = 0;
    let mut enums = 0;
    let mut unions = 0;
    let mut simples = 0;
    let mut cycle_cuts = 0;

    for type_def in ir.types.values() {
        match type_def {
            TypeDef::Struct(s) => {
                structs += 1;
                for f in &s.fields {
                    if f.is_cycle_cut {
                        cycle_cuts += 1;
                    }
                }
            }
            TypeDef::Enum(_) => enums += 1,
            TypeDef::Union(_) => unions += 1,
            TypeDef::Simple(_) => simples += 1,
        }
    }

    if let Some(ref ns) = ir.target_namespace {
        println!("  Namespace: {}", ns);
    }
    println!(
        "  Types: {} total ({} structs, {} enums, {} unions, {} simple restrictions)",
        ir.types.len(),
        structs,
        enums,
        unions,
        simples
    );
    println!("  Root elements: {}", ir.elements.len());
    if cycle_cuts > 0 {
        println!("  Tarjan SCC: Boxed {} recursive cut points", cycle_cuts);
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TargetEmitOptions<'a> {
    pub backend: Option<&'a str>,
    pub features: &'a [String],
    pub slots: Option<bool>,
    pub kw_only: Option<bool>,
    pub package: Option<&'a str>,
    pub mode: Option<&'a str>,
    pub zero_copy: Option<bool>,
    pub codecs: Option<bool>,
    pub style: Option<&'a str>,
    pub builder: Option<bool>,
    pub codec: Option<&'a str>,
    pub rkyv: Option<bool>,
    pub phf: Option<bool>,
    pub validation: Option<bool>,
    pub custom_header: Option<&'a str>,
}

fn emit_target_code(
    lang: &str,
    opts: TargetEmitOptions<'_>,
    out_dir: &Path,
    schema_path: &Path,
    ir: &SchemaIR,
) -> std::io::Result<()> {
    let use_records = !matches!(opts.style, Some("pojo" | "class" | "dataclass"));
    let direct_codec = opts.codec == Some("direct");
    let language = lang.to_lowercase();
    match language.as_str() {
        "python" | "py" => {
            let py_backend = opts
                .backend
                .and_then(PythonBackend::from_str_loose)
                .unwrap_or(PythonBackend::Dataclass);

            if py_backend == PythonBackend::Aot {
                let file_stem = schema_path
                    .file_stem()
                    .map(|s| heck::AsSnakeCase(s.to_string_lossy().as_ref()).to_string())
                    .unwrap_or_else(|| "models".into());
                let module_name = opts
                    .package
                    .map(|p| heck::AsSnakeCase(p).to_string())
                    .unwrap_or(file_stem);

                let aot_opts = PythonAotOptions {
                    module_name: module_name.clone(),
                    custom_header: opts.custom_header.map(|s| s.to_string()),
                };
                let aot_codegen = PythonAotCodegen::new(aot_opts);
                let aot_crate = aot_codegen.generate_crate(ir);

                fs::write(out_dir.join("Cargo.toml"), aot_crate.cargo_toml)?;
                fs::write(out_dir.join("pyproject.toml"), aot_crate.pyproject_toml)?;
                fs::write(out_dir.join("README.md"), aot_crate.readme)?;

                let src_dir = out_dir.join("src");
                fs::create_dir_all(&src_dir)?;
                fs::write(src_dir.join("lib.rs"), aot_crate.lib_rs)?;

                fs::write(
                    out_dir.join(format!("{}.pyi", module_name)),
                    aot_crate.pyi_stub,
                )?;
                fs::write(out_dir.join("py.typed"), "")?;
                return Ok(());
            }

            let options = PythonOptions {
                backend: py_backend,
                slots: opts.slots.unwrap_or(true),
                kw_only: opts.kw_only.unwrap_or(true),
                pep695_aliases: true,
                emit_meta: true,
                emit_root_aliases: true,
                emit_codecs: opts.codecs.unwrap_or(true),
                emit_json_metadata: true,
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = PythonCodegen::new(options);
            let code = codegen.generate_module(ir);

            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "models".into());

            let file_path = out_dir.join(format!("{}.py", file_stem));
            fs::write(file_path, code)?;

            let init_path = out_dir.join("__init__.py");
            if !init_path.exists() {
                let _ = fs::write(&init_path, "# Package generated by PolyXML\n");
            }
            Ok(())
        }
        "rust" | "rs" => {
            let options = RustOptions {
                zero_copy: opts.zero_copy.unwrap_or(true),
                derive_serde: true,
                derive_default: true,
                emit_polyxml_attrs: false,
                emit_root_aliases: true,
                emit_codecs: opts.codecs.unwrap_or(true),
                emit_rkyv: opts.rkyv.unwrap_or(false),
                phf: opts.phf.unwrap_or(false),
                pyo3: false,
                pyo3_module_name: None,
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = RustCodegen::new(options);
            let code = codegen.generate_module(ir);

            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "models".into());

            let file_path = out_dir.join(format!("{}.rs", file_stem));
            fs::write(file_path, code)?;

            let mod_path = out_dir.join("mod.rs");
            if !mod_path.exists() {
                let _ = fs::write(
                    &mod_path,
                    format!("pub mod {};\npub use {}::*;\n", file_stem, file_stem),
                );
            }
            Ok(())
        }
        "ts" | "typescript" => {
            let ts_backend = opts
                .backend
                .and_then(TypeScriptBackend::from_str_loose)
                .unwrap_or(TypeScriptBackend::None);

            let options = TypeScriptOptions {
                backend: ts_backend,
                // Backend selection controls Zod emission.
                emit_zod: false,
                use_interface: true,
                readonly_fields: false,
                emit_root_aliases: true,
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = TypeScriptCodegen::new(options);
            let code = codegen.generate_module(ir);

            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "models".into());

            let file_path = out_dir.join(format!("{}.ts", file_stem));
            fs::write(file_path, code)?;

            let index_path = out_dir.join("index.ts");
            if !index_path.exists() {
                let _ = fs::write(&index_path, format!("export * from \"./{}\";\n", file_stem));
            }
            Ok(())
        }
        "java" => {
            let pkg = opts.package.unwrap_or("generated.models").to_string();
            let java_backend = opts
                .backend
                .and_then(JavaBackend::from_str_loose)
                .unwrap_or(JavaBackend::Standard);

            let options = JavaOptions {
                package_name: pkg,
                backend: java_backend,
                use_records,
                emit_builder: opts.builder.unwrap_or(false),
                emit_direct_codec: direct_codec,
                validate_facets: true,
                bean_validation: opts.validation.unwrap_or(false),
                emit_root_aliases: true,
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = JavaCodegen::new(options);
            if direct_codec {
                codegen.validate_direct_codecs(ir).map_err(|message| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
                })?;
            }
            let files = codegen.generate_files(ir);

            for (filename, code) in files {
                let file_path = out_dir.join(filename);
                fs::write(file_path, code)?;
            }
            Ok(())
        }
        "cpp" | "c++" => {
            let ns = opts.package.unwrap_or("polyxml::generated");
            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "models".into());

            let cpp_mode = opts
                .mode
                .and_then(CppMode::from_str_loose)
                .unwrap_or(CppMode::HeaderOnly);

            let cpp_backend = opts
                .backend
                .and_then(CppBackend::from_str_loose)
                .unwrap_or(CppBackend::Standard);

            let options = CppOptions {
                namespace: ns.to_string(),
                mode: cpp_mode,
                backend: cpp_backend,
                standard: "c++20".to_string(),
                emit_equality_operators: true,
                emit_enum_converters: true,
                validate_facets: true,
                emit_root_aliases: true,
                emit_cmake: false,
                emit_meson: false,
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = CppCodegen::new(options);
            let files = codegen.generate_files(ir, &file_stem);

            for (filename, code) in files {
                let file_path = out_dir.join(filename);
                fs::write(file_path, code)?;
            }
            Ok(())
        }
        "go" => {
            let pkg = opts.package.unwrap_or("models");
            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "models".into());

            let go_backend = opts
                .backend
                .and_then(GoBackend::from_str_loose)
                .unwrap_or(GoBackend::Standard);

            let options = GoOptions {
                backend: go_backend,
                package_name: pkg.to_string(),
                emit_xml_tags: true,
                emit_json_tags: true,
                validate_choice_exclusivity: true,
                validate_facets: true,
                emit_root_aliases: true,
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = GoCodegen::new(options);
            let files = codegen.generate_files(ir, &file_stem);

            for (filename, code) in files {
                let file_path = out_dir.join(filename);
                fs::write(file_path, code)?;
            }
            Ok(())
        }
        "csharp" | "c#" | "cs" => {
            let ns = opts.package.unwrap_or("Generated");
            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "Models".into());

            let record_kind = if matches!(opts.style, Some("record-struct")) {
                CSharpRecordKind::Struct
            } else {
                CSharpRecordKind::Class
            };

            let options = CSharpOptions {
                namespace: ns.to_string(),
                emit_xml_attributes: true,
                emit_json_attributes: true,
                emit_validation: true,
                record_kind,
                use_records,
                use_file_scoped_namespaces: true,
                emit_root_records: true,
                emit_source_gen: opts.backend == Some("source-gen"),
                source_gen_context_name: format!("{}JsonContext", AsPascalCase(&file_stem)),
                custom_header: opts.custom_header.map(|s| s.to_string()),
            };

            let codegen = CSharpCodegen::new(options);
            let files = codegen.generate_files(ir, &file_stem);

            for (filename, code) in files {
                let file_path = out_dir.join(filename);
                fs::write(file_path, code)?;
            }
            Ok(())
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Unknown target: {lang}"),
        )),
    }
}

fn run_language_formatter(lang: &str, dir: &Path) {
    let dir_str = match dir.to_str() {
        Some(s) => s,
        None => return,
    };

    match lang.to_lowercase().as_str() {
        "python" | "py" => {
            let _ = Command::new("ruff").args(["format", dir_str]).status();
            let _ = Command::new("ruff")
                .args(["check", "--fix", "--silent", dir_str])
                .status();
        }
        "rust" | "rs" => {
            let _ = Command::new("cargo").args(["fmt"]).status();
        }
        "go" => {
            let _ = Command::new("gofmt").args(["-w", dir_str]).status();
        }
        "cpp" | "c++" => {
            if let Ok(entries) = fs::read_dir(dir) {
                let cpp_files: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.extension()
                            .map(|ext| ext == "hpp" || ext == "h" || ext == "cpp" || ext == "cppm")
                            .unwrap_or(false)
                    })
                    .collect();
                if !cpp_files.is_empty() {
                    let mut cmd = Command::new("clang-format");
                    cmd.arg("-i");
                    for f in cpp_files {
                        cmd.arg(f);
                    }
                    let _ = cmd.status();
                }
            }
        }
        "ts" | "typescript" => {
            let _ = Command::new("npx")
                .args(["prettier", "--write", dir_str])
                .status();
        }
        "java" => {
            let _ = Command::new("google-java-format")
                .args(["-i", dir_str])
                .status();
        }
        "csharp" | "c#" | "cs" => {
            let status = Command::new("csharpier").args([dir_str]).status();
            if status.is_err() || !status.as_ref().map(|s| s.success()).unwrap_or(false) {
                let _ = Command::new("dotnet")
                    .env("DOTNET_NOLOGO", "1")
                    .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
                    .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
                    .args(["format", "whitespace", dir_str])
                    .status();
            }
        }
        _ => {}
    }
}

fn run_transcode(args: TranscodeArgs) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Read, Write};

    // 1. Read input bytes
    let input_bytes = if let Some(ref path) = args.input {
        if path.as_os_str() == "-" {
            let mut buf = Vec::new();
            std::io::stdin().read_to_end(&mut buf)?;
            buf
        } else {
            fs::read(path)?
        }
    } else {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        buf
    };

    // 2. Determine from and to formats
    let from_format = if let Some(ref f) = args.from {
        f.to_lowercase()
    } else if let Some(ref path) = args.input {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "xml" => "xml".to_string(),
            "json" => "json".to_string(),
            _ => {
                if input_bytes.iter().find(|&&b| !b.is_ascii_whitespace()) == Some(&b'<') {
                    "xml".to_string()
                } else {
                    "json".to_string()
                }
            }
        }
    } else if input_bytes.iter().find(|&&b| !b.is_ascii_whitespace()) == Some(&b'<') {
        "xml".to_string()
    } else {
        "json".to_string()
    };

    let to_format = if let Some(ref t) = args.to {
        t.to_lowercase()
    } else if let Some(ref path) = args.output {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "xml" => "xml".to_string(),
            "json" => "json".to_string(),
            _ => {
                if from_format == "xml" {
                    "json".to_string()
                } else {
                    "xml".to_string()
                }
            }
        }
    } else if from_format == "xml" {
        "json".to_string()
    } else {
        "xml".to_string()
    };

    // 3. Load optional ModelSchema
    let model_schema = if let Some(ref schema_path) = args.schema {
        let mut parser = XsdParser::new();
        let ir = parser.parse_file(schema_path)?;
        Some(polyxml::ModelSchema::from_ir(&ir, args.root.as_deref())?)
    } else {
        None
    };

    let indent = if args.pretty { Some(2) } else { None };

    // 4. Perform transcoding
    let output_bytes = match (from_format.as_str(), to_format.as_str()) {
        ("xml", "json") => polyxml::xml_to_json(&input_bytes, model_schema, indent, true)?,
        ("json", "xml") => polyxml::json_to_xml(
            &input_bytes,
            model_schema,
            args.root.as_deref(),
            indent,
            None,
            None,
        )?,
        _ => {
            return Err(format!(
                "Unsupported transcoding direction from '{}' to '{}'",
                from_format, to_format
            )
            .into())
        }
    };

    // 5. Write output bytes
    if let Some(ref path) = args.output {
        if path.as_os_str() == "-" {
            std::io::stdout().write_all(&output_bytes)?;
        } else {
            fs::write(path, output_bytes)?;
        }
    } else {
        std::io::stdout().write_all(&output_bytes)?;
    }

    Ok(())
}

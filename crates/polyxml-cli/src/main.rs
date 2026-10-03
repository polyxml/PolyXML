mod completions;
pub mod config;
mod options;
use options::target_options;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use clap::{Args, CommandFactory, Parser, Subcommand};
use config::{TargetConfig, WorkspaceManifest};
use heck::AsPascalCase;
use polyxml::codegen::build_type_name_map;
use polyxml::codegen::cpp::{CppBackend, CppCodegen, CppMode, CppOptions};
use polyxml::codegen::csharp::{CSharpCodegen, CSharpOptions, CSharpRecordKind};
use polyxml::codegen::go::{GoBackend, GoCodegen, GoOptions};
use polyxml::codegen::java::{JavaBackend, JavaCodegen, JavaOptions};
use polyxml::codegen::python::{
    PythonAotCodegen, PythonAotOptions, PythonBackend, PythonCodegen, PythonOptions,
};
use polyxml::codegen::rust::{RustCodegen, RustOptions};
use polyxml::codegen::typescript::{TypeScriptBackend, TypeScriptCodegen, TypeScriptOptions};
use polyxml::ir::{QName, SchemaIR, TypeDef, TypeRef};
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

    /// Global element(s) to generate, including reachable and polymorphic types
    #[arg(long = "root-element", value_name = "ELEMENT")]
    pub root_elements: Vec<String>,

    /// Target language(s) to emit (python, rust, cpp, java, ts, go, csharp)
    #[arg(short = 'l', long = "lang", value_name = "LANG")]
    pub lang: Vec<String>,

    /// Target backend (e.g. dataclass/pydantic for Python; standard/jackson/jackson3 for Java)
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

    /// Split oversized modules into bounded topological chunks (optional setting, default: false)
    #[arg(long = "split-units", default_missing_value = "true", num_args = 0..=1)]
    pub split_units: Option<bool>,

    /// Target maximum types per compilation unit chunk (default: 250)
    #[arg(long = "chunk-size", value_name = "N")]
    pub chunk_size: Option<usize>,
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
            || !args.root_elements.is_empty()
            || args.split_units.is_some()
            || args.chunk_size.is_some()
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
        split_units: args.split_units,
        chunk_size: args.chunk_size,
    };
    let resolved_options = languages
        .iter()
        .map(|lang| emit_opts.resolve(lang))
        .collect::<std::io::Result<Vec<_>>>()?;

    let mut parser = XsdParser::new();
    let mut parsed_schemas = Vec::new();

    for schema_path in &args.schemas {
        if !schema_path.exists() {
            return Err(format!("Schema file not found: {}", schema_path.display()).into());
        }

        println!("Parsing schema: {}", schema_path.display());
        parsed_schemas.push((schema_path.clone(), parser.parse_file(schema_path)?));
    }
    let compiled_schemas = select_compiled_schemas(parsed_schemas, &args.root_elements)?;

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

fn select_compiled_schemas(
    parsed: Vec<(PathBuf, SchemaIR)>,
    roots: &[String],
) -> Result<Vec<(PathBuf, SchemaIR)>, Box<dyn std::error::Error>> {
    if roots.is_empty() {
        for (_, ir) in &parsed {
            report_schema_ir(ir);
        }
        return Ok(parsed);
    }

    // Resolve names across the complete input set so a root can belong to any
    // one schema, while an ambiguous local name still fails consistently.
    let mut root_index = SchemaIR::new();
    for (_, ir) in &parsed {
        root_index.elements.extend(ir.elements.clone());
    }
    let requested = root_index.select_root_elements(roots)?;
    let mut selected = Vec::new();
    for (path, ir) in parsed {
        let local_roots: Vec<String> = requested
            .elements
            .keys()
            .filter(|qname| ir.elements.contains_key(*qname))
            .map(|qname| {
                format!(
                    "{{{}}}{}",
                    qname.namespace.as_deref().unwrap_or(""),
                    qname.local
                )
            })
            .collect();
        if local_roots.is_empty() {
            continue;
        }
        let filtered = ir.select_root_elements(&local_roots)?;
        println!(
            "Retained {} global elements and {} of {} types in {}",
            filtered.elements.len(),
            filtered.types.len(),
            ir.types.len(),
            path.display()
        );
        report_schema_ir(&filtered);
        selected.push((path, filtered));
    }
    Ok(selected)
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

    if !manifest.modules.is_empty() {
        return run_module_build(&manifest, base_dir, &targets, args.dry_run, args.format);
    }

    let schema_files = manifest.expand_schemas(base_dir)?;
    if schema_files.is_empty() {
        println!("No schema files matched workspace schema patterns.");
        return Ok(());
    }

    let mut parser = XsdParser::new();
    let mut parsed_schemas = Vec::new();

    for schema_path in &schema_files {
        println!("Compiling schema: {}", schema_path.display());
        parsed_schemas.push((schema_path.clone(), parser.parse_file(schema_path)?));
    }
    let roots = manifest
        .workspace
        .as_ref()
        .map_or(&[][..], |ws| ws.root_elements.as_slice());
    let compiled_schemas = select_compiled_schemas(parsed_schemas, roots)?;

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

fn visible_modules_for(module: &str, manifest: &WorkspaceManifest) -> BTreeSet<String> {
    let mut visible = BTreeSet::new();
    visible.insert(module.to_string());
    let mut queue = vec![module.to_string()];
    while let Some(current) = queue.pop() {
        if let Some(config) = manifest.modules.get(&current) {
            for dep in &config.depends_on {
                if visible.insert(dep.clone()) {
                    queue.push(dep.clone());
                }
            }
        }
    }
    visible
}

fn run_module_build(
    manifest: &WorkspaceManifest,
    base_dir: &Path,
    targets: &[TargetConfig],
    dry_run: bool,
    format: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let order = manifest.module_order()?;
    let mut global = SchemaIR::new();
    let mut owners = BTreeMap::<QName, String>::new();
    let mut module_types = BTreeMap::<String, BTreeSet<QName>>::new();
    let mut module_elements = BTreeMap::new();
    let mut module_namespaces = BTreeMap::new();

    for name in &order {
        let paths = manifest.expand_module_schemas(name, base_dir)?;
        if paths.is_empty() {
            return Err(format!("Module '{name}' has no matching schema files").into());
        }
        let mut owned = BTreeSet::new();
        let mut elements = BTreeMap::new();
        for path in paths {
            println!("Compiling module [{name}]: {}", path.display());
            let mut parser = XsdParser::new();
            let ir = parser.parse_file(&path)?;
            module_namespaces
                .entry(name.clone())
                .or_insert_with(|| ir.target_namespace.clone());
            for (qname, def) in &ir.types {
                if qname.namespace == ir.target_namespace {
                    if let Some(existing) = owners.insert(qname.clone(), name.clone()) {
                        if existing != *name {
                            return Err(format!(
                                "Type {qname} is owned by both '{existing}' and '{name}'"
                            )
                            .into());
                        }
                    }
                    owned.insert(qname.clone());
                }
                global
                    .types
                    .entry(qname.clone())
                    .or_insert_with(|| def.clone());
            }
            for (qname, def) in &ir.elements {
                if qname.namespace == ir.target_namespace {
                    elements.insert(qname.clone(), def.clone());
                    global
                        .elements
                        .entry(qname.clone())
                        .or_insert_with(|| def.clone());
                }
            }
            global.namespaces.extend(ir.namespaces);
            for (head, members) in ir.substitution_groups {
                global
                    .substitution_groups
                    .entry(head)
                    .or_default()
                    .extend(members);
            }
        }
        module_types.insert(name.clone(), owned);
        module_elements.insert(name.clone(), elements);
    }

    let mut requested_roots = Vec::new();
    for name in &order {
        for root in &manifest.modules[name].root_elements {
            if root.starts_with('{') {
                requested_roots.push(root.clone());
                continue;
            }
            let matches: Vec<_> = module_elements[name]
                .keys()
                .filter(|qname| qname.local == *root)
                .collect();
            match matches.as_slice() {
                [] => return Err(format!("Root element '{root}' was not found in module '{name}'").into()),
                [qname] => requested_roots.push(format!(
                    "{{{}}}{}",
                    qname.namespace.as_deref().unwrap_or(""),
                    qname.local
                )),
                _ => return Err(format!("Root element '{root}' is ambiguous in module '{name}'; use {{namespace}}local-name").into()),
            }
        }
    }
    if !requested_roots.is_empty() {
        let original_count = global.types.len();
        global = global.select_root_elements(&requested_roots)?;
        for types in module_types.values_mut() {
            types.retain(|qname| global.types.contains_key(qname));
        }
        for elements in module_elements.values_mut() {
            elements.retain(|qname, _| global.elements.contains_key(qname));
        }
        println!(
            "Retained {} global elements and {} of {} types",
            global.elements.len(),
            global.types.len(),
            original_count
        );
    }

    retain_owned_type_closure(&mut global, &owners);

    for qname in global.types.keys() {
        if qname
            .namespace
            .as_deref()
            .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
        {
            continue;
        }
        if !owners.contains_key(qname) {
            return Err(
                format!("Imported type {qname} has no owning [modules.<name>] entry").into(),
            );
        }
    }

    if dry_run {
        for name in &order {
            println!("Module [{name}]: {} owned types", module_types[name].len());
        }
        return Ok(());
    }

    let output_base = manifest
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.output_base_dir.as_ref())
        .map(|dir| {
            if Path::new(dir).is_absolute() {
                PathBuf::from(dir)
            } else {
                base_dir.join(dir)
            }
        })
        .unwrap_or_else(|| base_dir.to_path_buf());

    for target in targets {
        let target_root = output_base.join(&target.output);
        fs::create_dir_all(&target_root)?;
        let language = target.target.to_ascii_lowercase();
        if language == "go" {
            let go_module = manifest
                .workspace
                .as_ref()
                .and_then(|workspace| workspace.go_module.as_deref())
                .unwrap_or("polyxml/generated");
            let go_mod = target_root.join("go.mod");
            if !go_mod.exists() {
                fs::write(&go_mod, format!("module {go_module}\n\ngo 1.22\n"))?;
            }
        }
        if language == "python" || language == "py" {
            fs::write(
                target_root.join("__init__.py"),
                "# Generated workspace modules\n",
            )?;
        }

        // Precompute canonical exported type names for all types in their owning modules.
        let mut owner_exported_names: BTreeMap<QName, String> = BTreeMap::new();
        for module_name in &order {
            let visible = visible_modules_for(module_name, manifest);
            let mut ir = global.clone();
            ir.target_namespace = module_namespaces[module_name].clone();
            ir.elements = module_elements[module_name].clone();
            ir.types.retain(|qname, _| {
                qname
                    .namespace
                    .as_deref()
                    .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
                    || owners
                        .get(qname)
                        .is_some_and(|owner| visible.contains(owner))
            });
            ir.external_types = owners
                .iter()
                .filter(|(qname, owner)| {
                    *owner != module_name
                        && visible.contains(*owner)
                        && !qname
                            .namespace
                            .as_deref()
                            .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
                })
                .map(|(qname, owner)| (qname.clone(), owner.clone()))
                .collect();
            let names = match language.as_str() {
                "go" => build_type_name_map(&ir, polyxml::codegen::go::to_go_type_name),
                "java" => build_type_name_map(&ir, polyxml::codegen::java::to_java_type_name),
                "csharp" | "cs" | "c#" => {
                    build_type_name_map(&ir, polyxml::codegen::csharp::to_csharp_type_name)
                }
                "cpp" | "c++" => build_type_name_map(&ir, polyxml::codegen::cpp::to_cpp_type_name),
                "typescript" | "ts" => {
                    build_type_name_map(&ir, polyxml::codegen::typescript::to_ts_type_name)
                }
                _ => build_type_name_map(&ir, |n| AsPascalCase(n).to_string()),
            };
            for (qname, owner) in &owners {
                if owner == module_name {
                    if let Some(n) = names.get(qname) {
                        owner_exported_names.insert(qname.clone(), n.clone());
                    }
                }
            }
        }

        let mut rust_root = String::new();
        for name in &order {
            let visible = visible_modules_for(name, manifest);
            let mut ir = global.clone();
            ir.target_namespace = module_namespaces[name].clone();
            ir.elements = module_elements[name].clone();
            ir.types.retain(|qname, _| {
                qname
                    .namespace
                    .as_deref()
                    .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
                    || owners
                        .get(qname)
                        .is_some_and(|owner| visible.contains(owner))
            });
            ir.external_types = owners
                .iter()
                .filter(|(qname, owner)| {
                    *owner != name
                        && visible.contains(*owner)
                        && !qname
                            .namespace
                            .as_deref()
                            .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
                })
                .map(|(qname, owner)| (qname.clone(), owner.clone()))
                .collect();

            let module_dir = target_root.join(name);
            fs::create_dir_all(&module_dir)?;
            let mut module_target = target.clone();
            let package_root = target.package.as_deref().or(target.namespace.as_deref());
            module_target.package = match language.as_str() {
                "go" => Some(name.clone()),
                "java" => Some(format!(
                    "{}.{}",
                    package_root.unwrap_or("generated.models"),
                    name
                )),
                "csharp" | "c#" | "cs" => {
                    Some(format!("{}.{}", package_root.unwrap_or("Generated"), name))
                }
                "cpp" | "c++" => Some(format!(
                    "{}::{}",
                    package_root.unwrap_or("polyxml::generated"),
                    name
                )),
                _ => target.package.clone(),
            };
            module_target.namespace = None;
            let opts = target_options(&module_target).resolve(&module_target.target)?;
            let fake_schema = PathBuf::from(format!("{name}.xsd"));
            emit_target_code(&language, opts, &module_dir, &fake_schema, &ir)?;
            let references = referenced_external_types(&ir, &language);
            inject_module_imports(
                &language,
                &module_dir,
                name,
                &references,
                &owners,
                &owner_exported_names,
                &ir,
                target,
                manifest,
                &target_root,
            )?;
            if format {
                run_language_formatter(&language, &module_dir);
            }
            if language == "rust" || language == "rs" {
                let safe_mod = polyxml::codegen::rust::sanitize_rust_module_name(name);
                if safe_mod == name.as_str() {
                    rust_root.push_str(&format!("pub mod {name};\n"));
                } else {
                    rust_root.push_str(&format!(
                        "#[path = \"{name}/mod.rs\"]\npub mod {safe_mod};\n"
                    ));
                }
            }
        }
        if !rust_root.is_empty() {
            fs::write(target_root.join("mod.rs"), rust_root)?;
        }
    }
    println!("Module build finished successfully.");
    Ok(())
}

/// Imported parsing frames can synthesize context-specific helpers that are
/// unused after the canonical owning module's declaration wins the merge.
fn retain_owned_type_closure(ir: &mut SchemaIR, owners: &BTreeMap<QName, String>) {
    fn collect(reference: &TypeRef, pending: &mut Vec<QName>) {
        match reference {
            TypeRef::Named(name) => pending.push(name.clone()),
            TypeRef::Boxed(inner) | TypeRef::List(inner) => collect(inner, pending),
            TypeRef::Primitive(_) => {}
        }
    }
    let mut pending = owners.keys().cloned().collect::<Vec<_>>();
    for element in ir.elements.values() {
        collect(&element.type_ref, &mut pending);
    }
    let mut reachable = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        match ir.types.get(&name) {
            Some(TypeDef::Struct(s)) => {
                if let Some(base) = &s.base_type {
                    pending.push(base.clone());
                }
                for field in &s.fields {
                    collect(&field.type_ref, &mut pending);
                }
            }
            Some(TypeDef::Union(u)) => {
                for branch in &u.branches {
                    collect(&branch.type_ref, &mut pending);
                }
            }
            Some(TypeDef::Simple(s)) => collect(&s.base_type, &mut pending),
            _ => {}
        }
    }
    ir.types.retain(|name, _| {
        reachable.contains(name)
            || name
                .namespace
                .as_deref()
                .is_some_and(|ns| ns.starts_with("urn:polyxml:builtins"))
    });
}

fn referenced_external_types(ir: &SchemaIR, language: &str) -> BTreeSet<QName> {
    fn collect(reference: &TypeRef, names: &mut BTreeSet<QName>) {
        match reference {
            TypeRef::Named(qname) => {
                names.insert(qname.clone());
            }
            TypeRef::Boxed(inner) | TypeRef::List(inner) => collect(inner, names),
            TypeRef::Primitive(_) => {}
        }
    }
    let mut names = BTreeSet::new();
    fn collect_base_fields(
        qname: &QName,
        ir: &SchemaIR,
        names: &mut BTreeSet<QName>,
        seen: &mut BTreeSet<QName>,
    ) {
        if !seen.insert(qname.clone()) {
            return;
        }
        let Some(TypeDef::Struct(base)) = ir.types.get(qname) else {
            return;
        };
        if let Some(parent) = &base.base_type {
            collect_base_fields(parent, ir, names, seen);
        }
        for field in &base.fields {
            collect(&field.type_ref, names);
        }
    }
    for def in ir.emitted_types() {
        match def {
            TypeDef::Struct(structure) => {
                if let Some(base) = &structure.base_type {
                    names.insert(base.clone());
                    if language == "java" {
                        collect_base_fields(base, ir, &mut names, &mut BTreeSet::new());
                    }
                }
                for field in &structure.fields {
                    collect(&field.type_ref, &mut names);
                }
            }
            TypeDef::Union(union) => {
                for branch in &union.branches {
                    collect(&branch.type_ref, &mut names);
                }
            }
            TypeDef::Simple(simple) => collect(&simple.base_type, &mut names),
            TypeDef::Enum(_) => {}
        }
    }
    for element in ir.elements.values() {
        collect(&element.type_ref, &mut names);
    }
    names.retain(|name| ir.is_external_type(name));
    names
}

#[derive(Debug)]
struct ModuleImportedType {
    orig_name: String,
    local_name: String,
}

#[allow(clippy::too_many_arguments)]
fn inject_module_imports(
    language: &str,
    module_dir: &Path,
    module_name: &str,
    references: &BTreeSet<QName>,
    owners: &BTreeMap<QName, String>,
    owner_exported_names: &BTreeMap<QName, String>,
    ir: &SchemaIR,
    target: &TargetConfig,
    manifest: &WorkspaceManifest,
    target_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if references.is_empty() {
        return Ok(());
    }
    let names = match language {
        "go" => build_type_name_map(ir, polyxml::codegen::go::to_go_type_name),
        "java" => build_type_name_map(ir, polyxml::codegen::java::to_java_type_name),
        "csharp" | "cs" | "c#" => {
            build_type_name_map(ir, polyxml::codegen::csharp::to_csharp_type_name)
        }
        "cpp" | "c++" => build_type_name_map(ir, polyxml::codegen::cpp::to_cpp_type_name),
        "typescript" | "ts" => {
            build_type_name_map(ir, polyxml::codegen::typescript::to_ts_type_name)
        }
        _ => build_type_name_map(ir, |name| AsPascalCase(name).to_string()),
    };
    let mut by_owner = BTreeMap::<String, Vec<ModuleImportedType>>::new();
    for qname in references {
        let owner = &owners[qname];
        let local_name = names[qname].clone();
        let orig_name = owner_exported_names
            .get(qname)
            .cloned()
            .unwrap_or_else(|| local_name.clone());
        by_owner
            .entry(owner.clone())
            .or_default()
            .push(ModuleImportedType {
                orig_name,
                local_name,
            });
    }
    let package_root = target.package.as_deref().or(target.namespace.as_deref());
    let go_module = manifest
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.go_module.as_deref())
        .unwrap_or("polyxml/generated");
    for entry in fs::read_dir(module_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let expected = match language {
            "python" | "py" => "py",
            "rust" | "rs" => "rs",
            "typescript" | "ts" => "ts",
            "java" => "java",
            "cpp" | "c++" => "hpp",
            "go" => "go",
            "csharp" | "cs" | "c#" => "cs",
            _ => continue,
        };
        if extension != expected
            || path
                .file_stem()
                .is_some_and(|stem| stem == "mod" || stem == "index" || stem == "__init__")
        {
            continue;
        }
        let mut code = fs::read_to_string(&path)?;
        match language {
            "python" | "py" => {
                let imports = by_owner
                    .iter()
                    .map(|(owner, types)| {
                        format!(
                            "from ..{owner}.{owner} import {}\n",
                            types
                                .iter()
                                .map(|t| format!("{} as {}", t.orig_name, t.local_name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .collect::<String>();
                code = code.replacen(
                    "from __future__ import annotations\n",
                    &format!("from __future__ import annotations\n\n{imports}"),
                    1,
                );
            }
            "rust" | "rs" => {
                let imports = by_owner
                    .iter()
                    .map(|(owner, types)| {
                        let items = types
                            .iter()
                            .map(|t| {
                                if t.orig_name == t.local_name {
                                    t.orig_name.clone()
                                } else {
                                    format!("{} as {}", t.orig_name, t.local_name)
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("use super::super::{owner}::{{{items}}};\n")
                    })
                    .collect::<String>();
                if let Some(pos) = code.find("\n\n") {
                    code.insert_str(pos + 2, &imports);
                }
            }
            "typescript" | "ts" => {
                let imports = by_owner
                    .iter()
                    .map(|(owner, types)| {
                        let mut names = types
                            .iter()
                            .map(|t| {
                                if t.orig_name == t.local_name {
                                    format!("type {}", t.orig_name)
                                } else {
                                    format!("type {} as {}", t.orig_name, t.local_name)
                                }
                            })
                            .collect::<Vec<_>>();
                        for t in types {
                            if code.contains(&format!("{}Schema", t.local_name)) {
                                if t.orig_name == t.local_name {
                                    names.push(format!("{}Schema", t.orig_name));
                                } else {
                                    names.push(format!(
                                        "{}Schema as {}Schema",
                                        t.orig_name, t.local_name
                                    ));
                                }
                            }
                        }
                        format!(
                            "import {{ {} }} from \"../{owner}/{owner}\";\n",
                            names.join(", ")
                        )
                    })
                    .collect::<String>();
                code = format!("{imports}{code}");
            }
            "java" => {
                let root = package_root.unwrap_or("generated.models");
                let direct_codec = target
                    .features
                    .iter()
                    .any(|feature| feature == "direct-codec");
                let file_stem = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("");
                for (owner, types) in &by_owner {
                    for imported in types {
                        if imported.local_name != imported.orig_name {
                            let qualified = format!("{root}.{owner}.{}", imported.orig_name);
                            if direct_codec {
                                code = replace_java_identifier(
                                    &code,
                                    &format!("{}Codec", imported.local_name),
                                    &format!("{qualified}Codec"),
                                );
                            }
                            code = replace_java_identifier(&code, &imported.local_name, &qualified);
                        }
                    }
                }
                let imports = by_owner
                    .iter()
                    .flat_map(|(owner, types)| {
                        types
                            .iter()
                            .filter(move |t| {
                                t.local_name == t.orig_name && file_stem != t.orig_name
                            })
                            .flat_map(move |t| {
                                let mut imports =
                                    vec![format!("import {root}.{owner}.{};\n", t.orig_name)];
                                if direct_codec {
                                    imports.push(format!(
                                        "import {root}.{owner}.{}Codec;\n",
                                        t.orig_name
                                    ));
                                }
                                imports
                            })
                    })
                    .collect::<String>();
                if let Some(pos) = code.find(";\n") {
                    code.insert_str(pos + 2, &imports);
                }
            }
            "cpp" | "c++" => {
                let root = package_root.unwrap_or("polyxml::generated");
                let includes = by_owner
                    .keys()
                    .map(|owner| format!("#include \"../{owner}/{owner}.hpp\"\n"))
                    .collect::<String>();
                let aliases = by_owner
                    .iter()
                    .flat_map(|(owner, types)| {
                        types.iter().map(move |t| {
                            if t.orig_name == t.local_name {
                                format!("using {root}::{owner}::{};\n", t.orig_name)
                            } else {
                                format!(
                                    "using {} = {root}::{owner}::{};\n",
                                    t.local_name, t.orig_name
                                )
                            }
                        })
                    })
                    .collect::<String>();
                code = format!("{includes}{code}");
                let ns = format!("namespace {root}::{module_name} {{");
                code = code.replacen(&ns, &format!("{ns}\n{aliases}"), 1);
            }
            "go" => {
                let imports = by_owner
                    .keys()
                    .map(|owner| format!("    {owner} \"{go_module}/{owner}\"\n"))
                    .collect::<String>();
                if let Some(pos) = code.find("import (\n") {
                    code.insert_str(pos + "import (\n".len(), &imports);
                } else if let Some(pos) = code
                    .find("\npackage ")
                    .and_then(|start| code[start + 1..].find('\n').map(|end| start + 1 + end))
                {
                    code.insert_str(pos + 1, &format!("\nimport (\n{imports})\n"));
                }
                let aliases = by_owner
                    .iter()
                    .flat_map(|(owner, types)| {
                        types.iter().map(move |t| {
                            format!("type {} = {owner}.{}\n", t.local_name, t.orig_name)
                        })
                    })
                    .collect::<String>();
                code.push_str(&format!("\n{aliases}"));
            }
            "csharp" | "cs" | "c#" => {
                let root = package_root.unwrap_or("Generated");
                let imports = by_owner
                    .keys()
                    .map(|owner| {
                        format!(
                            "using {};\n",
                            polyxml::codegen::csharp::to_csharp_namespace(&format!(
                                "{root}.{owner}"
                            ))
                        )
                    })
                    .collect::<String>();
                let aliases = by_owner
                    .iter()
                    .flat_map(|(owner, types)| {
                        let owner_ns = polyxml::codegen::csharp::to_csharp_namespace(&format!(
                            "{root}.{owner}"
                        ));
                        types.iter().filter_map(move |t| {
                            if t.orig_name != t.local_name {
                                Some(format!(
                                    "using {} = {owner_ns}.{};\n",
                                    t.local_name, t.orig_name
                                ))
                            } else {
                                None
                            }
                        })
                    })
                    .collect::<String>();
                code = format!("{imports}{aliases}{code}");
            }
            _ => {}
        }
        fs::write(path, code)?;
    }
    let _ = target_root;
    Ok(())
}

fn replace_java_identifier(code: &str, name: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut start = 0;
    for (index, _) in code.match_indices(name) {
        let before = code[..index].chars().next_back();
        let after = code[index + name.len()..].chars().next();
        let ident = |ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$';
        if before.is_some_and(ident) || after.is_some_and(ident) {
            continue;
        }
        out.push_str(&code[start..index]);
        out.push_str(replacement);
        start = index + name.len();
    }
    out.push_str(&code[start..]);
    out
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
    pub split_units: Option<bool>,
    pub chunk_size: Option<usize>,
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
                split_units: opts.split_units,
                chunk_size: opts.chunk_size,
            };

            let codegen = RustCodegen::new(options);

            let file_stem = schema_path
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_else(|| "models".into());

            let files = codegen.generate_files(ir, &file_stem);
            let is_chunked = files.iter().any(|(n, _)| n.starts_with("chunk_"));
            if is_chunked {
                let old_monolithic = out_dir.join(format!("{}.rs", file_stem));
                if old_monolithic.exists() {
                    let _ = fs::remove_file(old_monolithic);
                }
            }
            for (filename, code) in files {
                let file_path = out_dir.join(filename);
                fs::write(file_path, code)?;
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

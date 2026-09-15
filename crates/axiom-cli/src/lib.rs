//! Command-line entry points for the Axiom toolchain.

mod bridge;

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use axiom_agent_pool::{AgentTaskPool, PoolError, PoolVerificationPolicy, SmtResult};
use axiom_db::{AstDatabase, DbError, NodeHash};
use axiom_diagnostics::{diagnose_module, diagnose_source, DiagnosticStatus};
use axiom_ffi_gen::{
    generate_c_abi_rust_glue, generate_c_header_from_source, generate_typescript_from_source,
    FfiError,
};
use axiom_hole_extractor::{extract_holes, HoleTask};
use axiom_parser::{parse_module, ParseError};
use axiom_smt::{find_z3_executable, generate_smtlib2_with_module, resolve_solver_identity};
use axiom_transpiler::{transpile_to_rust, TranspileError};
use axiom_vfs::{
    export_database_module, export_module_to_source, sync_source_to_database, VfsError,
};
use axiom_wit::{generate_wit_from_source, WitError};
use bridge::generate_unity_bridge;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "axiom", version, about = "The Axiom language toolchain")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Parse an Axiom source file and report its unresolved holes.
    Check { file: PathBuf },

    /// Verify contracts and effects with SMT solver and counter-example generator.
    Verify {
        file: PathBuf,

        /// Output results as JSON for machine consumption / multi-agent self-healing.
        #[arg(long)]
        json: bool,

        /// Strict mode: require formal theorem proof (Proved) for all contracted functions (reject Sampled/Skipped).
        #[arg(long)]
        strict: bool,
    },

    /// Export Axiom functions and mathematical contracts to standard SMT-LIB2 (.smt2) format.
    ExportSmt2 {
        file: PathBuf,

        /// Write generated SMT-LIB2 query to a file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Extract implementation-hole tasks as JSON.
    ExtractHoles {
        file: PathBuf,

        /// Write one `<hole_id>.json` task file into this directory.
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },

    /// Transpile a hole-free Axiom source file to Rust.
    EmitRust {
        file: PathBuf,

        /// Write generated Rust to a file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Bypass formal verification gate (DANGEROUS: emits unverified code with warning).
        #[arg(long)]
        unsafe_unverified: bool,
    },

    /// Transpile, compile, and execute a hole-free Axiom program.
    Run {
        file: PathBuf,

        /// Bypass formal verification gate (DANGEROUS: runs unverified code with warning).
        #[arg(long)]
        unsafe_unverified: bool,
    },

    /// Generate a WebAssembly Component Model interface.
    ExportWit {
        file: PathBuf,

        /// Write generated WIT to a file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// WIT package ID. Defaults to the source file name in the `axiom` namespace.
        #[arg(long)]
        package: Option<String>,
    },

    /// Generate TypeScript declarations, C header files, or Rust C-ABI glue.
    ExportFfi {
        file: PathBuf,

        /// Target language: `ts`, `c`, or `rust`.
        #[arg(long)]
        lang: String,

        /// Write generated bindings to a file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Custom header guard name (for C headers).
        #[arg(long)]
        guard: Option<String>,
    },

    /// Scaffold a new Axiom project structure.
    Scaffold {
        /// Name of the new Axiom project.
        name: String,

        /// Project target template: `web`, `native`, or `ffi`.
        #[arg(long, default_value = "web")]
        target: String,

        /// Output directory where project files will be generated.
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },

    /// Generate an engine bridge from an Axiom source module.
    Bridge {
        source: PathBuf,

        /// Engine target. Currently supported: `unity`.
        #[arg(long)]
        target: String,

        /// Directory where generated bridge files will be written.
        #[arg(long)]
        out: PathBuf,

        /// Native library name used by generated P/Invoke declarations.
        #[arg(long, default_value = "axiom")]
        library_name: String,
    },

    /// Store and retrieve content-addressed modules in the AST database.
    Db {
        #[command(subcommand)]
        command: DatabaseCommands,
    },

    /// Manage and coordinate multi-agent hole-filling tasks with SMT verification.
    Pool {
        #[command(subcommand)]
        action: PoolAction,
    },

    /// Display canonical identity, version, and cryptographic hash of the verification solver.
    SolverInfo {
        /// Output solver identity as structured JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum PoolAction {
    /// Show current hole tasks, pending holes, and resolution status in an Axiom file.
    Status {
        file: PathBuf,

        /// Optional path to shared AST database file.
        #[arg(long)]
        db: Option<PathBuf>,

        /// Strict verification policy: require formal SMT proof for contracted functions.
        #[arg(long)]
        strict: bool,

        /// Output status and task envelopes as structured JSON.
        #[arg(long)]
        json: bool,
    },

    /// Submit a candidate expression solution for a specific hole and verify with SMT.
    Submit {
        file: PathBuf,

        /// Hole ID to resolve (e.g. `calc_gem_pickup_range_h0`).
        #[arg(long)]
        hole: String,

        /// Axiom expression string fulfilling the hole, or path to file containing expression.
        #[arg(long)]
        expr: String,

        /// Optional path to shared AST database file.
        #[arg(long)]
        db: Option<PathBuf>,

        /// Strict verification policy: require formal SMT proof.
        #[arg(long)]
        strict: bool,

        /// Output submission result and CAS events as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Assemble verified module from solutions into a target Axiom file or stdout.
    Assemble {
        file: PathBuf,

        /// Optional solutions JSON mapping hole_id -> expression string.
        #[arg(long)]
        solutions: Option<PathBuf>,

        /// Optional path to shared AST database file.
        #[arg(long)]
        db: Option<PathBuf>,

        /// Write assembled module to a file instead of stdout.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Strict verification policy: require formal SMT proof.
        #[arg(long)]
        strict: bool,
    },

    /// Automatically solve and self-heal a hole using external LLM / Agent engine (mock, codex).
    Solve {
        file: PathBuf,

        /// Hole ID to resolve (e.g. `abs_impl`).
        #[arg(long)]
        hole: String,

        /// Agent engine to use: `mock` or `codex`. Defaults to `mock`.
        #[arg(long, default_value = "mock")]
        engine: String,

        /// Maximum self-healing iterations.
        #[arg(long, default_value_t = 3)]
        max_steps: usize,

        /// Optional path to shared AST database file.
        #[arg(long)]
        db: Option<PathBuf>,

        /// Strict verification policy: require formal SMT proof.
        #[arg(long)]
        strict: bool,

        /// Output auto-heal execution history as structured JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum DatabaseCommands {
    /// Parse an Axiom file and store its AST by content hash.
    Put {
        file: PathBuf,

        /// Path to the persistent AST database.
        #[arg(long, default_value = ".axiom/astdb.json")]
        db: PathBuf,
    },

    /// Export a stored module as deterministic Axiom source.
    Export {
        hash: NodeHash,

        /// Path to the persistent AST database.
        #[arg(long, default_value = ".axiom/astdb.json")]
        db: PathBuf,

        /// Write source to a file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug)]
pub enum CliError {
    ReadSource {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: ParseError,
    },
    Serialize(serde_json::Error),
    CreateDirectory {
        path: PathBuf,
        source: io::Error,
    },
    WriteFile {
        path: PathBuf,
        source: io::Error,
    },
    Transpile {
        path: PathBuf,
        source: TranspileError,
    },
    RustCompiler(io::Error),
    RustCompilationFailed(Option<i32>),
    Execute {
        path: PathBuf,
        source: io::Error,
    },
    ProgramFailed(Option<i32>),
    VerificationFailed,
    OpenDatabase {
        path: PathBuf,
        source: DbError,
    },
    DatabaseProjection(VfsError),
    Wit(WitError),
    Ffi(FfiError),
    InvalidFfiLanguage(String),
    InvalidBridgeTarget(String),
    Pool(PoolError),
    HoleVerificationRejected(String),
    UnverifiedContracts {
        message: String,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadSource { path, source } => {
                write!(formatter, "could not read `{}`: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "could not parse `{}`: {source}", path.display())
            }
            Self::Serialize(source) => {
                write!(formatter, "could not serialize: {source}")
            }
            Self::CreateDirectory { path, source } => write!(
                formatter,
                "could not create output directory `{}`: {source}",
                path.display()
            ),
            Self::WriteFile { path, source } => {
                write!(formatter, "could not write `{}`: {source}", path.display())
            }
            Self::Transpile { path, source } => {
                write!(
                    formatter,
                    "could not transpile `{}`: {source}",
                    path.display()
                )
            }
            Self::RustCompiler(source) => write!(formatter, "could not start rustc: {source}"),
            Self::RustCompilationFailed(code) => {
                write!(formatter, "rustc failed with {}", format_exit_code(*code))
            }
            Self::Execute { path, source } => {
                write!(
                    formatter,
                    "could not execute `{}`: {source}",
                    path.display()
                )
            }
            Self::ProgramFailed(code) => {
                write!(formatter, "program failed with {}", format_exit_code(*code))
            }
            Self::VerificationFailed => write!(formatter, "verification failed with diagnostics"),
            Self::OpenDatabase { path, source } => {
                write!(
                    formatter,
                    "could not open AST database `{}`: {source}",
                    path.display()
                )
            }
            Self::DatabaseProjection(source) => write!(formatter, "{source}"),
            Self::Wit(source) => write!(formatter, "could not generate WIT: {source}"),
            Self::Ffi(source) => write!(formatter, "could not generate FFI bindings: {source}"),
            Self::InvalidFfiLanguage(lang) => {
                write!(
                    formatter,
                    "unsupported FFI target language `{lang}` (supported: ts, c, rust)"
                )
            }
            Self::InvalidBridgeTarget(target) => {
                write!(
                    formatter,
                    "unsupported bridge target `{target}` (supported: unity)"
                )
            }
            Self::Pool(source) => write!(formatter, "{source}"),
            Self::HoleVerificationRejected(reason) => {
                write!(formatter, "hole verification rejected: {reason}")
            }
            Self::UnverifiedContracts { message } => {
                write!(formatter, "formal verification gate failed: {message}")
            }
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadSource { source, .. }
            | Self::CreateDirectory { source, .. }
            | Self::WriteFile { source, .. }
            | Self::RustCompiler(source)
            | Self::Execute { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Serialize(source) => Some(source),
            Self::Transpile { source, .. } => Some(source),
            Self::OpenDatabase { source, .. } => Some(source),
            Self::DatabaseProjection(source) => Some(source),
            Self::Wit(source) => Some(source),
            Self::Ffi(source) => Some(source),
            Self::Pool(source) => Some(source),
            Self::InvalidFfiLanguage(_)
            | Self::InvalidBridgeTarget(_)
            | Self::RustCompilationFailed(_)
            | Self::ProgramFailed(_)
            | Self::VerificationFailed
            | Self::HoleVerificationRejected(_)
            | Self::UnverifiedContracts { .. } => None,
        }
    }
}

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Check { file } => check(&file),
        Commands::Verify { file, json, strict } => verify(&file, json, strict),
        Commands::ExportSmt2 { file, output } => export_smt2(&file, output.as_deref()),
        Commands::ExtractHoles { file, out_dir } => extract(&file, out_dir.as_deref()),
        Commands::EmitRust {
            file,
            output,
            unsafe_unverified,
        } => emit_rust(&file, output.as_deref(), unsafe_unverified),
        Commands::Run {
            file,
            unsafe_unverified,
        } => compile_and_run(&file, unsafe_unverified),
        Commands::ExportWit {
            file,
            output,
            package,
        } => export_wit(&file, output.as_deref(), package.as_deref()),
        Commands::ExportFfi {
            file,
            lang,
            output,
            guard,
        } => export_ffi(&file, &lang, output.as_deref(), guard.as_deref()),
        Commands::Scaffold {
            name,
            target,
            out_dir,
        } => scaffold(&name, &target, out_dir.as_deref()),
        Commands::Bridge {
            source,
            target,
            out,
            library_name,
        } => generate_bridge(&source, &target, &out, &library_name),
        Commands::Db { command } => match command {
            DatabaseCommands::Put { file, db } => database_put(&file, &db),
            DatabaseCommands::Export { hash, db, out } => {
                database_export(&hash, &db, out.as_deref())
            }
        },
        Commands::Pool { action } => handle_pool_action(action),
        Commands::SolverInfo { json } => solver_info(json),
    }
}

fn solver_info(json: bool) -> Result<(), CliError> {
    let identity = resolve_solver_identity();
    if json {
        let serialized = serde_json::to_string_pretty(&identity).map_err(CliError::Serialize)?;
        println!("{serialized}");
    } else {
        println!("Backend:   {}", identity.backend);
        println!("Available: {}", identity.available);
        println!("Version:   {}", identity.version);
        if let Some(path) = &identity.path {
            println!("Path:      {}", path);
        }
        if let Some(sha256) = &identity.sha256 {
            println!("SHA-256:   {}", sha256);
        }
    }
    Ok(())
}

fn generate_bridge(
    source_path: &Path,
    target: &str,
    out: &Path,
    library_name: &str,
) -> Result<(), CliError> {
    if !target.eq_ignore_ascii_case("unity") {
        return Err(CliError::InvalidBridgeTarget(target.to_owned()));
    }

    let source = read_source(source_path)?;
    let module = parse_module(&source).map_err(|source| CliError::Parse {
        path: source_path.to_owned(),
        source,
    })?;
    let files = generate_unity_bridge(&module, library_name);

    fs::create_dir_all(out).map_err(|source| CliError::CreateDirectory {
        path: out.to_owned(),
        source,
    })?;
    for (filename, contents) in [
        ("AxiomNative.cs", files.native),
        ("AxiomDoubleBuffer.cs", files.double_buffer),
        ("AxiomAssetRegistry.cs", files.asset_registry),
        ("AxiomRunner.cs", files.runner),
    ] {
        let path = out.join(filename);
        fs::write(&path, contents).map_err(|source| CliError::WriteFile { path, source })?;
    }

    println!("generated Unity bridge in {}", out.display());
    Ok(())
}

fn export_wit(file: &Path, output: Option<&Path>, package: Option<&str>) -> Result<(), CliError> {
    let source = read_source(file)?;
    let default_package = file
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("component");
    let wit = generate_wit_from_source(&source, package.unwrap_or(default_package))
        .map_err(CliError::Wit)?;

    if let Some(output) = output {
        fs::write(output, wit).map_err(|source| CliError::WriteFile {
            path: output.to_owned(),
            source,
        })?;
    } else {
        io::stdout()
            .write_all(wit.as_bytes())
            .map_err(|source| CliError::WriteFile {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
    }
    Ok(())
}

fn export_ffi(
    file: &Path,
    lang: &str,
    output: Option<&Path>,
    guard: Option<&str>,
) -> Result<(), CliError> {
    let source = read_source(file)?;
    let code = match lang.to_ascii_lowercase().as_str() {
        "ts" | "typescript" => generate_typescript_from_source(&source).map_err(CliError::Ffi)?,
        "c" | "header" => {
            let default_guard = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("AXIOM_BINDINGS")
                .to_ascii_uppercase()
                + "_H";
            generate_c_header_from_source(&source, guard.unwrap_or(&default_guard))
                .map_err(CliError::Ffi)?
        }
        "rust" | "c-abi" => {
            let module = parse_module(&source).map_err(|source| CliError::Parse {
                path: file.to_owned(),
                source,
            })?;
            generate_c_abi_rust_glue(&module).map_err(CliError::Ffi)?
        }
        other => return Err(CliError::InvalidFfiLanguage(other.to_string())),
    };

    if let Some(output) = output {
        fs::write(output, code).map_err(|source| CliError::WriteFile {
            path: output.to_owned(),
            source,
        })?;
    } else {
        io::stdout()
            .write_all(code.as_bytes())
            .map_err(|source| CliError::WriteFile {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
    }
    Ok(())
}

fn scaffold(name: &str, target: &str, out_dir: Option<&Path>) -> Result<(), CliError> {
    let project_dir = match out_dir {
        Some(dir) => dir.join(name),
        None => PathBuf::from(name),
    };
    let src_dir = project_dir.join("src");

    fs::create_dir_all(&src_dir).map_err(|source| CliError::CreateDirectory {
        path: src_dir.clone(),
        source,
    })?;

    let main_ax = format!(
        r#"// {name} - Axiom Core Logic
type State = {{ count: Int }}

fn init() -> State {{
    State {{ count: 0 }}
}}

fn calculate_next_count(current: Int) -> Int
    requires {{ current >= 0, current < 1000000 }}
    ensures {{ result > current }}
{{
    current + 1
}}

fn increment(s: State) -> State {{
    State {{ count: calculate_next_count(s.count) }}
}}
"#
    );

    let config = format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "target": "{target}",
  "main": "src/main.ax"
}}
"#
    );

    fs::write(src_dir.join("main.ax"), main_ax).map_err(|source| CliError::WriteFile {
        path: src_dir.join("main.ax"),
        source,
    })?;

    fs::write(project_dir.join("axiom.json"), config).map_err(|source| CliError::WriteFile {
        path: project_dir.join("axiom.json"),
        source,
    })?;

    println!(
        "Scaffolded new Axiom project `{name}` ({target}) in {}",
        project_dir.display()
    );
    Ok(())
}

fn database_put(file: &Path, database_path: &Path) -> Result<(), CliError> {
    let source = read_source(file)?;
    let mut database = open_database(database_path)?;
    let summary =
        sync_source_to_database(&mut database, &source).map_err(CliError::DatabaseProjection)?;
    println!("{}", summary.module_hash);
    Ok(())
}

fn database_export(
    module_hash: &NodeHash,
    database_path: &Path,
    output: Option<&Path>,
) -> Result<(), CliError> {
    let database = open_database(database_path)?;
    let source =
        export_database_module(&database, module_hash).map_err(CliError::DatabaseProjection)?;

    if let Some(output) = output {
        fs::write(output, source).map_err(|source| CliError::WriteFile {
            path: output.to_owned(),
            source,
        })?;
    } else {
        io::stdout()
            .write_all(source.as_bytes())
            .map_err(|source| CliError::WriteFile {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
    }
    Ok(())
}

fn open_database(path: &Path) -> Result<AstDatabase, CliError> {
    AstDatabase::open(path).map_err(|source| CliError::OpenDatabase {
        path: path.to_owned(),
        source,
    })
}

fn check(file: &Path) -> Result<(), CliError> {
    let source = read_source(file)?;
    let module = parse_module(&source).map_err(|source| CliError::Parse {
        path: file.to_owned(),
        source,
    })?;

    let reports = diagnose_module(&module);
    let mut has_semantic_error = false;

    for r in &reports {
        match r.status {
            DiagnosticStatus::EffectViolation | DiagnosticStatus::TypeError => {
                eprintln!("error in `{}`: {}", file.display(), r.diagnosis);
                eprintln!("  hint: {}", r.suggested_action);
                has_semantic_error = true;
            }
            _ => {}
        }
    }

    if has_semantic_error {
        return Err(CliError::VerificationFailed);
    }

    let hole_count = extract_holes(&module).len();

    println!(
        "OK {}: {} type(s), {} effect(s), {} function(s), {} unresolved hole(s)",
        file.display(),
        module.types.len(),
        module.effects.len(),
        module.functions.len(),
        hole_count
    );
    Ok(())
}

fn verify(file: &Path, json: bool, strict: bool) -> Result<(), CliError> {
    let source = read_source(file)?;
    let reports = diagnose_source(&source);

    // Fail-Closed verification evaluation:
    // 1. Any ParseError, TypeError, EffectViolation, UnresolvedHole, ContractViolation, Unknown -> HARD FAIL.
    // 2. If function has contracts:
    //    - Proved -> PASS
    //    - Sampled -> PASS in default mode (with notice), FAIL in --strict mode
    //    - Skipped -> HARD FAIL (contract declared but skipped = unverified false positive risk!)
    // 3. If function has NO contracts:
    //    - Skipped -> PASS (no contracts to prove)
    let all_ok = reports.iter().all(|r| match r.status {
        DiagnosticStatus::Proved | DiagnosticStatus::Tested => true,
        DiagnosticStatus::Sampled => !strict,
        DiagnosticStatus::Skipped => !r.has_contracts,
        DiagnosticStatus::VacuousContract
        | DiagnosticStatus::ContractViolation
        | DiagnosticStatus::Unknown
        | DiagnosticStatus::TypeError
        | DiagnosticStatus::EffectViolation
        | DiagnosticStatus::ParseError
        | DiagnosticStatus::UnresolvedHole => false,
    });

    if json {
        let json_text = serde_json::to_string_pretty(&reports).map_err(CliError::Serialize)?;
        println!("{json_text}");
    } else {
        let solver_name = if find_z3_executable().is_some() {
            "Microsoft Z3 SMT-LIB2 Engine"
        } else {
            "Axiom Symbolic Interval Engine (Discrete Grid Sampling)"
        };
        println!("Verifying `{}` using {}...", file.display(), solver_name);
        for r in &reports {
            let func_name = r.function_name.as_deref().unwrap_or("module");
            match r.status {
                DiagnosticStatus::Proved => {
                    println!("  [PROVED] {func_name} (formally proved by SMT solver)");
                }
                DiagnosticStatus::Tested => {
                    println!("  [TESTED] {func_name} (verified on deterministic test vectors)");
                }
                DiagnosticStatus::VacuousContract => {
                    println!(
                        "  [VACUOUS_FAIL] {func_name} - Contract is vacuously true: {}",
                        r.diagnosis
                    );
                }
                DiagnosticStatus::Sampled => {
                    if strict {
                        println!("  [SAMPLED_FAIL] {func_name} (only sampled; --strict requires formal Z3 proof)");
                    } else {
                        println!("  [SAMPLED] {func_name} (passed grid test, install Z3 for formal proof)");
                    }
                }
                DiagnosticStatus::Skipped => {
                    if r.has_contracts {
                        println!("  [SKIPPED_FAIL] {func_name} - Has formal contracts but was skipped (Fail-Closed)");
                    } else {
                        println!("  [SKIPPED] {func_name} - {}", r.diagnosis);
                    }
                }
                DiagnosticStatus::Unknown => {
                    println!("  [UNKNOWN] {func_name} - {}", r.diagnosis);
                }
                DiagnosticStatus::ContractViolation => {
                    println!("  [VIOLATION] {func_name} - {}", r.diagnosis);
                    if let Some(ce) = &r.counter_example {
                        println!("    Counter-Example: {ce:?}");
                    }
                }
                DiagnosticStatus::TypeError => {
                    println!("  [TYPE_ERROR] {func_name} - {}", r.diagnosis);
                }
                DiagnosticStatus::EffectViolation => {
                    println!("  [EFFECT_ERROR] {func_name} - {}", r.diagnosis);
                }
                DiagnosticStatus::ParseError => {
                    println!("  [PARSE_ERROR] {}", r.diagnosis);
                }
                DiagnosticStatus::UnresolvedHole => {
                    println!("  [UNRESOLVED_HOLE] {func_name} - {}", r.diagnosis);
                }
            }
        }
    }

    if all_ok {
        Ok(())
    } else {
        Err(CliError::VerificationFailed)
    }
}

fn export_smt2(file: &Path, output: Option<&Path>) -> Result<(), CliError> {
    let source = read_source(file)?;
    let module = parse_module(&source).map_err(|source| CliError::Parse {
        path: file.to_owned(),
        source,
    })?;

    let mut combined_smt =
        String::from("; Standard SMT-LIB2 export generated by Axiom Toolchain\n\n");
    let mut exported_count = 0;

    for func in &module.functions {
        if func.ensures.is_empty() || !func.effects.is_empty() {
            continue;
        }
        match generate_smtlib2_with_module(func, Some(&module)) {
            Ok(query) => {
                combined_smt.push_str(&format!("; === Function: {} ===\n", func.name));
                combined_smt.push_str(&query);
                combined_smt.push_str("\n\n");
                exported_count += 1;
            }
            Err(err) => {
                eprintln!(
                    "warning: could not export SMT-LIB2 for `{}`: {err}",
                    func.name
                );
            }
        }
    }

    if let Some(output) = output {
        fs::write(output, combined_smt).map_err(|source| CliError::WriteFile {
            path: output.to_owned(),
            source,
        })?;
        println!(
            "Exported {} SMT-LIB2 verification query/queries to {}",
            exported_count,
            output.display()
        );
    } else {
        io::stdout()
            .write_all(combined_smt.as_bytes())
            .map_err(|source| CliError::WriteFile {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
    }

    Ok(())
}

fn extract(file: &Path, out_dir: Option<&Path>) -> Result<(), CliError> {
    let source = read_source(file)?;
    let module = parse_module(&source).map_err(|source| CliError::Parse {
        path: file.to_owned(),
        source,
    })?;
    let tasks = extract_holes(&module);

    if let Some(out_dir) = out_dir {
        write_hole_tasks(out_dir, &tasks)?;
        println!(
            "extracted {} hole task(s) to {}",
            tasks.len(),
            out_dir.display()
        );
    } else {
        let json = serde_json::to_string_pretty(&tasks).map_err(CliError::Serialize)?;
        println!("{json}");
    }
    Ok(())
}

fn write_hole_tasks(out_dir: &Path, tasks: &[HoleTask]) -> Result<(), CliError> {
    fs::create_dir_all(out_dir).map_err(|source| CliError::CreateDirectory {
        path: out_dir.to_owned(),
        source,
    })?;

    for task in tasks {
        let filename = if task.hole_id == "hole" || task.hole_id.is_empty() {
            &task.target_function
        } else {
            &task.hole_id
        };
        let path = out_dir.join(format!("{filename}.json"));
        let mut json = task.to_json_pretty();
        json.push('\n');
        fs::write(&path, json).map_err(|source| CliError::WriteFile { path, source })?;
    }
    Ok(())
}

fn verify_and_transpile_source(
    file: &Path,
    source: &str,
    unsafe_unverified: bool,
) -> Result<String, CliError> {
    let module = parse_module(source).map_err(|source| CliError::Parse {
        path: file.to_owned(),
        source,
    })?;

    let has_contracted = module.functions.iter().any(|f| f.has_contracts());

    if !unsafe_unverified && has_contracted {
        let smt_results = axiom_smt::verify_module(&module);
        for f in &module.functions {
            if f.has_contracts() {
                match smt_results.get(&f.name) {
                    Some(axiom_smt::SmtResult::Proved { .. }) => {}
                    other => {
                        let reason = match other {
                            Some(axiom_smt::SmtResult::CounterExample(ce)) => {
                                format!("counter-example found: {ce:?}")
                            }
                            Some(axiom_smt::SmtResult::VacuousContract { reason }) => {
                                format!("vacuous contract: {reason}")
                            }
                            Some(axiom_smt::SmtResult::Skipped(reason)) => {
                                format!("verification skipped: {reason}")
                            }
                            Some(axiom_smt::SmtResult::Unknown(reason)) => {
                                format!("solver unknown: {reason}")
                            }
                            Some(axiom_smt::SmtResult::Sampled { .. }) => {
                                "only discretely sampled, not mathematically proved".into()
                            }
                            None => "no verification result".into(),
                            _ => unreachable!(),
                        };
                        return Err(CliError::UnverifiedContracts {
                            message: format!(
                                "function `{}` has formal contracts that are NOT proved ({reason}). Pass `--unsafe-unverified` to override.",
                                f.name
                            ),
                        });
                    }
                }
            }
        }
    }

    let mut rust = transpile_to_rust(&module).map_err(|source| CliError::Transpile {
        path: file.to_owned(),
        source,
    })?;

    if unsafe_unverified {
        rust = format!(
            "// WARNING: Generated with --unsafe-unverified, contracts are NOT formally proved.\n{rust}"
        );
    } else if has_contracted {
        rust = format!(
            "// [Axiom Verification Receipt] All formal contracts proved by SMT solver.\n{rust}"
        );
    }

    Ok(rust)
}

fn emit_rust(file: &Path, output: Option<&Path>, unsafe_unverified: bool) -> Result<(), CliError> {
    let source = read_source(file)?;
    let rust = verify_and_transpile_source(file, &source, unsafe_unverified)?;

    if let Some(output) = output {
        fs::write(output, rust).map_err(|source| CliError::WriteFile {
            path: output.to_owned(),
            source,
        })?;
    } else {
        io::stdout()
            .write_all(rust.as_bytes())
            .map_err(|source| CliError::WriteFile {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
    }
    Ok(())
}

fn compile_and_run(file: &Path, unsafe_unverified: bool) -> Result<(), CliError> {
    let source = read_source(file)?;
    let rust = verify_and_transpile_source(file, &source, unsafe_unverified)?;
    let build_dir = TemporaryBuildDirectory::create()?;
    let rust_path = build_dir.path.join("main.rs");
    let executable_path = build_dir
        .path
        .join(format!("axiom-program{}", std::env::consts::EXE_SUFFIX));
    fs::write(&rust_path, rust).map_err(|source| CliError::WriteFile {
        path: rust_path.clone(),
        source,
    })?;

    let compile_status = Command::new("rustc")
        .arg("--edition=2021")
        .arg(&rust_path)
        .arg("-o")
        .arg(&executable_path)
        .status()
        .map_err(CliError::RustCompiler)?;
    if !compile_status.success() {
        return Err(CliError::RustCompilationFailed(compile_status.code()));
    }

    let program_status =
        Command::new(&executable_path)
            .status()
            .map_err(|source| CliError::Execute {
                path: executable_path,
                source,
            })?;
    if !program_status.success() {
        return Err(CliError::ProgramFailed(program_status.code()));
    }
    Ok(())
}

fn read_source(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::ReadSource {
        path: path.to_owned(),
        source,
    })
}

fn format_exit_code(code: Option<i32>) -> String {
    code.map_or_else(|| "no exit code".into(), |code| format!("exit code {code}"))
}

struct TemporaryBuildDirectory {
    path: PathBuf,
}

impl TemporaryBuildDirectory {
    fn create() -> Result<Self, CliError> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("axiom-run-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).map_err(|source| CliError::CreateDirectory {
            path: path.clone(),
            source,
        })?;
        Ok(Self { path })
    }
}

impl Drop for TemporaryBuildDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn create_task_pool(
    file: &Path,
    db: Option<&Path>,
    policy: PoolVerificationPolicy,
) -> Result<AgentTaskPool, CliError> {
    let pool = if let Some(db_path) = db {
        AgentTaskPool::open_persisted(file, db_path).map_err(CliError::Pool)?
    } else {
        let source = read_source(file)?;
        AgentTaskPool::from_source(&source).map_err(CliError::Pool)?
    };
    Ok(pool.with_policy(policy))
}

fn handle_pool_action(action: PoolAction) -> Result<(), CliError> {
    match action {
        PoolAction::Status {
            file,
            db,
            strict,
            json,
        } => {
            let policy = if strict {
                PoolVerificationPolicy::StrictProved
            } else {
                PoolVerificationPolicy::Permissive
            };
            let pool = create_task_pool(&file, db.as_deref(), policy)?;
            let all_tasks = pool.all_tasks();
            let pending = pool.pending_hole_ids();
            if json {
                let task_dtos: Vec<serde_json::Value> = all_tasks
                    .iter()
                    .map(|task| {
                        let status = pool
                            .hole_status(&task.hole_id)
                            .unwrap_or(axiom_agent_pool::HoleStatus::Unresolved);
                        let env = pool.envelope_for_hole(&task.hole_id);
                        serde_json::json!({
                            "hole_id": task.hole_id,
                            "target_function": task.target_function,
                            "status": match status {
                                axiom_agent_pool::HoleStatus::Verified => "VERIFIED",
                                axiom_agent_pool::HoleStatus::Provisional => "PROVISIONAL",
                                axiom_agent_pool::HoleStatus::Unresolved => "UNRESOLVED",
                            },
                            "allowed_effects": task.allowed_effects,
                            "envelope": env,
                        })
                    })
                    .collect();
                let status_dto = serde_json::json!({
                    "file": file.display().to_string(),
                    "db": db.as_ref().map(|p| p.display().to_string()),
                    "policy": format!("{:?}", pool.policy()),
                    "total_tasks": all_tasks.len(),
                    "pending_holes": pending.len(),
                    "tasks": task_dtos,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status_dto).map_err(CliError::Serialize)?
                );
            } else {
                println!("Axiom Task Pool: {}", file.display());
                if let Some(db_path) = db {
                    println!("Database: {}", db_path.display());
                }
                println!("Policy: {:?}", pool.policy());
                println!("Total Tasks: {}", all_tasks.len());
                println!("Pending Holes: {}", pending.len());
                for task in all_tasks {
                    let status = pool
                        .hole_status(&task.hole_id)
                        .unwrap_or(axiom_agent_pool::HoleStatus::Unresolved);
                    let status_str = match status {
                        axiom_agent_pool::HoleStatus::Verified => "[VERIFIED]",
                        axiom_agent_pool::HoleStatus::Provisional => "[PROVISIONAL]",
                        axiom_agent_pool::HoleStatus::Unresolved => "[UNRESOLVED]",
                    };
                    let eff_str = if task.allowed_effects.is_empty() {
                        "pure".to_string()
                    } else {
                        format!("effects: [{}]", task.allowed_effects.join(", "))
                    };
                    println!(
                        "  {} ?{} in function `{}` ({})",
                        status_str, task.hole_id, task.target_function, eff_str
                    );
                }
            }
            Ok(())
        }
        PoolAction::Submit {
            file,
            hole,
            expr,
            db,
            strict,
            json,
        } => {
            let solution_expr = if Path::new(&expr).exists() {
                fs::read_to_string(&expr)
                    .map_err(|source| CliError::ReadSource {
                        path: PathBuf::from(&expr),
                        source,
                    })?
                    .trim()
                    .to_string()
            } else {
                expr.trim().to_string()
            };
            let policy = if strict {
                PoolVerificationPolicy::StrictProved
            } else {
                PoolVerificationPolicy::Permissive
            };
            let mut pool = create_task_pool(&file, db.as_deref(), policy)?;
            if !json {
                println!(
                    "Submitting candidate solution for hole `?{hole}` in `{}`...",
                    file.display()
                );
            }
            let outcome = pool
                .submit_solution_with_outcome(&hole, &solution_expr, "axiom-worker")
                .map_err(CliError::Pool)?;

            let dto = outcome.to_dto(&hole);
            render_submit_outcome(&dto, json, strict)
        }
        PoolAction::Assemble {
            file,
            solutions,
            output,
            db,
            strict,
        } => {
            let policy = if strict {
                PoolVerificationPolicy::StrictProved
            } else {
                PoolVerificationPolicy::Permissive
            };
            let mut pool = create_task_pool(&file, db.as_deref(), policy)?;
            if let Some(solutions_path) = solutions {
                let sol_content =
                    fs::read_to_string(&solutions_path).map_err(|source| CliError::ReadSource {
                        path: solutions_path.clone(),
                        source,
                    })?;
                let map: std::collections::HashMap<String, String> =
                    serde_json::from_str(&sol_content).map_err(CliError::Serialize)?;
                for (hole_id, expr_str) in map {
                    println!("Applying solution for ?{hole_id}...");
                    let res = pool
                        .submit_solution(&hole_id, &expr_str)
                        .map_err(CliError::Pool)?;
                    match res {
                        SmtResult::CounterExample(inputs) => {
                            return Err(CliError::HoleVerificationRejected(format!(
                                "counter-example for ?{hole_id} with inputs: {inputs:?}"
                            )));
                        }
                        SmtResult::Unknown(reason) | SmtResult::Skipped(reason) => {
                            return Err(CliError::HoleVerificationRejected(format!(
                                "verification failed for ?{hole_id}: {reason}"
                            )));
                        }
                        _ => {}
                    }
                }
            }
            let verified_module = pool.assemble_verified_module().map_err(CliError::Pool)?;
            let assembled_source = export_module_to_source(&verified_module);
            if let Some(out_path) = output {
                fs::write(&out_path, &assembled_source).map_err(|source| CliError::WriteFile {
                    path: out_path.clone(),
                    source,
                })?;
                println!(
                    "Assembled verified module written to `{}`",
                    out_path.display()
                );
            } else {
                print!("{assembled_source}");
            }
            Ok(())
        }
        PoolAction::Solve {
            file,
            hole,
            engine,
            max_steps,
            db,
            strict,
            json,
        } => {
            let policy = if strict {
                PoolVerificationPolicy::StrictProved
            } else {
                PoolVerificationPolicy::Permissive
            };
            let mut pool = create_task_pool(&file, db.as_deref(), policy)?;
            if !json {
                println!(
                    "Solving hole `?{hole}` in `{}` using engine `{engine}` (max steps: {max_steps})...",
                    file.display()
                );
            }

            let outcome = axiom_agent_pool::auto_heal::solve_and_heal_with_engine(
                &mut pool, &hole, &engine, max_steps,
            )
            .map_err(CliError::Pool)?;

            if json {
                let json_str =
                    serde_json::to_string_pretty(&outcome).map_err(CliError::Serialize)?;
                println!("{json_str}");
            } else {
                println!("Auto-Heal Summary for ?{}:", outcome.hole_id);
                println!(
                    "  Status:      {}",
                    if outcome.success {
                        "SUCCESS (Verified)"
                    } else {
                        "FAILED"
                    }
                );
                println!("  Total Steps: {}", outcome.total_steps);
                if let Some(sol) = &outcome.final_solution {
                    println!("  Solution:    {}", sol);
                }
                println!("  Healing Trajectory:");
                for step in &outcome.history {
                    println!(
                        "    Step {}: status={:?}, proved={}, candidate=`{}`",
                        step.step, step.status, step.proved, step.candidate_solution
                    );
                    if let Some(ce) = &step.counter_example {
                        println!("      Counterexample: {ce}");
                    }
                }
            }

            if outcome.success {
                Ok(())
            } else {
                Err(CliError::HoleVerificationRejected(format!(
                    "failed to find verified solution within {max_steps} steps"
                )))
            }
        }
    }
}

fn render_submit_outcome(
    dto: &axiom_agent_pool::SubmitOutcomeDto,
    json: bool,
    strict: bool,
) -> Result<(), CliError> {
    if json {
        let json_str = serde_json::to_string_pretty(dto).map_err(CliError::Serialize)?;
        println!("{json_str}");
    } else {
        for ev in &dto.cas_events {
            match ev {
                axiom_agent_pool::CasEvent::StaleRootDetected { expected, current } => {
                    println!(
                        "[CAS_EVENT:STALE_ROOT_DETECTED] expected: {:?}, current: {:?}",
                        expected, current
                    );
                }
                axiom_agent_pool::CasEvent::RebaseSucceeded {
                    rebased_root,
                    attempts,
                } => {
                    println!(
                        "[CAS_EVENT:REBASE_SUCCEEDED] rebased to root: {} (attempts: {})",
                        rebased_root, attempts
                    );
                }
                axiom_agent_pool::CasEvent::Ast3WayRebaseSucceeded {
                    rebased_root,
                    cascade_verified_count,
                } => {
                    println!(
                        "[CAS_EVENT:AST_3WAY_REBASE_SUCCEEDED] rebased to root: {} (cascade verified: {} functions)",
                        rebased_root, cascade_verified_count
                    );
                }
                axiom_agent_pool::CasEvent::AstMergeConflict {
                    conflict_count,
                    summary,
                } => {
                    println!(
                        "[CAS_EVENT:AST_MERGE_CONFLICT] {conflict_count} structural conflict(s) detected:\n{summary}"
                    );
                }
                axiom_agent_pool::CasEvent::CascadeVerificationFailed {
                    failed_functions,
                    reasons,
                } => {
                    println!(
                        "[CAS_EVENT:CASCADE_VERIFICATION_FAILED] {} downstream function(s) failed contracts:",
                        failed_functions.len()
                    );
                    for r in reasons {
                        println!("  - {r}");
                    }
                }
            }
        }

        match dto.status {
            axiom_agent_pool::SubmitStatus::Accepted => {
                let solver = dto.solver.as_deref().unwrap_or("Z3");
                let version = dto.solver_version.as_deref().unwrap_or("unknown");
                let obligations = dto.obligations_checked.unwrap_or(0);
                println!(
                    "[ACCEPTED] Hole `?{}` formally verified with SMT proof ({solver} v{version}, obligations: {obligations}).",
                    dto.hole
                );
                if dto.upgraded_to_verified {
                    println!("  [UPGRADED] Successfully upgraded hole status from provisional draft to verified.");
                }
            }
            axiom_agent_pool::SubmitStatus::Provisional => {
                let points = dto.obligations_checked.unwrap_or(0);
                if strict {
                    println!(
                        "[REJECTED] Verification sampled {points} points but strict formal proof is required."
                    );
                } else {
                    println!(
                        "[ACCEPTED] Hole `?{}` verified via sampling ({points} points checked).",
                        dto.hole
                    );
                }
            }
            axiom_agent_pool::SubmitStatus::Rejected => {
                if let Some(ce) = &dto.counter_example {
                    println!("[REJECTED] SMT counter-example found");
                    println!("Counter-example inputs: {ce}");
                } else {
                    println!(
                        "[REJECTED] SMT verification failed: {}",
                        dto.reason.as_deref().unwrap_or("contract violated")
                    );
                }
            }
            axiom_agent_pool::SubmitStatus::Unknown => {
                println!(
                    "[REJECTED] SMT solver returned unknown: {}",
                    dto.reason.as_deref().unwrap_or("unknown")
                );
            }
            axiom_agent_pool::SubmitStatus::Skipped => {
                println!(
                    "[REJECTED] Verification skipped: {}",
                    dto.reason.as_deref().unwrap_or("skipped")
                );
            }
        }
    }

    match dto.status {
        axiom_agent_pool::SubmitStatus::Accepted => Ok(()),
        axiom_agent_pool::SubmitStatus::Provisional if !strict => Ok(()),
        axiom_agent_pool::SubmitStatus::Provisional => Err(CliError::HoleVerificationRejected(
            dto.reason
                .clone()
                .unwrap_or_else(|| "strict formal proof required".to_string()),
        )),
        _ => Err(CliError::HoleVerificationRejected(
            dto.reason
                .clone()
                .unwrap_or_else(|| "verification rejected".to_string()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_info_cli_json_and_plain() {
        let cli_json = Cli::try_parse_from(["axiom", "solver-info", "--json"]).unwrap();
        assert!(run(cli_json).is_ok());

        let cli_plain = Cli::try_parse_from(["axiom", "solver-info"]).unwrap();
        assert!(run(cli_plain).is_ok());
    }
}

//! Command-line entry points for the Axiom toolchain.

use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use axiom_db::{AstDatabase, DbError, NodeHash};
use axiom_diagnostics::{diagnose_source, DiagnosticStatus};
use axiom_hole_extractor::{extract_holes, HoleTask};
use axiom_parser::{parse_module, ParseError};
use axiom_transpiler::{transpile_source_to_rust, TranspileError};
use axiom_vfs::{export_database_module, sync_source_to_database, VfsError};
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
    },

    /// Transpile, compile, and execute a hole-free Axiom program.
    Run { file: PathBuf },

    /// Store and retrieve content-addressed modules in the AST database.
    Db {
        #[command(subcommand)]
        command: DatabaseCommands,
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
            Self::RustCompilationFailed(_) | Self::ProgramFailed(_) | Self::VerificationFailed => {
                None
            }
        }
    }
}

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Check { file } => check(&file),
        Commands::Verify { file, json } => verify(&file, json),
        Commands::ExtractHoles { file, out_dir } => extract(&file, out_dir.as_deref()),
        Commands::EmitRust { file, output } => emit_rust(&file, output.as_deref()),
        Commands::Run { file } => compile_and_run(&file),
        Commands::Db { command } => match command {
            DatabaseCommands::Put { file, db } => database_put(&file, &db),
            DatabaseCommands::Export { hash, db, out } => {
                database_export(&hash, &db, out.as_deref())
            }
        },
    }
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

fn verify(file: &Path, json: bool) -> Result<(), CliError> {
    let source = read_source(file)?;
    let reports = diagnose_source(&source);
    let all_verified = reports.iter().all(|r| r.status == DiagnosticStatus::Verified);

    if json {
        let json_text = serde_json::to_string_pretty(&reports).map_err(CliError::Serialize)?;
        println!("{json_text}");
    } else {
        println!("Verifying `{}`...", file.display());
        for r in &reports {
            match r.status {
                DiagnosticStatus::Verified => {
                    println!("  [VERIFIED] {}", r.function_name.as_deref().unwrap_or("module"));
                }
                _ => {
                    println!("  [{:?}] {}", r.status, r.diagnosis);
                    if let Some(ce) = &r.counter_example {
                        println!("    Counter-Example: {ce:?}");
                    }
                    if !r.suggested_action.is_empty() {
                        println!("    Suggested Action: {}", r.suggested_action);
                    }
                }
            }
        }
    }

    if all_verified {
        Ok(())
    } else {
        Err(CliError::VerificationFailed)
    }
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
        let path = out_dir.join(format!("{}.json", task.hole_id));
        let mut json = task.to_json_pretty();
        json.push('\n');
        fs::write(&path, json).map_err(|source| CliError::WriteFile { path, source })?;
    }
    Ok(())
}

fn emit_rust(file: &Path, output: Option<&Path>) -> Result<(), CliError> {
    let source = read_source(file)?;
    let rust = transpile_source_to_rust(&source).map_err(|source| CliError::Transpile {
        path: file.to_owned(),
        source,
    })?;

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

fn compile_and_run(file: &Path) -> Result<(), CliError> {
    let source = read_source(file)?;
    let rust = transpile_source_to_rust(&source).map_err(|source| CliError::Transpile {
        path: file.to_owned(),
        source,
    })?;
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

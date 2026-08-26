//! Content-addressed storage for Axiom abstract syntax trees.

use axiom_ast::{EffectDecl, FunctionDecl, Module, TypeDef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// A stable, typed SHA-256 content identifier for an AST node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeHash(String);

impl NodeHash {
    /// Returns the textual content identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A malformed textual AST node hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNodeHashError {
    value: String,
}

impl fmt::Display for ParseNodeHashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid AST node hash `{}`; expected `#<kind>_<64 lowercase hex digits>`",
            self.value
        )
    }
}

impl std::error::Error for ParseNodeHashError {}

impl FromStr for NodeHash {
    type Err = ParseNodeHashError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let valid_prefix = ["#type_", "#fn_", "#effect_", "#mod_"]
            .iter()
            .find(|prefix| value.starts_with(**prefix));
        let digest = valid_prefix.and_then(|prefix| value.strip_prefix(prefix));
        if digest.is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ParseNodeHashError {
                value: value.to_owned(),
            })
        }
    }
}

impl AsRef<str> for NodeHash {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for NodeHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn compute_hash<T: Serialize>(prefix: &str, value: &T) -> NodeHash {
    // All current AST types contain only ordered structures, so compact JSON is
    // a deterministic canonical representation across database instances.
    let bytes = serde_json::to_vec(value).expect("Axiom AST serialization cannot fail");
    let digest = Sha256::digest(bytes);
    NodeHash(format!("#{prefix}_{digest:x}"))
}

/// Computes the content identifier of a type definition.
pub fn compute_type_hash(def: &TypeDef) -> NodeHash {
    compute_hash("type", def)
}

/// Computes the content identifier of a function declaration.
pub fn compute_function_hash(func: &FunctionDecl) -> NodeHash {
    compute_hash("fn", func)
}

/// Computes the content identifier of an effect declaration.
pub fn compute_effect_hash(eff: &EffectDecl) -> NodeHash {
    compute_hash("effect", eff)
}

/// Computes the content identifier of a module.
pub fn compute_module_hash(module: &Module) -> NodeHash {
    compute_hash("mod", module)
}

/// The content identifiers created while inserting a complete module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleRecord {
    pub module_hash: NodeHash,
    pub type_hashes: Vec<NodeHash>,
    pub function_hashes: Vec<NodeHash>,
    pub effect_hashes: Vec<NodeHash>,
}

impl ModuleRecord {
    /// Returns the module's content identifier.
    pub fn hash(&self) -> &NodeHash {
        &self.module_hash
    }
}

/// An error opening or updating an AST database.
#[derive(Debug)]
pub enum DbError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
}

impl fmt::Display for DbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "AST database I/O error: {error}"),
            Self::Serialization(error) => {
                write!(formatter, "invalid AST database serialization: {error}")
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Serialization(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct DatabaseState {
    #[serde(default)]
    types: BTreeMap<NodeHash, TypeDef>,
    #[serde(default)]
    functions: BTreeMap<NodeHash, FunctionDecl>,
    #[serde(default)]
    effects: BTreeMap<NodeHash, EffectDecl>,
    #[serde(default)]
    modules: BTreeMap<NodeHash, Module>,
}

/// A content-addressed AST database backed by a JSON file or memory.
#[derive(Debug)]
pub struct AstDatabase {
    path: Option<PathBuf>,
    state: DatabaseState,
}

impl AstDatabase {
    /// Opens an existing database file or creates an empty one at `path`.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if path.exists() {
            let bytes = fs::read(path)?;
            let state = serde_json::from_slice(&bytes)?;
            Ok(Self {
                path: Some(path.to_path_buf()),
                state,
            })
        } else {
            let database = Self {
                path: Some(path.to_path_buf()),
                state: DatabaseState::default(),
            };
            database.persist(&database.state)?;
            Ok(database)
        }
    }

    /// Creates an empty database without a persistent backing file.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            state: DatabaseState::default(),
        }
    }

    /// Inserts a module and every type, function, and effect it owns.
    pub fn insert_module(&mut self, module: &Module) -> Result<ModuleRecord, DbError> {
        let type_hashes = module
            .types
            .iter()
            .map(compute_type_hash)
            .collect::<Vec<_>>();
        let function_hashes = module
            .functions
            .iter()
            .map(compute_function_hash)
            .collect::<Vec<_>>();
        let effect_hashes = module
            .effects
            .iter()
            .map(compute_effect_hash)
            .collect::<Vec<_>>();
        let module_hash = compute_module_hash(module);

        let mut next_state = self.state.clone();
        for (hash, definition) in type_hashes.iter().zip(&module.types) {
            next_state.types.insert(hash.clone(), definition.clone());
        }
        for (hash, function) in function_hashes.iter().zip(&module.functions) {
            next_state.functions.insert(hash.clone(), function.clone());
        }
        for (hash, effect) in effect_hashes.iter().zip(&module.effects) {
            next_state.effects.insert(hash.clone(), effect.clone());
        }
        next_state
            .modules
            .insert(module_hash.clone(), module.clone());

        self.persist(&next_state)?;
        self.state = next_state;

        Ok(ModuleRecord {
            module_hash,
            type_hashes,
            function_hashes,
            effect_hashes,
        })
    }

    /// Retrieves a function by its content identifier.
    pub fn get_function(&self, hash: &NodeHash) -> Result<Option<FunctionDecl>, DbError> {
        Ok(self.state.functions.get(hash).cloned())
    }

    /// Retrieves a type definition by its content identifier.
    pub fn get_type(&self, hash: &NodeHash) -> Result<Option<TypeDef>, DbError> {
        Ok(self.state.types.get(hash).cloned())
    }

    /// Retrieves an effect declaration by its content identifier.
    pub fn get_effect(&self, hash: &NodeHash) -> Result<Option<EffectDecl>, DbError> {
        Ok(self.state.effects.get(hash).cloned())
    }

    /// Retrieves a module by its content identifier.
    pub fn get_module(&self, hash: &NodeHash) -> Result<Option<Module>, DbError> {
        Ok(self.state.modules.get(hash).cloned())
    }

    fn persist(&self, state: &DatabaseState) -> Result<(), DbError> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(state)?;
        fs::write(path, bytes)?;
        Ok(())
    }
}

impl Default for AstDatabase {
    fn default() -> Self {
        Self::in_memory()
    }
}

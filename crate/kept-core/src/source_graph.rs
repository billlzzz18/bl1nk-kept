//! Source graph indexing, scope graphs, symbol resolution, and incremental file event management.
//!
//! Provides lexical and structural symbol indexing (definitions, references, imports, aliases, implementations, type references)
//! for codebases with persistence, compaction, batching, and incremental file change coalescing.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Kind of symbol definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefinitionKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Variable,
    Module,
    Other(String),
}

/// Symbol definition in a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolDefinition {
    pub name: String,
    pub kind: DefinitionKind,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub doc: Option<String>,
}

/// Kind of symbol reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    Call,
    TypeUsage,
    Import,
    Implementation,
    ValueUsage,
    Other(String),
}

/// Symbol reference occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolReference {
    pub symbol_name: String,
    pub kind: ReferenceKind,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
}

/// Import declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRecord {
    pub path: String,
    pub alias: Option<String>,
    pub file_path: String,
    pub line: usize,
    pub is_wildcard: bool,
}

/// Implementation relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementationRecord {
    pub trait_name: Option<String>,
    pub target_type: String,
    pub file_path: String,
    pub line: usize,
}

/// Statistics for a source graph index snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceGraphStats {
    pub total_files: usize,
    pub total_definitions: usize,
    pub total_references: usize,
    pub total_imports: usize,
    pub total_implementations: usize,
}

/// In-memory scope graph index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeGraphIndex {
    pub definitions: Vec<SymbolDefinition>,
    pub references: Vec<SymbolReference>,
    pub imports: Vec<ImportRecord>,
    pub implementations: Vec<ImplementationRecord>,
    pub indexed_files: HashSet<String>,
}

pub type SourceGraph = ScopeGraphIndex;
pub type GraphIndex = ScopeGraphIndex;

impl ScopeGraphIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Find all definitions matching symbol name.
    pub fn find_definitions(&self, name: &str) -> Vec<SymbolDefinition> {
        self.definitions
            .iter()
            .filter(|d| d.name == name)
            .cloned()
            .collect()
    }

    /// Find all references to a symbol name.
    pub fn find_references(&self, name: &str) -> Vec<SymbolReference> {
        self.references
            .iter()
            .filter(|r| r.symbol_name == name)
            .cloned()
            .collect()
    }

    /// Get index statistics.
    pub fn stats(&self) -> SourceGraphStats {
        SourceGraphStats {
            total_files: self.indexed_files.len(),
            total_definitions: self.definitions.len(),
            total_references: self.references.len(),
            total_imports: self.imports.len(),
            total_implementations: self.implementations.len(),
        }
    }

    /// Compact internal vectors by deduplicating and removing orphan references.
    pub fn compact(&mut self) {
        let mut seen_defs = HashSet::new();
        self.definitions
            .retain(|d| seen_defs.insert((d.name.clone(), d.file_path.clone(), d.line, d.column)));

        let mut seen_refs = HashSet::new();
        self.references.retain(|r| {
            seen_refs.insert((
                r.symbol_name.clone(),
                format!("{:?}", r.kind),
                r.file_path.clone(),
                r.line,
                r.column,
            ))
        });

        let mut seen_imports = HashSet::new();
        self.imports
            .retain(|i| seen_imports.insert((i.path.clone(), i.file_path.clone(), i.line)));

        let mut seen_impls = HashSet::new();
        self.implementations.retain(|im| {
            seen_impls.insert((
                im.trait_name.clone(),
                im.target_type.clone(),
                im.file_path.clone(),
                im.line,
            ))
        });
    }

    /// Remove all indexed records associated with a specific file.
    pub fn remove_file(&mut self, path: &str) {
        self.indexed_files.remove(path);
        self.definitions.retain(|d| d.file_path != path);
        self.references.retain(|r| r.file_path != path);
        self.imports.retain(|i| i.file_path != path);
        self.implementations.retain(|im| im.file_path != path);
    }
}

/// Configuration options for the index builder.
#[derive(Debug, Clone)]
pub struct IndexBuilderConfig {
    pub max_file_size_bytes: u64,
    pub max_prefix_check_bytes: usize,
    pub skip_hidden: bool,
    pub batch_size: usize,
}

impl Default for IndexBuilderConfig {
    fn default() -> Self {
        Self {
            max_file_size_bytes: 2 * 1024 * 1024, // 2MB
            max_prefix_check_bytes: 1024,
            skip_hidden: true,
            batch_size: 50,
        }
    }
}

/// Tree-sitter AST-based symbol extraction (Phase 1.1)
pub mod syntax;

/// Multi-language (Python, JS/TS, Go) tree-sitter extraction (Phase 1.1)
pub mod multilang;

/// Builder for constructing a `ScopeGraphIndex` from files or directories.
#[derive(Debug, Clone, Default)]
pub struct IndexBuilder {
    config: IndexBuilderConfig,
}

pub type GraphBuilder = IndexBuilder;

impl IndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: IndexBuilderConfig) -> Self {
        Self { config }
    }

    pub fn max_file_size_bytes(mut self, bytes: u64) -> Self {
        self.config.max_file_size_bytes = bytes;
        self
    }

    pub fn skip_hidden(mut self, skip: bool) -> Self {
        self.config.skip_hidden = skip;
        self
    }

    pub fn build_batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }

    /// Parses any supported source file (Rust, Python, JS/TS, Go) into symbols.
    /// Returns an empty index for unsupported extensions.
    pub fn parse_file(path: &Path, content: &str) -> ScopeGraphIndex {
        let file_path_str = path.to_string_lossy().replace('\\', "/");
        // NOTE-012: เลือกภาษาจาก extension — ไม่รองรับ = คืน index ว่าง
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        let Some(language) = multilang::language_from_extension(extension) else {
            return ScopeGraphIndex::new();
        };
        if language == multilang::Language::Rust {
            return Self::parse_rust_file(path, content);
        }
        // NOTE-015: ใช้ match แทน unwrap_or_else เพื่อเลี่ยง clippy::unwrap_or_default
        match multilang::parse_source(language, &file_path_str, content) {
            Some(index) => index,
            None => ScopeGraphIndex::new(),
        }
    }

    /// Checks if a file is binary by inspecting bounded initial bytes.
    pub fn is_binary_file(path: &Path, max_bytes: usize) -> bool {
        if let Ok(mut file) = fs::File::open(path) {
            use std::io::Read;
            let mut buf = vec![0u8; max_bytes];
            if let Ok(n) = file.read(&mut buf) {
                return buf[..n].contains(&0);
            }
        }
        false
    }

    /// Parses a single Rust source file into symbols using tree-sitter AST.
    /// Falls back to the legacy regex scanner if the AST parser is unavailable.
    pub fn parse_rust_file(path: &Path, content: &str) -> ScopeGraphIndex {
        let file_path_str = path.to_string_lossy().replace('\\', "/");
        // NOTE-008: AST parsing ตรงตาม grammar — regex scanner เก็บไว้เป็น fallback เท่านั้น
        if let Some(index) = syntax::parse_rust_ast(&file_path_str, content) {
            return index;
        }
        Self::parse_rust_file_regex(path, content)
    }

    /// Legacy regex-based scanner (fallback เมื่อ AST parser ใช้ไม่ได้)
    pub fn parse_rust_file_regex(path: &Path, content: &str) -> ScopeGraphIndex {
        let mut index = ScopeGraphIndex::new();
        let file_path_str = path.to_string_lossy().replace('\\', "/");
        index.indexed_files.insert(file_path_str.clone());

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();

            if trimmed.starts_with("//") || trimmed.is_empty() {
                continue;
            }

            // Function definition: fn name(...) or pub fn name(...)
            if let Some(fn_idx) = trimmed.find("fn ") {
                let rest = &trimmed[fn_idx + 3..];
                if let Some(name_end) =
                    rest.find(|c: char| c == '(' || c == '<' || c.is_whitespace())
                {
                    let fn_name = rest[..name_end].trim();
                    if !fn_name.is_empty() {
                        index.definitions.push(SymbolDefinition {
                            name: fn_name.to_string(),
                            kind: DefinitionKind::Function,
                            file_path: file_path_str.clone(),
                            line: line_num,
                            column: 1,
                            doc: None,
                        });
                    }
                }
            }

            // Struct definition: struct Name ... or pub struct Name ...
            if let Some(st_idx) = trimmed.find("struct ") {
                let rest = &trimmed[st_idx + 7..];
                let name_end = rest
                    .find(|c: char| {
                        c == '{' || c == '(' || c == ';' || c == '<' || c.is_whitespace()
                    })
                    .unwrap_or(rest.len());
                let struct_name = rest[..name_end].trim();
                if !struct_name.is_empty() {
                    index.definitions.push(SymbolDefinition {
                        name: struct_name.to_string(),
                        kind: DefinitionKind::Struct,
                        file_path: file_path_str.clone(),
                        line: line_num,
                        column: 1,
                        doc: None,
                    });
                }
            }

            // Enum definition: enum Name ... or pub enum Name ...
            if let Some(en_idx) = trimmed.find("enum ") {
                let rest = &trimmed[en_idx + 5..];
                let name_end = rest
                    .find(|c: char| c == '{' || c == '<' || c.is_whitespace())
                    .unwrap_or(rest.len());
                let enum_name = rest[..name_end].trim();
                if !enum_name.is_empty() {
                    index.definitions.push(SymbolDefinition {
                        name: enum_name.to_string(),
                        kind: DefinitionKind::Enum,
                        file_path: file_path_str.clone(),
                        line: line_num,
                        column: 1,
                        doc: None,
                    });
                }
            }

            // Trait definition: trait Name ... or pub trait Name ...
            if let Some(tr_idx) = trimmed.find("trait ") {
                let rest = &trimmed[tr_idx + 6..];
                let name_end = rest
                    .find(|c: char| c == '{' || c == ':' || c == '<' || c.is_whitespace())
                    .unwrap_or(rest.len());
                let trait_name = rest[..name_end].trim();
                if !trait_name.is_empty() {
                    index.definitions.push(SymbolDefinition {
                        name: trait_name.to_string(),
                        kind: DefinitionKind::Trait,
                        file_path: file_path_str.clone(),
                        line: line_num,
                        column: 1,
                        doc: None,
                    });
                }
            }

            // Type alias: type Name = ...
            if let Some(tp_idx) = trimmed.find("type ") {
                let rest = &trimmed[tp_idx + 5..];
                let name_end = rest
                    .find(|c: char| c == '=' || c == '<' || c.is_whitespace())
                    .unwrap_or(rest.len());
                let type_name = rest[..name_end].trim();
                if !type_name.is_empty() {
                    index.definitions.push(SymbolDefinition {
                        name: type_name.to_string(),
                        kind: DefinitionKind::TypeAlias,
                        file_path: file_path_str.clone(),
                        line: line_num,
                        column: 1,
                        doc: None,
                    });
                }
            }

            // Impl blocks: impl Trait for Type or impl Type
            if let Some(stripped) = trimmed.strip_prefix("impl ") {
                let rest = stripped.trim_end_matches('{').trim();
                if let Some(for_idx) = rest.find(" for ") {
                    let trait_part = rest[..for_idx].trim().to_string();
                    let target_part = rest[for_idx + 5..].trim().to_string();
                    index.implementations.push(ImplementationRecord {
                        trait_name: Some(trait_part),
                        target_type: target_part,
                        file_path: file_path_str.clone(),
                        line: line_num,
                    });
                } else {
                    index.implementations.push(ImplementationRecord {
                        trait_name: None,
                        target_type: rest.to_string(),
                        file_path: file_path_str.clone(),
                        line: line_num,
                    });
                }
            }

            // Use / Imports
            if let Some(stripped) = trimmed.strip_prefix("use ") {
                let use_body = stripped.trim_end_matches(';').trim();
                let is_wildcard = use_body.ends_with("::*");
                let (path_str, alias) = if let Some(as_idx) = use_body.find(" as ") {
                    let p = use_body[..as_idx].trim().to_string();
                    let a = use_body[as_idx + 4..].trim().to_string();
                    (p, Some(a))
                } else {
                    (use_body.to_string(), None)
                };

                index.imports.push(ImportRecord {
                    path: path_str,
                    alias,
                    file_path: file_path_str.clone(),
                    line: line_num,
                    is_wildcard,
                });
            }

            // Function calls / Type usages (references)
            for part in trimmed.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if !part.is_empty()
                    && part
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == '_')
                    && part.chars().next().is_some_and(|c| c.is_uppercase())
                {
                    index.references.push(SymbolReference {
                        symbol_name: part.to_string(),
                        kind: ReferenceKind::TypeUsage,
                        file_path: file_path_str.clone(),
                        line: line_num,
                        column: 1,
                    });
                }
            }
        }

        index
    }

    /// Build scope graph index from a root directory.
    pub fn build_from_directory(&self, root: &Path) -> ScopeGraphIndex {
        let mut index = ScopeGraphIndex::new();
        self.scan_recursive(root, &mut index);
        index.compact();
        index
    }

    fn scan_recursive(&self, dir: &Path, index: &mut ScopeGraphIndex) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if self.config.skip_hidden && name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                self.scan_recursive(&path, index);
            } else if path.is_file() {
                // NOTE-014: index ทุกภาษาที่ parse_file รองรับ (Rust/Python/JS/TS/Go)
                let supported = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|ext| multilang::language_from_extension(ext).is_some());
                if supported {
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() > self.config.max_file_size_bytes {
                            continue;
                        }
                    }

                    if Self::is_binary_file(&path, self.config.max_prefix_check_bytes) {
                        continue;
                    }

                    if let Ok(content) = fs::read_to_string(&path) {
                        let file_idx = Self::parse_file(&path, &content);
                        index.definitions.extend(file_idx.definitions);
                        index.references.extend(file_idx.references);
                        index.imports.extend(file_idx.imports);
                        index.implementations.extend(file_idx.implementations);
                        index.indexed_files.extend(file_idx.indexed_files);
                    }
                }
            }
        }
    }
}

/// Type of file event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEventKind {
    Created,
    Modified,
    Deleted,
}

/// Change event on a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
}

pub type SourceFileEvent = FileEvent;
pub type GraphEvent = FileEvent;

impl FileEvent {
    pub fn created(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: FileEventKind::Created,
        }
    }

    pub fn modified(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: FileEventKind::Modified,
        }
    }

    pub fn deleted(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: FileEventKind::Deleted,
        }
    }
}

/// Coalesce rapid or overlapping file events for the same path.
pub fn coalesce_file_events(events: &[FileEvent]) -> Vec<FileEvent> {
    let mut map: HashMap<PathBuf, FileEventKind> = HashMap::new();

    for ev in events {
        match ev.kind {
            FileEventKind::Created => {
                map.insert(ev.path.clone(), FileEventKind::Created);
            }
            FileEventKind::Modified => {
                map.entry(ev.path.clone())
                    .and_modify(|k| {
                        if *k != FileEventKind::Created {
                            *k = FileEventKind::Modified;
                        }
                    })
                    .or_insert(FileEventKind::Modified);
            }
            FileEventKind::Deleted => {
                if let Some(FileEventKind::Created) = map.get(&ev.path) {
                    map.remove(&ev.path);
                } else {
                    map.insert(ev.path.clone(), FileEventKind::Deleted);
                }
            }
        }
    }

    map.into_iter()
        .map(|(path, kind)| FileEvent { path, kind })
        .collect()
}

/// Configuration for IndexManager.
#[derive(Debug, Clone, Default)]
pub struct IndexManagerConfig {
    pub builder_config: IndexBuilderConfig,
}

pub type KeptGraphIndexConfig = IndexManagerConfig;

/// Manages thread-safe incremental index updates and snapshots.
pub struct IndexManager {
    config: IndexManagerConfig,
    index: Arc<RwLock<ScopeGraphIndex>>,
}

pub type GraphManager = IndexManager;

impl IndexManager {
    pub fn new(config: IndexManagerConfig) -> Self {
        Self {
            config,
            index: Arc::new(RwLock::new(ScopeGraphIndex::new())),
        }
    }

    pub fn from_index(index: ScopeGraphIndex) -> Self {
        Self {
            config: IndexManagerConfig::default(),
            index: Arc::new(RwLock::new(index)),
        }
    }

    /// Returns an Arc snapshot of the current scope graph index.
    pub fn get_snapshot(&self) -> Arc<ScopeGraphIndex> {
        // NOTE-009: RwLock poison recovery — ใช้ into_inner() เมื่อ lock poisoned เพื่อไม่ panic
        let guard = self.index.read().unwrap_or_else(|e| e.into_inner());
        Arc::new(guard.clone())
    }

    /// Process a batch of incremental file events with coalescing.
    pub fn apply_events(&self, events: &[FileEvent]) {
        let coalesced = coalesce_file_events(events);
        let mut guard = self.index.write().unwrap_or_else(|e| e.into_inner());

        for event in coalesced {
            let path_str = event.path.to_string_lossy().replace('\\', "/");
            match event.kind {
                FileEventKind::Deleted => {
                    guard.remove_file(&path_str);
                }
                FileEventKind::Created | FileEventKind::Modified => {
                    guard.remove_file(&path_str);
                    if event.path.is_file() {
                        if IndexBuilder::is_binary_file(
                            &event.path,
                            self.config.builder_config.max_prefix_check_bytes,
                        ) {
                            continue;
                        }
                        if let Ok(meta) = fs::metadata(&event.path) {
                            if meta.len() > self.config.builder_config.max_file_size_bytes {
                                continue;
                            }
                        }
                        if let Ok(content) = fs::read_to_string(&event.path) {
                            let file_idx = IndexBuilder::parse_file(&event.path, &content);
                            guard.definitions.extend(file_idx.definitions);
                            guard.references.extend(file_idx.references);
                            guard.imports.extend(file_idx.imports);
                            guard.implementations.extend(file_idx.implementations);
                            guard.indexed_files.extend(file_idx.indexed_files);
                        }
                    }
                }
            }
        }
        guard.compact();
    }
}

/// Save index to disk as JSON.
pub fn save_index(index: &ScopeGraphIndex, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, json)?;
    Ok(())
}

/// Load index from disk JSON.
pub fn load_index(path: impl AsRef<Path>) -> Result<ScopeGraphIndex, std::io::Error> {
    let content = fs::read_to_string(path)?;
    let index: ScopeGraphIndex = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rust_definitions_and_references() {
        let code = r#"
            pub struct UserProfile {
                pub id: u64,
            }

            pub trait AuthProvider {
                fn authenticate(&self) -> bool;
            }

            impl AuthProvider for UserProfile {
                fn authenticate(&self) -> bool {
                    true
                }
            }

            use std::collections::HashMap as Map;
            use std::sync::*;

            pub fn process_user(profile: UserProfile) {
                let check = profile.authenticate();
            }
        "#;

        let path = Path::new("src/auth.rs");
        let index = IndexBuilder::parse_rust_file(path, code);

        assert_eq!(index.find_definitions("UserProfile").len(), 1);
        assert_eq!(index.find_definitions("AuthProvider").len(), 1);
        assert_eq!(index.find_definitions("process_user").len(), 1);
        assert_eq!(index.imports.len(), 2);
        assert_eq!(index.implementations.len(), 1);
        assert_eq!(index.implementations[0].target_type, "UserProfile");
        assert_eq!(
            index.implementations[0].trait_name,
            Some("AuthProvider".to_string())
        );
    }

    #[test]
    fn persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("kept-idx-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let idx_path = dir.join("index.json");

        let mut index = ScopeGraphIndex::new();
        index.definitions.push(SymbolDefinition {
            name: "TestFn".to_string(),
            kind: DefinitionKind::Function,
            file_path: "test.rs".to_string(),
            line: 1,
            column: 1,
            doc: None,
        });

        save_index(&index, &idx_path).unwrap();
        let loaded = load_index(&idx_path).unwrap();

        assert_eq!(loaded.definitions.len(), 1);
        assert_eq!(loaded.definitions[0].name, "TestFn");

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn event_coalescing() {
        let path = PathBuf::from("a.rs");
        let events = vec![
            FileEvent::created(path.clone()),
            FileEvent::modified(path.clone()),
            FileEvent::modified(path.clone()),
        ];
        let coalesced = coalesce_file_events(&events);
        assert_eq!(coalesced.len(), 1);
        assert_eq!(coalesced[0].kind, FileEventKind::Created);

        let events_del = vec![
            FileEvent::created(path.clone()),
            FileEvent::deleted(path.clone()),
        ];
        let coalesced_del = coalesce_file_events(&events_del);
        assert_eq!(coalesced_del.len(), 0);
    }
}

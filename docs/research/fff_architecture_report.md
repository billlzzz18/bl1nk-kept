# Research Report: FFF (Fast File Finder) Architecture & Integration Contract

**Target:** `https://github.com/dmtrKovalenko/fff`
**License:** MIT
**Primary Language:** Rust (Monorepo with C, Lua, Node.js, Bun, Python bindings)

---

## 1. Executive Summary & Purpose

FFF is a resident, in-memory file search and indexing engine designed specifically for **long-running applications** (AI agents, IDE extensions, Neovim plugins, MCP servers) rather than one-off shell invocations.

- **Core Problem Solved:** CLI tools like `ripgrep` and `fzf` spawn separate OS processes and re-walk the directory tree on every invocation. On large repos (500k+ files), each scan takes 3–9 seconds.
- **FFF Approach:** Initializes once via `FilePicker::new_with_shared_state()` or `SharedFilePicker`, keeps an in-memory index in resident memory, listens to filesystem events via a background watcher, and serves repeated queries in **sub-10ms** with SIMD matchers and LMDB frecency ranking.

---

## 2. Monorepo Architecture & Crate Boundaries

```
fff/
├── crates/
│   ├── fff-search          # Core Rust engine: FilePicker, indexing, watcher, frecency, query history
│   ├── fff-grep            # Content search engine: SIMD plain-text matcher, regex, multi-grep (Aho-Corasick)
│   ├── fff-query-parser    # Query & constraint syntax parser (git status, size, date, path glob)
│   ├── fff-c               # Stable C ABI (libfff_c) providing FFI for other language bindings
│   ├── fff-nvim            # Neovim integration via Lua / mlua bindings
│   └── fff-mcp             # Stdio MCP server binary (fffind, ffgrep, fff-multi-grep)
├── packages/
│   ├── fff-node            # Node.js SDK (@ff-labs/fff-node)
│   ├── fff-bun             # Bun SDK (@ff-labs/fff-bun)
│   ├── pi-fff              # pi agent integration extension (@ff-labs/pi-fff)
│   └── fff-python          # Python bindings via PyO3
└── lua/                    # Pure Lua frontend for Neovim (requires Neovim >= 0.10.0)
```

---

## 3. Rust API Contracts (`fff-search`)

### Core Types & Roles
- **`FilePicker`**: Single-threaded core indexer. Handles directory traversal, gitignore resolution, in-memory path trees, and fuzzy searching.
- **`SharedFilePicker`**: Thread-safe (`Arc<RwLock<Option<FilePicker>>>`) wrapper. Handles async lifecycle operations (triggering background rescans, safe reads/writes without blocking callers).
- **`FFFMode`**: Enum (`FFFMode::Ai` vs `FFFMode::Neovim`). `Ai` mode optimizes ranking and metadata extraction for agent tools.
- **`FrecencyTracker` / `SharedFrecency`**: LMDB-backed database tracking file access frequency and recency.
- **`QueryTracker` / `SharedQueryTracker`**: Search query history tracker providing "combo-boost" scoring for repeated terms.
- **`FilePickerOptions`**: Configuration struct for root path, ignored patterns, hidden file visibility, and thread counts.

### Key API Pattern
```rust
use fff_search::file_picker::FilePicker;
use fff_search::{FFFMode, FilePickerOptions, SharedFilePicker, SharedFrecency};

let shared_picker = SharedFilePicker::default();
let shared_frecency = SharedFrecency::default();

let options = FilePickerOptions {
    base_path: root_path.into(),
    mode: FFFMode::Ai,
    ..Default::default()
};

// Initializes background walker and watcher
FilePicker::new_with_shared_state(shared_picker.clone(), shared_frecency.clone(), options)?;

// Querying
if let Some(picker) = shared_picker.read()?.as_ref() {
    let results = picker.fuzzy_search(query, search_options);
}
```

---

## 4. Integration Blueprint for `bl1nk-kept`

### A. `kept-core` (Section 2 & 3 in `TODO.md`)
- Add `fff-search = "0.10"` to workspace dependencies.
- Create `crate/kept-core/src/scanner/fff.rs` as the adapter layer.
- Wrap `FilePicker` to replace legacy recursive `read_dir` in `scanner/scan.rs`.
- Extract file records: relative paths, file size, modified timestamps, binary flag (`is_binary`), and porcelain Git status (`git_status`).

### B. `kept-cli` (Section 3 in `TODO.md`)
- `kept scan`: Generate `ScanIndex` snapshots directly from FFF inventory.
- `kept find`: Utilize FFF's typo-resistant fuzzy ranking and constraint filters.
- `kept review` & `kept duplicates`: Feed candidate file sets from the FFF snapshot into the 4-stage duplicate detection pipeline (`size` → `partial hash` → `full hash` → `evidence`).

### C. `bl1nk-kept-mcp` (Section 4 in `TODO.md`)
- Maintain a single long-lived `FffManager` holding `SharedFilePicker` instances mapped per canonical workspace root.
- Expose MCP tools: `filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, and `filesystem_status`.

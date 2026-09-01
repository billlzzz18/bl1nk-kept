# bl1nk-kept

[ภาษาไทย](README.th.md) · [Specification](SPEC.md) · [Roadmap](TODO.md) · [Research](research/README.md) · [Benchmarks](benchmarks/README.md) · [Schema](schema/README.md)

**bl1nk-kept** is a Rust workspace for inspecting keywords, filesystems, duplicates, and offline documents through one CLI: `kept`. <!-- rumdl-disable-line line-length -->

| Crate | Role |
| --- | --- |
| `kept-core` | Registry model, search, filesystem analysis, duplicate detection, and data foundations |
| `kept-doc` | Offline document conversion through Universal IR |
| `kept-cli` | The `kept` command-line interface |

## Why bl1nk-kept

**One evidence path for names, files, and documents.** `kept` starts with a
keyword registry and a filesystem index, then keeps the signals separate:
lexical similarity is not content equality, and a duplicate claim is backed by
size, partial hash, full hash, and group evidence.

**Built for Thai-aware retrieval without hiding uncertainty.** BM25, Thai
bigrams, synonym compatibility, and configurable n-gram fuzzy retrieval work
together while near matches remain candidates rather than silently becoming
canonical data.

**Offline document work stays inspectable.** Universal IR gives Markdown
conversion a typed target instead of treating documents as opaque text, while
the planned PDF adapter keeps native extraction, page diagnostics, and optional
OCR as distinct stages.

**Defaults must be explainable.** Public corpus provenance, repeated
experiments, raw benchmark artifacts, and a generated public schema turn
implementation choices into inputs that can be inspected and revised.

## Start

```bash
kept setup
kept doctor
kept scan ./workspace
kept review ./workspace
kept find ./workspace --type pdf --min-size 50mb
kept duplicates ./workspace
```

`config.yaml` is user-owned: `kept config` summarizes profiles and scopes, while
`config defaults`, `config profile`, and `config scope` manage it through
task-level commands. `config edit` remains the advanced YAML path.
`kept doctor --fix` recovers a missing or invalid config after preserving a
backup. Use `kept group` to manage registry groups and field schemas. `review`
consumes the config for read-only naming findings; this release has no rename or
apply command.

## Explore

| Need | Start here |
| --- | --- |
| Product boundaries and architecture | [SPEC.md](SPEC.md) |
| Current and completed work | [TODO.md](TODO.md) |
| Contributor workflow | [CONTRIBUTING.md](CONTRIBUTING.md) |
| Research behind implementation decisions | [research/](research/README.md) |
| Reproducible performance comparisons | [benchmarks/](benchmarks/README.md) |
| Public registry document contract | [schema/](schema/README.md) |
| Complete CLI command reference | [get-start.md](get-start.md) |
| Decisions derived from research | [ADR](docs/adr/) |
| Open decisions before implementation | [plan.md](plan.md) |

## License

MIT

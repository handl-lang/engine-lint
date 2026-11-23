# tool-lint

Rule-based linter: evaluates a `lang.lints` table of `LintRuleDef`s plus named
native checks over a parsed module — rules are data, not Rust matches.

A `tool-*` crate: a **tool** engine in the family (not one of the five
operation slots).

```rust
use hnl_tool_lint::LintEngine;

let engine = LintEngine::new();
let diagnostics = engine.lint_module(&lang, &module)?; // rules come from `lang.lints`
```

`LintCheck::Match { node_kind, when }` covers structural rules;
`LintCheck::Native(name)` covers the built-ins (`builtin.empty-block`,
`builtin.unreachable-code`, `builtin.shadowed-binding`,
`builtin.unused-binding`).

## Development

```bash
cargo run --manifest-path xtask/Cargo.toml -- gate   # fmt → clippy → test → doc
```

The repo's `xtask/` crate provides the gate tasks (`fmt`, `clippy`, `test`, `doc`,
`check`, `gate`); `ci.sh` is the gate entry. Deps resolve from git, so no
sibling checkouts are needed; a plain clone test-runs as-is.

## License

MIT.

## Documentation

<https://handl-lang.github.io/docs/tool-lint/>

# engine-lint

Rule-based linter: evaluates a `lang.lints` table of `LintRuleDef`s plus named
native checks over a parsed module — rules are data, not Rust matches.

```rust
use hnl_engine_lint::LintEngine;

let engine = LintEngine::new();
let diagnostics = engine.lint_module(&lang, &module)?; // rules come from `lang.lints`
```

Part of the `engine-*` family (see `docs/engines`): a **tool**, not one of the
five operation slots. `LintCheck::Match { node_kind, when }` covers structural
rules; `LintCheck::Native(name)` covers the built-ins (`builtin.empty-block`,
`builtin.unreachable-code`, `builtin.shadowed-binding`, `builtin.unused-binding`).

## Docs

- [Reference](https://handl-lang.github.io/docs/engine-lint/reference/) — public API
- [How-to](https://handl-lang.github.io/docs/engine-lint/how-to/) — task recipes
- [Explanation](https://handl-lang.github.io/docs/engine-lint/explanation/) — design

## Development

```bash
cargo test     # deps resolve from git; no sibling checkouts needed
```

`ci.sh` is the gate: `fmt --check` → `clippy -D warnings` → `test` → `doc -D warnings`.

## License

MIT.

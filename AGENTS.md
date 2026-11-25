# Agent guide — tool-lint

`hnl-tool-lint` — rule-based linter: evaluates a `lang.lints` table of
`LintRuleDef`s plus named native checks over a parsed module — rules
are data, not Rust matches. `publish = false` (git-install only).
Rust 2024.

## Layout

| path | role |
| --- | --- |
| `src/lib.rs` | crate docs + public API |
| `src/module.rs` | module-level lint orchestration |
| `src/native.rs` | named native checks |
| `src/fix.rs` | lint fixits/rewrites |
| `docs/` | linter doc |
| `xtask/`, `workflows/gate.yaml`, `bupha.toml`, `.cargo/` | vendored repo gate + CI wiring |

## Contract

- Deps resolve from git — `handl-language` → `handl-lang/kernel`,
  `handl-plugin-ext-diag`/`hnl-tool-lex`/`hnl-tool-parse` → sibling
  repos, `branch = "main"`.
- New lint rules belong in `lang.lints` data (`LintRuleDef`); only
  genuinely procedural checks go in `native.rs` — keep the split.

## Verification

```bash
cargo xtask gate   # fmt --check → clippy -D warnings → test → doc
```


<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

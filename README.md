# engine-lint

`hnl-engine-lint` — The data-driven AST lint engine (ADR-0044 §3). Walks an [`Ast`] using [`handl_language::Language`] data to produce expression-level diagnostics.

Sibling-repo deps are git dependencies (`branch = "main"`); the `[patch]`
tables resolve them from sibling checkouts for local development.

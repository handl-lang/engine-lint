# engine-lint

A rule-based linter over a parsed module: `lang.lints` rows plus named native checks.

## What it is

A **tool** in the `engine-*` family. Lint rules are language data. The engine supplies the evaluation strategy and a small set of native checks (scope, shadowing, reachability) the declarative vocabulary cannot yet express.

## Start here

- **Reference** — [public API](/engine-lint/reference/)
- **How-to** — [task recipes](/engine-lint/how-to/)
- **Explanation** — [design and rationale](/engine-lint/explanation/)

No tutorials: this repo is a building block used from Rust. End-to-end
walkthroughs live with the product that composes it — see the
[Handl platform](/handl/).

# Explanation

## Why this crate exists

Lint rules are language data. The engine supplies the evaluation strategy and a small set of native checks (scope, shadowing, reachability) the declarative vocabulary cannot yet express.

## Where it sits

This crate belongs to the `engine-*` family. Plugins, kits and hosts reach it
through the kit's slot, so it can be swapped without touching a language
definition.

See the [reference](/engine-lint/reference/) for the API and the
[how-to guides](/engine-lint/how-to/) for tasks.

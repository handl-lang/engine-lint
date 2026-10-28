# How-to guides

Task recipes for `engine-lint`.

## Run the gate

```bash
./ci.sh          # fmt -> clippy -D warnings -> test -> doc -D warnings
./ci.sh test     # one stage
```

## Use it from Rust

```bash
cargo add --git https://github.com/handl-lang/engine-lint --branch main
```

See the [reference](/engine-lint/reference/) for the types involved and the
[explanation](/engine-lint/explanation/) for why the crate is shaped this way.

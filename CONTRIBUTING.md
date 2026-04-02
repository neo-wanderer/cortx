# Contributing to cortx

Thanks for your interest in contributing!

## Getting Started

```bash
git clone https://github.com/neo-wanderer/cortx.git
cd cortx
cargo build
cargo test
```

## Before Submitting a PR

Please make sure the following all pass:

```bash
cargo fmt --all -- --check
cargo clippy -- -W clippy::all
cargo test
```

## What Makes a Good PR

- **Focused** — one logical change per PR
- **Tested** — new behavior should have tests
- **Descriptive** — clear commit messages using [conventional commits](https://www.conventionalcommits.org/) (e.g., `feat:`, `fix:`, `docs:`)

## Reporting Issues

Found a bug or have a feature request? Please [open an issue](https://github.com/neo-wanderer/cortx/issues).

## License

By contributing, you agree that your contributions will be dual-licensed under the [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE) licenses.

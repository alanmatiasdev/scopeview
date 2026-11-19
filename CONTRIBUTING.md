# Contributing to ScopeView

Thanks for your interest. Bug reports, translations, documentation and code are all welcome.
For anything larger than a small fix, please open an issue first so we can agree on the approach.

## Setting up

You need the latest stable [Rust](https://www.rust-lang.org/tools/install). On Linux you also need
`libclang` (Arch: `clang`, Debian/Ubuntu: `libclang-dev`) plus `libxkbcommon-dev` and
`libwayland-dev` on Debian/Ubuntu.

```sh
git clone https://github.com/alanmatiasdev/scopeview
cd scopeview
cargo run -p scopeview --release
```

## Before you open a pull request

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI runs the same checks on Linux, Windows and macOS. Tests that need a camera are ignored by
default (`cargo test -p scopeview-camera --test hardware -- --ignored --test-threads=1`).

## Guidelines

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) with the
  crate as scope: `feat(core): ...`, `fix(camera): ...`, `docs: ...`.
- Measurement math belongs in `scopeview-core` with unit tests. Keep UI code thin.
- Every user-visible string goes through `tr!("key")` and needs an entry in all six files under
  `crates/i18n/locales/`. A test fails when the catalogs drift apart.
- Behavior changes should update the matching document in `docs/specs/`. Significant decisions get
  an ADR in `docs/adr/`.
- Camera decoding must stay pure Rust: do not enable nokhwa's `decoding` feature.

## Translations

The translations were written without review by native speakers. Corrections are very welcome:
edit `crates/i18n/locales/<language>/main.ftl` and open a pull request.

## License

By contributing you agree that your work is released under the project's license,
GPL-3.0-or-later.

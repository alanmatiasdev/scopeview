# Releasing

Releases are built and published by `.github/workflows/release.yml` when a tag `vX.Y.Z` is pushed.

1. Pick the version (Semantic Versioning) and set it in `[workspace.package]` in `Cargo.toml`.
   Run `cargo check` so `Cargo.lock` follows.
2. Add a `## [X.Y.Z] - YYYY-MM-DD` section to `CHANGELOG.md`. Its text becomes the release notes.
3. Update the `<release>` entry in `packaging/linux/*.metainfo.xml` and run
   `appstreamcli validate --no-net packaging/linux/*.metainfo.xml` (CI runs on Ubuntu 22.04, whose
   validator is too old for the current metadata format, so this is a local check).
4. Make sure `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
   and `cargo test --workspace` pass, and commit.
5. Create a signed tag and push it:

   ```sh
   git tag -s vX.Y.Z -m "ScopeView X.Y.Z"
   git push origin main vX.Y.Z
   ```

The workflow checks that the tag, `Cargo.toml` and `CHANGELOG.md` agree, builds the Linux
(x86_64, aarch64), Windows and macOS artifacts, writes `SHA256SUMS` and creates the release.

## Artifacts

| Platform | Files |
| --- | --- |
| Linux | `scopeview-X.Y.Z-<arch>-linux.tar.gz` (with `install.sh`), `.deb`, `.rpm` |
| Windows | `scopeview-X.Y.Z-setup.exe`, `scopeview-X.Y.Z-x86_64-windows.zip` |
| macOS | `ScopeView-X.Y.Z-macos-universal.dmg` |

Windows and macOS files are not code signed; see ADR-0003.

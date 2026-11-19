# ADR-0003: Distribution channels

- Status: accepted
- Date: 2026-11-19

## Context

ScopeView ships as one binary per OS. Users should be able to download something and run it,
without a Rust toolchain.

## Options considered

- **cargo-dist**: little configuration, but it does not produce `.deb`, `.rpm`, `.dmg` or a
  Windows installer, so we would need extra steps anyway.
- **A hand-written release workflow** on GitHub Actions that builds each artifact with the
  native tool of its platform (`cargo-deb`, `cargo-generate-rpm`, Inno Setup, `hdiutil`).

## Decision

Version 1.0 is published on **GitHub Releases only**, built by `.github/workflows/release.yml`
when a `v*` tag is pushed:

- Linux (x86_64 and aarch64): `.tar.gz` (with an install script, also used on Arch-based
  distributions), `.deb` and `.rpm`.
- Windows (x86_64): installer (`setup.exe`) and a portable `.zip`.
- macOS (Apple silicon and Intel in one universal binary): `.dmg`.
- A `SHA256SUMS` file for everything.

Tags are signed. Package managers (winget, Scoop, Homebrew, AUR, Flathub, Snap) and the
Microsoft Store are possible later additions and are not part of 1.0. Not targeted: the Mac App
Store (paid program, sandbox, GPL friction), Android and iOS.

## Consequences

- Windows and macOS builds are not code signed. Windows shows a SmartScreen warning and macOS
  needs a right-click > Open on first launch. Signing can be added when certificates exist.
- macOS bundles carry `NSCameraUsageDescription` so the system can ask for camera access.
- Flatpak would need `--device=all` for camera access and Snap the `camera` interface.

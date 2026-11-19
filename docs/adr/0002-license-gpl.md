# ADR-0002: GPL-3.0-or-later

- Status: accepted
- Date: 2026-11-19

## Context

The project is open source and will be distributed through many channels.

## Options considered

- **GPL-3.0-or-later**: keeps forks open; accepted by Flathub, Snap, winget, Homebrew, AUR.
  Friction with the Apple App Store terms.
- **MIT OR Apache-2.0**: no store friction anywhere.
- **MPL-2.0**: file-level copyleft.

## Decision

GPL-3.0-or-later. The Mac App Store is not a launch target.

## Consequences

All dependencies must be GPL-compatible (check with `cargo-deny`). Contributions are
accepted under the same license.

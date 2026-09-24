# Dependency licences and security

This records the licence of every direct dependency of DiskClearance. Each entry
was verified against the downloaded package or crate metadata, not recalled from
memory, on 2026-09-24.

This repository is MIT-licensed. Every direct dependency is MIT, Apache-2.0, or
dual MIT/Apache-2.0; none is GPL, AGPL, or source-available non-compete. A new
dependency whose licence is incompatible with MIT is rejected.

Verification method:

- Rust: `cargo metadata --manifest-path src-tauri/Cargo.toml --format-version 1`
  for each direct dependency's `license` and version, with versions confirmed
  against `src-tauri/Cargo.lock`.
- npm: `node -e "require('./node_modules/<package>/package.json').license"` for
  each package, with versions confirmed against `package-lock.json`.

## Rust direct dependencies

| Package     | Version | Licence           | Licence source           |
| ----------- | ------- | ----------------- | ------------------------ |
| tauri       | 2.11.5  | Apache-2.0 OR MIT | cargo metadata `license` |
| tauri-build | 2.6.3   | Apache-2.0 OR MIT | cargo metadata `license` |
| serde       | 1.0.229 | MIT OR Apache-2.0 | cargo metadata `license` |
| serde_json  | 1.0.151 | MIT OR Apache-2.0 | cargo metadata `license` |
| rusqlite    | 0.37.0  | MIT               | cargo metadata `license` |
| ts-rs       | 12.0.1  | MIT               | cargo metadata `license` |

## npm direct dependencies

| Package              | Version | Licence           | Licence source         |
| -------------------- | ------- | ----------------- | ---------------------- |
| react                | 19.3.0  | MIT               | package.json `license` |
| react-dom            | 19.3.0  | MIT               | package.json `license` |
| @tauri-apps/api      | 2.11.1  | Apache-2.0 OR MIT | package.json `license` |
| @types/react         | 19.3.0  | MIT               | package.json `license` |
| @types/react-dom     | 19.3.0  | MIT               | package.json `license` |
| @vitejs/plugin-react | 6.1.1   | MIT               | package.json `license` |
| prettier             | 3.9.6   | MIT               | package.json `license` |
| typescript           | 6.0.3   | Apache-2.0        | package.json `license` |
| vite                 | 8.3.0   | MIT               | package.json `license` |
| @tauri-apps/cli      | 2.11.4  | Apache-2.0 OR MIT | package.json `license` |
| vitest               | 5.0.0   | MIT               | package.json `license` |

## Bundled SQLite, and why the audit job is load-bearing

`rusqlite` is built with the `bundled` feature. SQLite is therefore compiled
from the amalgamated C source into this binary; the copy of SQLite shipped by
the host operating system is never used. A SQLite security fix reaches
DiskClearance only when this project bumps `libsqlite3-sys` (the crate that
vendors the amalgamation), not when the operating system updates its own
SQLite. That makes the `cargo audit` step in
`.github/workflows/validate.yml` load-bearing: it is the mechanism that surfaces
a new SQLite advisory against the vendored version, and a dependency bump is the
only way to take the fix.

## Automated advisory checks

- `npm audit --audit-level=moderate` runs on every pull request. At the time of
  writing it reports no vulnerabilities.
- `cargo audit` runs on every pull request. At the time of writing it reports no
  vulnerabilities, and the following warnings, which are accepted transitive
  risks rather than direct dependencies:

  | Advisory          | Crate              | Version | Kind         |
  | ----------------- | ------------------ | ------- | ------------ |
  | RUSTSEC-2024-0370 | proc-macro-error   | 1.0.4   | unmaintained |
  | RUSTSEC-2025-0081 | unic-char-property | 0.9.0   | unmaintained |
  | RUSTSEC-2025-0075 | unic-char-range    | 0.9.0   | unmaintained |
  | RUSTSEC-2025-0080 | unic-common        | 0.9.0   | unmaintained |
  | RUSTSEC-2025-0100 | unic-ucd-ident     | 0.9.0   | unmaintained |
  | RUSTSEC-2025-0098 | unic-ucd-version   | 0.9.0   | unmaintained |
  | RUSTSEC-2024-0429 | glib               | 0.18.5  | unsound      |

  These arrive through Tauri's dependency graph (the `unic` and `glib` crates via
  the Linux/GTK stack, `proc-macro-error` via an older macro crate). No fixed
  version is available within the current dependency ranges. The cargo audit job
  will fail if a vulnerability, rather than an unmaintained or unsound warning,
  appears.

# DiskClearance

**See what goes. Keep what matters.**

DiskClearance is an open-source, safety-first desktop application for understanding and reclaiming storage. It is designed around explainable findings, review before action, and an honest distinction between moving files to Trash and deleting them permanently.

> **Project status:** pre-implementation foundation. The application shell builds, but scanning, cleanup, duplicate detection, application removal, recovery, and Windows support are planned for v1.0 and are not implemented yet.

## Planned for v1.0

- A cancellable Mac storage scan with partial, permission-limited results.
- Safe cleanup suggestions backed by visible evidence and risk classes.
- Large-file and folder exploration with list and treemap views.
- Exact-content duplicate groups with conservative selection.
- Application removal with explained related-file matches.
- Dedicated developer-tool and local AI-model storage views.
- Review plans that are revalidated immediately before execution.
- Move to Trash by default, optional irreversible deletion, and truthful operation history.

The first supported platform will be macOS 13 Ventura and later, on **Apple Silicon only** — Intel Macs are not supported in v1.0. The architecture keeps platform behavior isolated so Windows support can be added after the macOS v1.0 is complete.

**Released builds are unsigned and not notarized.** Code signing is planned, not implemented. Until it is done, macOS will refuse to open a downloaded build on a double-click; use right-click → Open the first time, or clear the quarantine attribute. Nothing here is signed yet and this note stays until something is.

## Development

Requirements:

- Node.js 22 or later (declared as `engines.node` in `package.json`)
- Rust 1.88 or later, the minimum pinned in `rust-toolchain.toml`
- macOS or Windows system dependencies required by Tauri 2

```bash
npm install
npm run check
npm run tauri dev
```

The repository currently contains only the validated project foundation. See [the specification](docs/SPEC.md), [architecture](docs/ARCHITECTURE.md), and [roadmap](docs/ROADMAP.md) before contributing.

## Safety

DiskClearance handles destructive filesystem operations. Security or data-loss issues should not be filed publicly; follow [SECURITY.md](SECURITY.md). No release should be trusted with real files until the release checklist and destructive-operation test suite pass.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md). The project is licensed under the [MIT License](LICENSE).

Direct dependencies and their licences are recorded in [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md). CycloneDX SBOMs for both ecosystems are generated from the committed lockfiles by `scripts/generate-sbom.sh`; nothing is attached to a release until the release checklist is satisfied.

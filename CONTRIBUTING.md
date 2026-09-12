# Contributing

Thank you for helping make destructive storage operations safer and easier to understand.

## Before starting

Open an issue before substantial product, architecture, or safety changes. Small fixes may go directly to a pull request. Read `AGENTS.md` and the documents under `docs/`.

## Local checks

```bash
npm install
npm run check
```

For desktop development:

```bash
npm run tauri dev
```

## Pull-request expectations

- Explain the problem and the behavior changed.
- Include tests for Rust safety logic and UI behavior.
- Include before/after screenshots for interface changes.
- Keep public claims truthful about what is implemented.
- Do not use real personal file paths or destructive tests against a developer's home directory. Use isolated temporary fixtures.

By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

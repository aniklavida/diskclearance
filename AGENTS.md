# Contributor agent guide

These instructions apply to humans and coding agents working in this repository.

## Start here

1. Read `docs/SPEC.md`, `docs/ARCHITECTURE.md`, and `docs/ROADMAP.md`.
2. Run `npm install`, then `npm run check`.
3. Keep destructive filesystem behavior in Rust. React may request actions and render results; it must never perform filesystem mutation.

## Engineering rules

- Treat paths, permissions, symlinks, mount boundaries, and file identity as hostile inputs.
- A scan is read-only. No action may occur without a reviewed plan.
- Re-resolve and revalidate each target immediately before an operation.
- Never select Review or Protected items automatically.
- Do not claim recovery after permanent deletion.
- Platform behavior belongs behind the platform adapter.
- Add tests before changing safety classes, deletion planning, path protection, or history semantics.
- Keep UI copy calm, precise, and free of fear or inflated savings claims.
- Preserve keyboard access, visible focus, reduced motion, and non-color status cues.

## Commands

```bash
npm run format:check
npm run test
npm run build
npm run rust:fmt
npm run rust:test
npm run rust:check
```

Run `npm run check` before opening a pull request.

## Pull requests

- Keep each change focused and explain user-visible behavior.
- State which truthfulness status applies: implemented and tested, experimental, planned, or unsupported.
- Include tests for behavior changes and screenshots for visible UI changes.
- Never commit secrets, credentials, machine-specific paths, generated build output, or real user file paths.

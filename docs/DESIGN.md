# Design system and brand specification

## Brand decision

DiskClearance is a trust-first macOS system utility for storage reclamation and forensic disk safety. In safety-critical system software, visual calm signals technical competence and reduces user anxiety. The visual identity is built using the project's design system under the Technical Dense and Calm Editorial directions.

- **Vibe words:** calm, exact, trustworthy.
- **Brand hue:** 200 (steel teal family).
- **Chroma:** 0.09.
- **Warmth:** 0.006 (neutrals subtly tinted toward the 200° hue for coherence).
- **Brand lightness:** 0.55 (light mode accent, calibrated to guarantee ≥ 4.5:1 WCAG AA contrast against white surfaces) and 0.68 (dark mode accent).
- **Type pairing:** System UI stack (`-apple-system`, `BlinkMacSystemFont`, `SF Pro Text`, `SF Pro`, `system-ui`, `sans-serif`) paired with System Monospace (`ui-monospace`, `Menlo`, `Monaco`, `SF Mono`, `monospace`) for filesystem paths, rule identifiers, and tabular numerals. Zero font files, zero third-party web font network requests.
- **Appearance:** Follows the host operating system preference (`prefers-color-scheme`). Both light and dark palettes are fully supported without manual toggle friction.

---

## Safety and communication rules

1. **Safety is the product:** The interface never relies on color alone to communicate state. Every safety classification, operational outcome, and risk boundary pairs a distinct textual label with an icon glyph.
2. **Reassurance over alarm:** Protected system components (system frameworks, developer command-line tools, keychains) are styled in a neutral slate tone (`--class-protected-*`) with a lock icon (`🔒`). A protected item is reassurance that the application knows what to leave alone. It is never decorated with danger red or warning alerts.
3. **Red strictly for irreversible destruction:** Red (`--class-irreversible-*`) is exclusively reserved for unrecoverable actions (permanent deletion bypassing the Trash) or genuine fatal system failures.
4. **Accent restraint:** The accent color is budgeted to occupy less than 5% of any screen surface—principally the single primary action button, active navigation selection, focus rings, and active selection checkboxes.
5. **One primary action per screen:** Each screen presents exactly one prominent primary action (`Scan Mac`, `Move to Trash`, etc.). Secondary and exploratory actions remain visually subdued.
6. **No deceptive patterns:** No gradient backgrounds, no colored left borders, no emoji in system chrome, no zebra-striped tables, and no modals for routine forms. The confirmation sheet for irreversible operations is the single documented modal exception.

---

## Structural tokens and reconciliation

Where the project's design system differs from the product's locked structural tokens, the locked values take precedence to protect verified layout contracts:

| Token category     | Locked product value                      | Reconciliation note                                           |
| :----------------- | :---------------------------------------- | :------------------------------------------------------------ |
| **Spacing scale**  | 4, 8, 12, 16, 24, 32, 48 px (`--space-*`) | Fixed 7-step scale preserved without interpolation.           |
| **Control radius** | 8 px (`--radius-control`)                 | Preserved for buttons, text inputs, and select elements.      |
| **Card radius**    | 12 px (`--radius-card`)                   | Preserved for cards, aggregate containers, and treemap tiles. |
| **Surface radius** | 16 px (`--radius-surface`)                | Preserved for modal dialogs and major window containers.      |
| **Pill radius**    | 999 px (`--radius-full`)                  | Preserved for status badges and indicator pills.              |
| **Target size**    | Min 32 × 32 px, primary 40 px             | Interactive controls enforce touch/pointer clearance.         |
| **Focus ring**     | 2 px accent with 2 px offset              | High-visibility focus indicators across keyboard navigation.  |

---

## Semantic color tokens

All color roles are derived from oklch formulas converted to precomputed sRGB hex values to satisfy WCAG AA contrast (≥ 4.5:1 for body copy and interactive components) in both light and dark appearances:

| Role token                | Light     | Dark      | Purpose and contrast notes                                    |
| :------------------------ | :-------- | :-------- | :------------------------------------------------------------ |
| `--window`                | `#f6fbfc` | `#0c1010` | Window canvas surface and backdrop scrim                      |
| `--surface`               | `#f1f6f7` | `#171b1c` | Secondary view canvas, table headers, flat panels             |
| `--surface-raised`        | `#ffffff` | `#212525` | Cards, popovers, elevated surfaces, light button text         |
| `--divider`               | `#dadfdf` | `#2d3131` | Hairline borders, table dividers, panel splitters             |
| `--line`                  | `#dadfdf` | `#2d3131` | Structural lines and list separators                          |
| `--text-primary`          | `#181b1b` | `#e5e9e9` | Primary headlines, entity titles, tabular numerals (≥ 12:1)   |
| `--text-secondary`        | `#5a5f5f` | `#a0a6a6` | Supporting descriptions, metadata, timestamps (≥ 5.9:1)       |
| `--accent`                | `#118186` | `#49a9ae` | Primary action background, clean indicators (≥ 4.5:1 on text) |
| `--accent-soft`           | `#dbf4f6` | `#153335` | Active nav background, selection tint                         |
| `--accent-fg`             | `#00656a` | `#64c3c8` | Active navigation label, selection icons (≥ 5.9:1 on soft)    |
| `--class-rebuildable-bg`  | `#dcf7e1` | `#1a2e1e` | Rebuildable badge background tint                             |
| `--class-rebuildable-fg`  | `#1a5e30` | `#73c385` | Rebuildable badge text and glyph (≥ 6.7:1)                    |
| `--class-review-bg`       | `#ffeecd` | `#3b2b0d` | Review badge background tint                                  |
| `--class-review-fg`       | `#6e4900` | `#e6b55d` | Review badge text and glyph (≥ 7.0:1)                         |
| `--class-protected-bg`    | `#d3d9d9` | `#2f3434` | Protected badge background tint                               |
| `--class-protected-fg`    | `#181b1b` | `#cbcece` | Protected badge text and lock glyph (≥ 7.9:1)                 |
| `--class-irreversible-bg` | `#ffe5e1` | `#442321` | Irreversible deletion modal background                        |
| `--class-irreversible-fg` | `#9e2523` | `#f07f77` | Irreversible deletion warning text and action (≥ 5.3:1)       |

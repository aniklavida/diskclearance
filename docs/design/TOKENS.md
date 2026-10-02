# Design tokens

## Principles

Use semantic tokens, support light and dark appearance together, and reserve red for irreversible actions or genuine failure.

## Foundation

- Font: system UI stack (`-apple-system`, `BlinkMacSystemFont`, `SF Pro Text`, `SF Pro`, `system-ui`, `sans-serif`); system monospace (`ui-monospace`, `Menlo`, `Monaco`, `SF Mono`, `monospace`) for paths and tabular numerals.
- Spacing: 4, 8, 12, 16, 24, 32, 48 px.
- Radius: 8 px for controls, 12 px for cards, 16 px for major surfaces, 999 px for full pills.
- Minimum target: 32 × 32 px; primary controls 40 px or taller.
- Focus ring: 2 px accent plus 2 px offset.

## Semantic colour roles

| Token                     | Light     | Dark      | Role                                                 |
| :------------------------ | :-------- | :-------- | :--------------------------------------------------- |
| `--window`                | `#f6fbfc` | `#0c1010` | Window background canvas, modal backdrop scrim       |
| `--surface`               | `#f1f6f7` | `#171b1c` | List background, table header, view surface          |
| `--surface-raised`        | `#ffffff` | `#212525` | Cards, popovers, callout bodies, button text (light) |
| `--divider`               | `#dadfdf` | `#2d3131` | Border lines, card dividers, table separators        |
| `--text-primary`          | `#181b1b` | `#e5e9e9` | Primary headlines, item names, tabular numerals      |
| `--text-secondary`        | `#5a5f5f` | `#a0a6a6` | Explanatory copy, secondary metadata, notes          |
| `--accent`                | `#118186` | `#49a9ae` | Primary actions, clean disk status icon, focus ring  |
| `--accent-soft`           | `#dbf4f6` | `#153335` | Informational callouts, active navigation fill       |
| `--accent-fg`             | `#00656a` | `#64c3c8` | High-contrast accent labels on soft backgrounds      |
| `--class-rebuildable-bg`  | `#dcf7e1` | `#1a2e1e` | Rebuildable badge background                         |
| `--class-rebuildable-fg`  | `#1a5e30` | `#73c385` | Rebuildable badge text and glyph                     |
| `--class-review-bg`       | `#ffeecd` | `#3b2b0d` | Review badge, warning callout background             |
| `--class-review-fg`       | `#6e4900` | `#e6b55d` | Review badge, warning callout text and glyph         |
| `--class-protected-bg`    | `#d3d9d9` | `#2f3434` | Protected lock background, standard access badge     |
| `--class-protected-fg`    | `#181b1b` | `#cbcece` | Protected lock icon and text                         |
| `--class-irreversible-bg` | `#ffe5e1` | `#442321` | Fatal error banner background, unrecoverable action  |
| `--class-irreversible-fg` | `#9e2523` | `#f07f77` | Fatal error banner text, unrecoverable action text   |

Exact values are implemented as CSS custom properties and must satisfy WCAG AA contrast in both appearances. Red is strictly reserved for irreversible operations; Protected items remain a calm, neutral slate reassurance.

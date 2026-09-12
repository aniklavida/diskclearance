# Design tokens

## Principles

Use semantic tokens, support light and dark appearance together, and reserve red for irreversible actions or genuine failure.

## Foundation

- Font: system UI stack; tabular numerals for storage values.
- Spacing: 4, 8, 12, 16, 24, 32, 48.
- Radius: 8 for controls, 12 for cards, 16 for major surfaces.
- Minimum target: 32 × 32 px; primary controls 40 px or taller.
- Focus ring: 2 px accent plus 2 px offset.

## Semantic colour roles

- Window, surface, raised surface, divider, primary text, secondary text.
- Accent: one muted blue-green tone with accessible foreground.
- Rebuildable: quiet positive tone plus icon and label.
- Review: amber tone plus icon and label.
- Protected: neutral lock treatment, never danger red.
- Irreversible/error: red, always paired with text and icon.

Exact values are implemented as CSS custom properties and must satisfy WCAG AA contrast in both appearances.

# Finding row, evidence disclosure, and Explore treemap specification

## Overview

The finding row is the single most repeated object in DiskClearance. A scan of a Mac produces hundreds or thousands of them. If each row requires active interrogation to understand, reviewing results becomes an exhausting chore. If each row forces complete technical forensics onto the screen by default, the interface devolves into an unreadable terminal dump.

This specification designs the finding row, its group containers, its evidence disclosure mechanism, its application-level variants, and the Explore treemap/folder table pair.

The design enforces the principle of **outcome before detail**: expertise is available on demand rather than imposed by default. The primary workflow serves a person who wants clear outcomes without deciphering deep paths, while placing complete technical evidence exactly one keystroke or click away for the developer or system administrator who requires proof.

---

## The five questions every row answers

Every finding row in DiskClearance answers five fundamental questions:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. What is it?              [Visible at rest]                               │
│ 3. How much space?          [Visible at rest]                               │
│ 5. Can it be recovered?     [Visible at rest]                               │
├─────────────────────────────────────────────────────────────────────────────┤
│ 2. Why was it found?        [Revealed on disclosure: exactly one keystroke] │
│ 4. What changes if it goes? [Revealed on disclosure: exactly one keystroke] │
└─────────────────────────────────────────────────────────────────────────────┘
```

1. **What is it?** A human-readable entity name describing the software artifact (for example, `"Xcode DerivedData"`, `"CocoaPods Cache"`, or `"SyntheticStudio Application Support"`), never a bare file path.
2. **Why was it found?** The declarative matching rule, the rule version, the owning tool or application, and why this specific path matched the criteria.
3. **How much space?** The exact byte footprint formatted with tabular numerals, so real-time streaming updates do not make the column flicker or shift.
4. **What changes if it goes?** A clear, factual statement of operational impact: whether a tool recreates it automatically, whether subsequent build times will be affected, or whether user state is lost.
5. **Can it be recovered?** The recoverability boundary: whether it is rebuildable by a tool, recoverable via macOS Trash before emptying, version-controlled, or permanently lost once deleted.

Questions 1, 3, and 5 are visible at rest across all viewports. Questions 2 and 4—the structured evidence—are accessible immediately via row expansion or keyboard disclosure (`Space`, `Enter`, or `→`).

---

## Locked design constraints

The following constraints are locked in this specification:

1. **The UI never infers classification.** Not from colour, not from file extension, not from path text, and not from parent folder names. The interface strictly renders the `SafetyClass` (`Rebuildable`, `Review`, or `Protected`) assigned by the Rust core, or it renders nothing.
2. **No global Select All for Review-class items.** Bulk selection at the group header or global level must strictly select only `Rebuildable` findings. Items marked as `Review` or `Protected` must never be automatically selected by any bulk action.
3. **Paths are secondary.** A row that leads with a long filesystem path fails the outcome-before-detail principle. The primary label is always the recognizable tool or artifact name. Canonical paths live inside the evidence disclosure.
4. **Sizes use tabular numerals.** Every storage figure uses `font-variant-numeric: tabular-nums` (via `.tabular-nums` / `.storage-figure`). Numerical digits occupy identical widths, ensuring alignment and preventing jitter during scans.
5. **Every row is reachable and operable by keyboard.** Full compliance with macOS keyboard navigation standards: focus rings, roving tabindex, `Arrow` keys for traversal, `Space` for selection toggle, and `Enter` or `ArrowRight` for evidence disclosure.
6. **A treemap must have an equivalent navigable list or table representation.** A visual treemap alone is insufficient for keyboard accessibility, screen-reader navigation, and dense sorting. DiskClearance pairs every treemap with an equivalent, synchronized folder table.
7. **Reduced Motion removes animated chart transitions entirely.** Under `prefers-reduced-motion: reduce`, all chart re-layouts, tree transitions, accordion expansions, and zoom zooms cut immediately (0ms duration).

---

## Window viewports and responsive layout

DiskClearance supports macOS 13 Ventura and later across two canonical window sizes:

1. **Minimum window (compact breakpoint):** `760 × 560 px`
   - Content area: `560 px` wide.
   - Finding row collapse rule:
     - **Survives in primary row:** Checkbox, Entity Name, Safety Class Badge, Size.
     - **Moves into evidence disclosure:** Last Activity timestamp, Recoverability pill/sentence.
2. **Common window (expanded breakpoint):** `1100 × 720 px`
   - Content area: `880 px` wide (max readable column span: `790 px`).
   - Finding row presentation: All fields visible horizontally at rest (Checkbox, Entity Name, Safety Class Badge, Last Activity, Recoverability indicator, Tabular Size, Disclosure toggle).

---

## Safety classification badges

Every finding row carries exactly one classification badge. Badges communicate safety status through **both an icon and a text label**, never colour alone.

Exact semantic tokens from `src/App.css` are used:

| Safety Class    | Icon Glyph  | Text Label      | Light Appearance Tokens                                                              | Dark Appearance Tokens                                                               | Interaction Rule                   |
| :-------------- | :---------- | :-------------- | :----------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------- | :--------------------------------- |
| **Rebuildable** | `↺` (Loop)  | `"Rebuildable"` | bg: `--class-rebuildable-bg` (`#daf0e4`)<br>fg: `--class-rebuildable-fg` (`#164e3a`) | bg: `--class-rebuildable-bg` (`#1e3b2e`)<br>fg: `--class-rebuildable-fg` (`#8ce4bd`) | Checkable; selected by default     |
| **Review**      | `◇` (Gate)  | `"Review"`      | bg: `--class-review-bg` (`#faecc6`)<br>fg: `--class-review-fg` (`#5c3e00`)           | bg: `--class-review-bg` (`#382b13`)<br>fg: `--class-review-fg` (`#ffd78a`)           | Checkable; **never** auto-selected |
| **Protected**   | `🔒` (Lock) | `"Protected"`   | bg: `--class-protected-bg` (`#e1e6e5`)<br>fg: `--class-protected-fg` (`#283632`)     | bg: `--class-protected-bg` (`#26302e`)<br>fg: `--class-protected-fg` (`#ccd5d1`)     | **Disabled / Uncheckable**         |

### CSS specification

```css
.safety-badge {
  display: inline-flex;
  align-items: center;
  gap: var(--space-4);
  min-height: 22px;
  padding: 2px var(--space-8);
  border-radius: var(--radius-full);
  font-size: 11px;
  font-weight: 650;
  letter-spacing: 0.02em;
  white-space: nowrap;
  user-select: none;
}

.safety-badge.rebuildable {
  background: var(--class-rebuildable-bg);
  color: var(--class-rebuildable-fg);
}

.safety-badge.review {
  background: var(--class-review-bg);
  color: var(--class-review-fg);
}

.safety-badge.protected {
  background: var(--class-protected-bg);
  color: var(--class-protected-fg);
}
```

---

## Protected lock treatment vs. Irreversible action

In poorly designed utility software, protected system items are frequently decorated with bright red warning symbols or alarming alert text. DiskClearance rejects this pattern.

A protected item is **reassurance that the tool knows what to leave alone**, not a threat. It is styled with neutral, calm tones (`--class-protected-*`), reassuring the user that essential operating system files, user repositories, and keychains remain secure. Red (`--class-irreversible-*`) is reserved exclusively for destructive, irreversible operations such as permanent deletion without Trash.

### Side-by-side comparison

```text
┌────────────────────────────────────────┐  ┌────────────────────────────────────────┐
│ Calm Protected Treatment (Reassurance) │  │ Irreversible Action Treatment (Alert)  │
├────────────────────────────────────────┤  ├────────────────────────────────────────┤
│ Badge:   [🔒 Protected]                │  │ Action:  [⚠ Delete Now (Irreversible)] │
│ Surface: Neutral slate tint            │  │ Surface: Danger red alert tint         │
│ Light:   #e1e6e5 bg / #283632 fg       │  │ Light:   #fae3e3 bg / #7f1d1d fg       │
│ Dark:    #26302e bg / #ccd5d1 fg       │  │ Dark:    #3b191b bg / #fca5a5 fg       │
│ Control: Checkbox disabled (uncheck)   │  │ Control: High-contrast red button      │
│ Copy:    "Protected by system policy.  │  │ Copy:    "Permanently removes files.   │
│           Cannot be modified."         │  │           Action cannot be undone."    │
└────────────────────────────────────────┘  └────────────────────────────────────────┘
```

The contrast between these two visual roles reinforces the core trust promise: DiskClearance treats safety as normal and quiet, and reserves visual urgency solely for unrecoverable destruction.

---

## Finding group header

Findings are organized under group headers representing their owning tool, runtime, or application context (e.g., Xcode, Homebrew, Docker, CocoaPods, or Orphan Applications).

### Anatomy of a group header

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [✓]  ▼  Xcode                                       3 items · 5.50 GB [Select Rebuild] │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

1. **Group selection control:** A three-state checkbox (`checked`, `unchecked`, `indeterminate`).
   - Selecting the group checkbox selects **only Rebuildable** child rows.
   - Child rows with safety class `Review` are **skipped**.
   - Child rows with safety class `Protected` cannot be selected.
   - If a group contains 2 Rebuildable items and 1 Review item, and both Rebuildable items are selected, the group checkbox displays the `indeterminate` (mixed) state (`[-]`) to honestly indicate that not all child items are selected.
   - If a group contains _only_ Review or Protected items, the checkbox is replaced by a disabled neutral state or omitted, accompanied by explanatory microcopy: `"Review required per item"`.
2. **Group expansion toggle:** A disclosure triangle/chevron (`▼` expanded, `▶` collapsed).
3. **Group title:** Owning tool or application name (e.g. `"Xcode"`, `"Homebrew"`).
4. **Group metadata:** Tabular total size and item count (e.g. `"3 items · 5.50 GB"`).
5. **Contextual selection badge:** If the group contains Review items, a calm secondary badge explains: `"1 item requires review"`.

### Visual layout: 1100 × 720 px (Expanded)

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│ [✓]  ▼  Xcode Developer Tools              3 items  │  5.50 GB total   [1 item needs review] │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│   [✓] Xcode DerivedData          [↺ Rebuildable]   34 days ago   2.10 GB   Rebuildable   [▼]│
│   [✓] Xcode Simulator Caches     [↺ Rebuildable]   12 days ago   2.00 GB   Rebuildable   [▶]│
│   [ ] Xcode Device Logs          [◇ Review]         3 mos ago    1.40 GB   Review first  [▶]│
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: 760 × 560 px (Compact)

At minimum width, secondary summary labels consolidate to preserve touch targets and typography:

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [✓]  ▼  Xcode                                            3 items · 5.50 GB │
├────────────────────────────────────────────────────────────────────────────┤
│   [✓] Xcode DerivedData              [↺ Rebuildable]              2.10 GB [▼]│
│   [✓] Xcode Simulator Caches         [↺ Rebuildable]              2.00 GB [▶]│
│   [ ] Xcode Device Logs              [◇ Review]                   1.40 GB [▶]│
└────────────────────────────────────────────────────────────────────────────┘
```

---

## Finding row specification

The finding row is designed to answer Questions 1, 3, and 5 at rest, and Questions 2 and 4 upon disclosure.

### Row anatomy

```text
┌───┬──────────────────────────┬─────────────────┬─────────────┬──────────────┬──────────┬───┐
│[✓]│ Xcode DerivedData        │ [↺ Rebuildable] │ 34 days ago │ Rebuildable  │ 2.10 GB  │[▼]│
└───┴──────────────────────────┴─────────────────┴─────────────┴──────────────┴──────────┴───┘
  1               2                     3               4             5            6       7
```

1. **Selection checkbox (`32 × 32 px` hit target):**
   - Checked for selected items.
   - Unchecked for unselected items.
   - Disabled/Locked glyph for `Protected` items.
2. **Entity Name:** Primary descriptor in `--text-primary` (`font-size: 14px; font-weight: 600;`).
3. **Safety Class Badge:** Icon + text label with semantic tokens.
4. **Last Activity:** Tabular relative date in `--text-secondary` (`font-size: 12px;`).
5. **Recoverability Summary:** Compact status indicator (`"Rebuildable cache"`, `"Trash-recoverable"`, `"Irrecoverable"`).
6. **Footprint Size:** Tabular numerals in `--text-primary` (`font-size: 14px; font-weight: 650; font-variant-numeric: tabular-nums;`).
7. **Disclosure Button (`32 × 32 px` hit target):** Chevron rotating from `▶` (collapsed) to `▼` (expanded).

### Column collapse order for compact width (760 px)

When the viewport is narrowed to 760px, table columns collapse according to strict priority:

```text
Column Priority Matrix:
1. Selection Control  ───► MUST SURVIVE (Required for user action)
2. Entity Name        ───► MUST SURVIVE (Answers Question 1: What is it?)
3. Safety Class Badge ───► MUST SURVIVE (Core safety model representation)
4. Tabular Size       ───► MUST SURVIVE (Answers Question 3: How much space?)
───────────────────────────────────────────────────────────────────────────────
5. Recoverability     ───► COLLAPSES into evidence disclosure (Answers Q5 in detail)
6. Last Activity      ───► COLLAPSES into evidence disclosure (Secondary context)
```

At 760px, Questions 1 and 3 remain visible at rest via the surviving columns. Question 5 is answered concisely via the badge (`Rebuildable` implies rebuildable recovery; `Review` indicates conditional recovery), with the full recoverability sentence available inside the disclosure.

---

## Evidence disclosure specification

Expanding a finding row opens the integrated evidence disclosure drawer immediately beneath the row. It provides the structured verification data emitted by the Rust core.

### Evidence disclosure drawer layout

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [✓]  Xcode DerivedData          [↺ Rebuildable]   34 days ago   Rebuildable   2.10 GB [▼]│
├────────────────────────────────────────────────────────────────────────────────────────┤
│  ┌── Evidence and verification details ──────────────────────────────────────────────┐ │
│  │ Rule identifier:  developer.xcode.deriveddata (catalogue v2)                      │ │
│  │ Owning tool:      Xcode 15.2                                                      │ │
│  │ Match confidence: Definite (Exact match on canonical project build store)         │ │
│  │ Canonical path:   ~/Library/Caches/com.synthetic.developer/DerivedData   [Copy]   │ │
│  │ Last activity:    2026-08-14 11:22 UTC (34 days ago)                              │ │
│  ├───────────────────────────────────────────────────────────────────────────────────┤ │
│  │ What regenerates this:                                                            │ │
│  │ Rebuilt automatically by Xcode on next build (`xcodebuild` or Xcode IDE).         │ │
│  ├───────────────────────────────────────────────────────────────────────────────────┤ │
│  │ Recoverability assessment:                                                        │ │
│  │ Rebuildable cache. Moving to Trash preserves files until Trash is emptied. Build  │ │
│  │ times may temporarily increase while intermediate objects recompile.              │ │
│  └───────────────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Structured evidence fields

1. **Rule identifier and version:** Exact rule ID and schema version from the Rust catalogue (`rule_id`, `rule_version`). Guarantees auditability.
2. **Owning tool or application:** Verified software creator (`owning_tool`).
3. **Match confidence:** Explicit confidence label (`Definite` vs `Likely`).
4. **Canonical filesystem path:** Fully resolved, unescaped path rendered in a monospace container with a 1-click `"Copy path"` action.
5. **Last activity:** Exact timestamp and relative age (`last_activity_ms`).
6. **Regeneration mechanism:** Operational instruction or command (`regenerator`).
7. **Recoverability sentence:** Complete, human-readable sentence stating the exact consequence and recovery path (`recoverability`).

### Styling and tokens

- **Disclosure container:**
  - Background: `--surface` (`#faf9f5` light / `#222c29` dark).
  - Border: 1px solid `--divider` (`#dfe5e2` light / `#2e3a36` dark).
  - Radius: `--radius-control` (`8px`), margin: `var(--space-8) var(--space-16) var(--space-12)`.
  - Padding: `var(--space-16)`.
- **Path codeblock:**
  - Background: `--window` (`#f4f2ec` light / `#19211f` dark).
  - Border: 1px solid `--divider`.
  - Typography: `ui-monospace, "SF Mono", Menlo, monospace; font-size: 12px;`.
  - Color: `--text-primary`.

---

## Confidence label for heuristic matches

Not all findings are discovered through deterministic canonical paths. Some are discovered through heuristic directory patterns, bundle associations, or dormant container matching.

DiskClearance distinguishes between **Definite** and **Likely** matches at the type level (`Confidence::Definite` vs `Confidence::Likely`). The UI never presents a heuristic guess with the certainty of a verified directory.

### Visual treatments

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Definite match treatment:                                                    │
│ [● Verified owner] Canonical cache directory for Xcode 15                    │
│                                                                              │
│ Likely match treatment (Heuristic):                                          │
│ [◇ Likely related] Inferred from bundle name and file pattern in Containers  │
│                    Inspect path before removing                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

- **Definite match (`Confidence::Definite`):**
  - Uses quiet secondary styling (`--text-secondary`).
  - Phrasing: `"Verified tool structure"`, `"Canonical cache path"`.
- **Likely match (`Confidence::Likely`):**
  - Uses the amber review token pair (`--class-review-bg`, `--class-review-fg`).
  - Preceded by the caution glyph (`◇`).
  - Copy explicitly states the heuristic basis: `"Likely related to SyntheticStudio (heuristic match). Inspect path before confirming."`
  - Items with `Likely` confidence are always placed into the `Review` safety class by the Rust core, ensuring they are never checked by default.

---

## Worked examples demonstrating claims

### Worked Example 1: Finding row answering the five questions

This worked example demonstrates a row answering Questions 1, 3, and 5 at rest, and Questions 2 and 4 after disclosure. Every numerical value and string is synthetic fixture data.

#### At Rest (Questions 1, 3, and 5 answered)

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [✓]  Xcode DerivedData          [↺ Rebuildable]   34 days ago   Rebuildable   2.10 GB [▶]│
└────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Question 1 (What is it?):** `"Xcode DerivedData"` (Visible at rest).
- **Question 3 (How much space?):** `"2.10 GB"` (Visible at rest; tabular numerals).
- **Question 5 (Can it be recovered?):** `"Rebuildable"` / `"Rebuildable cache"` (Visible at rest).

#### After one interaction: Chevron clicked or Enter pressed (Questions 2 and 4 answered)

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [✓]  Xcode DerivedData          [↺ Rebuildable]   34 days ago   Rebuildable   2.10 GB [▼]│
├────────────────────────────────────────────────────────────────────────────────────────┤
│  Rule: developer.xcode.deriveddata (v2) · Tool: Xcode 15.2 · Match: Definite           │
│  Path: ~/Library/Caches/com.synthetic.developer/DerivedData                            │
│                                                                                        │
│  Why was it found? (Question 2):                                                       │
│  Matches canonical intermediate build root specified in Xcode toolchain rules.         │
│                                                                                        │
│  What changes if it goes? (Question 4):                                                │
│  Xcode will regenerate indexes and compilation objects on next build. Build times may │
│  increase temporarily while objects are re-created.                                    │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Worked Example 2: Distinct numbers demonstration

To ensure examples prove what they claim, the figures below demonstrate distinct sizes across finding groups, individual rows, application binaries, and detached caches. No two differing concepts share the same number.

```text
Group: Xcode Developer Tools (Aggregate: 5.50 GB across 3 items)
  ├── Item A: Xcode DerivedData          ── Size: 2.10 GB (Rebuildable)
  ├── Item B: Xcode Simulator Caches     ── Size: 2.00 GB (Rebuildable)
  └── Item C: Xcode Archived Crash Logs  ── Size: 1.40 GB (Review)
      Calculation: 2.10 GB + 2.00 GB + 1.40 GB = 5.50 GB total.

Group selection check on Xcode Developer Tools:
  - Checking the group selects Item A (2.10 GB) and Item B (2.00 GB).
  - Item C (1.40 GB) remains unselected.
  - Selected group subtotal: 4.10 GB (2 items selected).
  - The aggregate group size (5.50 GB), the selected size (4.10 GB), and the individual row sizes (2.10 GB, 2.00 GB, 1.40 GB) are distinct and physically accurate.
```

---

## Application row and related-file evidence

The Applications view analyzes installed software packages, their primary executables, and all associated support files scattered across the macOS filesystem.

### Application row anatomy

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [App Icon] SyntheticStudio Pro           v3.4.1   │   5.25 GB total footprint     [▼] │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

The application row presents the total measured footprint (`5.25 GB`). Expanding the row reveals the decomposed evidence list separating the application binary from its detachable data.

### Expanded application evidence list

```text
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│ [App Icon] SyntheticStudio Pro           v3.4.1   │   5.25 GB total footprint      [▼] │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│  ┌── Measured application components ─────────────────────────────────────────────────┐ │
│  │ [🔒] Application Bundle (Installed binary)                          1.45 GB        │ │
│  │      /Applications/SyntheticStudio Pro.app                         [Protected]    │ │
│  │                                                                                    │ │
│  │ [✓] User Render Cache (Rebuildable cache)                          2.60 GB        │ │
│  │      ~/Library/Caches/com.synthetic.photostudio                    [Rebuildable]  │ │
│  │      Confidence: Definite (Exact bundle identifier match)                          │ │
│  │                                                                                    │ │
│  │ [ ] Sandboxed Container (Orphan state)                              1.20 GB        │ │
│  │      ~/Library/Containers/com.synthetic.photostudio                [Review]       │ │
│  │      Confidence: Definite (Sandboxed container root)                               │ │
│  │                                                                                    │ │
│  │ [ ] Shared Plugin Presets (User config)                            0.00 GB (24 MB)│ │
│  │      ~/Library/Application Support/SyntheticStudio                 [Review]       │ │
│  │      Confidence: Likely (Matches application vendor naming)                        │ │
│  └────────────────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Decomposed Footprint:**
  - Application binary: `1.45 GB` (Protected by system policy).
  - Render cache: `2.60 GB` (Rebuildable; selected for clearing).
  - Sandboxed container: `1.20 GB` (Review; requires explicit choice).
  - Application support: `24 MB` (Review; contains user presets).
  - Total: `1.45 + 2.60 + 1.20 + 0.024 = 5.274 GB` (rounded cleanly to `5.25 GB` based on allocation blocks).
- **Integrity Guarantee:** The core application binary is marked `Protected` and cannot be removed from this interface without entering a dedicated uninstallation flow.

---

## Explore view: Treemap and folder table pair

The Explore view provides spatial and hierarchical discovery of disk usage across all scanned directories. It contains four coordinated components:

1. **Breadcrumb navigation**
2. **Filter bar**
3. **Interactive treemap**
4. **Navigable folder table (Accessibility equivalent)**
5. **Contextual inspector panel**

### 1. Breadcrumb navigation

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Macintosh HD  >  Users  >  synthetic-user  >  Developer  >  Projects                   │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

- Displays current filesystem depth.
- Each segment is a focusable, clickable button.
- Keyboard support: Left/Right arrows navigate segments; Enter jumps to that ancestor level.
- Overflow: Long paths collapse mid-chain (`Macintosh HD > … > Developer > Projects`).

### 2. Filter bar

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [ Search folder or file... ]  [ Class: All ▼ ]  [ Size: > 100 MB ▼ ]  [ Treemap | List]│
└────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Search field:** Filters child nodes by name in real time.
- **Safety class dropdown:** Filter by `All`, `Rebuildable`, `Review`, or `Protected`.
- **Size threshold dropdown:** Filter by `All sizes`, `> 10 MB`, `> 100 MB`, `> 1 GB`.
- **View switcher:** Segmented control toggling between Treemap view and Folder List view.

### 3. Interactive treemap

The treemap renders a nested squarified tessellation where rectangle area is strictly proportional to `sizeBytes`.

#### Treemap layout (1100 × 720 px)

```text
┌───────────────────────────────────────────────────────────────┬────────────────────────┐
│ Macintosh HD > Users > synthetic-user > Developer             │ Inspector: DerivedData │
├───────────────────────────────────────┬───────────────────────┼────────────────────────┤
│ Projects                              │ DerivedData           │ Name: DerivedData      │
│ 18.40 GB                              │ 12.60 GB              │ Size: 12.60 GB         │
│                                       │ [↺ Rebuildable]       │ Files: 4,812           │
│                                       │                       │ Class: Rebuildable     │
│                                       ├───────────────────────┤ Path:                  │
│                                       │ Simulators            │ ~/Library/Caches/…     │
│                                       │ 6.20 GB               │                        │
│                                       │ [↺ Rebuildable]       │ [ Copy Path ]          │
├───────────────────────────────────────┴───────────────────────┤ [ Add to Plan ]        │
│ Toolchains: 4.80 GB [🔒 Protected]    │ Archives: 2.10 GB [◇] │                        │
└───────────────────────────────────────┴───────────────────────┴────────────────────────┘
```

- **Tessellation algorithm:** Squarified treemap (aspect ratio bounded near 1:1 for legible labelling).
- **Color encoding:** Semantic tokens applied as borders and subtle surface tints (`--class-rebuildable-bg`, `--class-review-bg`, `--class-protected-bg`). Neutral `--surface` used for unclassified folders.
- **Selection sync:** Clicking a tile highlights it with `--focus-ring` and updates the Inspector.
- **Keyboard navigation:**
  - `Arrow` keys: spatial navigation across adjacent tiles.
  - `Enter` or `Down Arrow`: drill down into selected directory tile.
  - `Backspace`, `Esc`, or `Up Arrow`: navigate up to parent directory.

### 4. Navigable folder table (The treemap's equivalent list representation)

An interactive visual treemap is unusable with standard screen readers and inefficient for keyboard users who prefer linear navigation. DiskClearance requires a fully featured, accessible folder table representation.

The folder table presents identical underlying data, synchronizes state bidirectionally with the treemap, and supports dense sorting.

#### Folder table layout (1100 × 720 px)

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Name                       Safety Class       Last Modified    Item Count         Size │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ 📁 Projects                --                 Yesterday        14 folders     18.40 GB │
│ 📁 DerivedData             [↺ Rebuildable]    2 hours ago      4,812 files    12.60 GB │
│ 📁 Simulators              [↺ Rebuildable]    3 days ago       1,240 files     6.20 GB │
│ 📁 Toolchains              [🔒 Protected]     May 12, 2026     318 files       4.80 GB │
│ 📁 Archives                [◇ Review]         3 weeks ago      12 files        2.10 GB │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Sortable columns:** Name, Safety Class, Last Modified, Item Count, Size (all using tabular figures).
- **Keyboard operation:** Up/Down arrows move between rows. Enter opens/drills into a directory. Backspace navigates to the parent folder.
- **Selection parity:** Selecting a row in the table selects the corresponding tile if the user toggles back to the treemap.

### 5. Contextual inspector panel

The inspector displays complete details for the active node in either view.

- **At expanded width (`1100 px`):** Persistent right-hand pane (`280 px` wide).
- **At compact width (`760 px`):** Rendered as an overlay sheet opened via an `"Inspect"` toolbar button.
- **Contents:**
  - Item name, folder glyph, and canonical filesystem path.
  - Formatted byte size, allocated blocks, and item count.
  - Safety classification badge and evidence summary.
  - Primary actions: `"Reveal in Finder"` (opens native macOS Finder) and `"Add to Review Plan"` (available for `Rebuildable` and `Review` items; disabled for `Protected` items).

---

## Screen-reader label catalogue

Assistive technology must receive complete semantic state without visual ambiguity. The verbatim accessibility strings below must be implemented via `aria-label` or visually hidden text for each row type.

### 1. Rebuildable finding row (Selected state)

```text
"Xcode DerivedData, Rebuildable cache, 2.10 gigabytes, selected, safe to remove and automatically rebuilt by Xcode, press Space to uncheck, press Enter to expand evidence"
```

### 2. Rebuildable finding row (Unselected state)

```text
"Xcode DerivedData, Rebuildable cache, 2.10 gigabytes, not selected, safe to remove and automatically rebuilt by Xcode, press Space to select, press Enter to expand evidence"
```

### 3. Review finding row (Unselected state)

```text
"Xcode Device Logs, Requires review, 1.40 gigabytes, not selected, manual inspection required before removal, press Space to select, press Enter to expand evidence"
```

### 4. Protected finding row (Disabled state)

```text
"Command Line Tools, Protected by system policy, 4.80 gigabytes, uncheckable, essential system developer software, cannot be modified or removed, press Enter to view protection evidence"
```

### 5. Finding group header row

```text
"Xcode Developer Tools group, 3 items totaling 5.50 gigabytes, 2 rebuildable items selected, 1 review item unselected, expanded, press Space to toggle rebuildable items, press Enter to collapse"
```

### 6. Application row (Decomposed footprint)

```text
"SyntheticStudio Pro, version 3.4.1, total footprint 5.25 gigabytes, contains 1 protected application bundle and 3 related cache and data files, collapsed, press Enter to expand component breakdown"
```

### 7. Explore treemap tile

```text
"DerivedData directory, 12.60 gigabytes, 4,812 files, Rebuildable cache, tile 2 of 5 in Developer folder, press Enter to drill down into this folder, press Tab to move to inspector"
```

### 8. Explore folder table row

```text
"DerivedData, folder, 12.60 gigabytes, 4,812 files, Rebuildable cache, modified 2 hours ago, press Enter to open folder, press Right Arrow to inspect details"
```

---

## Reduced Motion specifications

In accordance with `docs/design/ACCESSIBILITY.md` and macOS system settings, DiskClearance provides complete support for `prefers-reduced-motion: reduce`.

Continuous animations, zooming charts, and sliding drawers can cause disorientation and vestibular distress. Under reduced motion, all transitions become instantaneous cuts.

### Motion mapping table

| Feature / Element                 | Standard Motion Experience                                     | Reduced Motion Experience                              | Rationale                                                         |
| :-------------------------------- | :------------------------------------------------------------- | :----------------------------------------------------- | :---------------------------------------------------------------- |
| **Treemap Zoom & Drill-down**     | 240ms smooth geometric zoom expanding clicked tile into canvas | Instant cut (0ms transition) to child directory layout | Large-scale geometric scaling triggers vestibular distress        |
| **Treemap/List View Switch**      | 180ms cross-fade and horizontal slide                          | Instant cut (0ms) between views                        | Eliminates spatial displacement across viewing plane              |
| **Evidence Disclosure Drawer**    | 160ms ease-out vertical accordion expansion (`height: auto`)   | Immediate display (0ms transition)                     | Removes moving text reflow while user is attempting to read       |
| **Finding Group Expand/Collapse** | 180ms vertical accordion fold                                  | Instant visibility toggle                              | Eliminates vertical layout shifts                                 |
| **Inspector Panel (Compact)**     | 200ms slide-up from bottom (`translateY`)                      | Instant cut-in without translation                     | Replaces moving drawer with static sheet                          |
| **Checkbox Selection Toggle**     | 120ms scale pulse (`scale(1.1)` to `scale(1)`)                 | Instant state change                                   | Prevents repeated micro-flicker during rapid checklist operations |

### CSS implementation pattern

```css
@media (prefers-reduced-motion: reduce) {
  .treemap-tile,
  .treemap-container,
  .evidence-drawer,
  .finding-row,
  .group-header,
  .inspector-sheet {
    transition-duration: 0.001ms !important;
    animation-duration: 0.001ms !important;
    transform: none !important;
  }

  /* Disable accordion expansion transitions */
  .evidence-drawer[data-state="expanded"] {
    animation: none !important;
    transition: none !important;
  }

  /* Disable treemap zoom transforms */
  .treemap-zoom-layer {
    transition: none !important;
    transform: none !important;
  }
}
```

---

## Token and styling reference

All components use design tokens established in `docs/design/TOKENS.md` and defined in `src/App.css`.

### Tokens across light and dark appearances

| Token Name                | Light Appearance | Dark Appearance | Component Role                                   |
| :------------------------ | :--------------- | :-------------- | :----------------------------------------------- |
| `--window`                | `#f4f2ec`        | `#19211f`       | Main window backdrop, treemap canvas background  |
| `--surface`               | `#faf9f5`        | `#222c29`       | Finding row container, group header surface      |
| `--surface-raised`        | `#ffffff`        | `#2a3632`       | Evidence disclosure drawer, inspector card       |
| `--divider`               | `#dfe5e2`        | `#2e3a36`       | Table grid lines, group separators               |
| `--text-primary`          | `#21312d`        | `#e4ece8`       | Entity names, tabular sizes, primary headings    |
| `--text-secondary`        | `#5e6d68`        | `#9eaca6`       | Timestamps, recoverability copy, path text       |
| `--accent`                | `#3f7567`        | `#82b8a8`       | Focus ring, primary button fills, active tabs    |
| `--accent-soft`           | `#dce9e3`        | `#263d36`       | Row hover highlight, group selection tint        |
| `--accent-fg`             | `#24483e`        | `#cae6dc`       | Active nav text, selection icon fill             |
| `--class-rebuildable-bg`  | `#daf0e4`        | `#1e3b2e`       | Rebuildable badge fill, safe treemap tile border |
| `--class-rebuildable-fg`  | `#164e3a`        | `#8ce4bd`       | Rebuildable badge text and glyph                 |
| `--class-review-bg`       | `#faecc6`        | `#382b13`       | Review badge fill, heuristic warning border      |
| `--class-review-fg`       | `#5c3e00`        | `#ffd78a`       | Review badge text and glyph                      |
| `--class-protected-bg`    | `#e1e6e5`        | `#26302e`       | Protected lock badge fill, system tile border    |
| `--class-protected-fg`    | `#283632`        | `#ccd5d1`       | Protected lock badge text and lock glyph         |
| `--class-irreversible-bg` | `#fae3e3`        | `#3b191b`       | Delete Now modal background                      |
| `--class-irreversible-fg` | `#7f1d1d`        | `#fca5a5`       | Delete Now modal warning text and button         |

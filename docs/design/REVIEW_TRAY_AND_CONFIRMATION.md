# Review tray, confirmation sheet, and completion summary specification

## Overview

The review tray, confirmation sheet, and completion summary govern the irreversible moment in DiskClearance: the transition where a user authorizes the modification or destruction of files on their system.

Every diagnostic view prior to this moment is strictly read-only. A user may scan, inspect evidence, filter by safety classification, and explore directory structures without altering a single byte. Once deletion is confirmed, files are either relocated to the macOS Trash or permanently destroyed. If this transition is deceptive, confusing, or careless, user data can be lost. Everything else in the interface can be refined across releases; this flow cannot be got wrong once.

This specification locks the architecture, interaction patterns, visual design, and exact copy for these three critical surfaces across light and dark appearances, at both the standard window size (`1100 × 720 px`) and the minimum supported window size (`760 × 560 px`).

```text
┌─────────────────┐       ┌────────────────────────┐       ┌────────────────────────┐
│ 1. Review Tray  │ ────► │ 2. Confirmation Sheet  │ ────► │ 3. Completion Summary  │
│ Selection totals│       │ Consequence in words   │       │ Honest 4-way accounting│
│ Class breakdown │       │ Trash vs Delete Now    │       │ Succeeded/Failed/etc.  │
│ Safe docked bar │       │ Pre-flight revalidation│       │ Per-item audit table   │
└─────────────────┘       └────────────────────────┘       └────────────────────────┘
```

---

## Locked design constraints

The following constraints are locked in this specification:

1. **Trash and permanent deletion never share a name, a colour, a confirmation pattern or a total.** Moving files to the macOS Trash and permanently destroying files are distinct actions with different recoverability semantics. They must never use the same verb, the same chromatic styling, the same modal layout, or be aggregated into a single figure.
2. **Red appears only for the irreversible action and for genuine failure. Nothing else in these three surfaces is red.** Protected items, review indicators, running storage figures, and standard buttons use neutral slate, amber, or brand accents. Red is reserved exclusively for unrecoverable destruction (`Delete Now`) and unrecoverable runtime errors (`Failed`).
3. **No recovery is implied after permanent deletion, in any copy, anywhere.** Permanent deletion removes files without routing through the Trash. No interface copy may suggest that permanent deletion is undoable, recoverable, or restorable from history. DiskClearance records what occurred in history as audit proof; history does not restore destroyed bytes.
4. **Focus is restored correctly when a sheet closes, and focus order follows reading order inside it.** When the confirmation sheet opens, initial focus is placed on the safe default action (`Move to Trash`) or the primary safe control. Keyboard traversal (`Tab`, `Shift+Tab`) is trapped strictly inside the sheet. When dismissed via `Cancel` or `Escape`, focus returns to the invocation control in the review tray.
5. **Reduced Motion applies: a sheet that must not animate should appear, not slide.** When `prefers-reduced-motion: reduce` is enabled, modal sheets, tray reveals, and status transitions cut in instantly (0ms duration) without translation transforms or fades.
6. **The default button is the recoverable one.** If a user presses `Return` without reading, the triggered action is strictly `Move to Trash`. Permanent deletion requires explicit, multi-step user interaction and can never be triggered by an accidental keystroke on dialog launch.

---

## Window viewports and responsive layout

DiskClearance supports macOS 13 Ventura and later across two canonical window viewports:

1. **Minimum window (compact breakpoint):** `760 × 560 px`
   - Content column width: `560 px` (window width `760 px` minus `200 px` sidebar).
   - Review tray: Fixed-docked bottom bar spanning the content column. Uses a compact stacked layout (`--space-12` vertical padding) to preserve touch and click targets without obscuring table content.
   - Content scroll offset: The parent finding table / review container enforces `padding-bottom: 120px` and `scroll-padding-bottom: 120px`. The bottom-most row scrolls completely clear of the docked tray.
   - Confirmation sheet: Centered modal dialog sized to `620 × 480 px` with internal scroll containment for item breakdowns.
2. **Common window (expanded breakpoint):** `1100 × 720 px`
   - Content column width: `880 px` (window width `1100 px` minus `220 px` sidebar; max readable line length `790 px`).
   - Review tray: Fixed-docked bottom bar spanning the content pane. Uses a single horizontal flex line (`--space-16` padding) with ample breathing room.
   - Content scroll offset: The parent finding container enforces `padding-bottom: 88px` and `scroll-padding-bottom: 88px`.
   - Confirmation sheet: Centered modal dialog sized to `720 × 520 px`.

---

## Surface 1: Review tray

### Purpose and placement

The review tray is an always-visible summary docked to the bottom of the review and cleanup views. It acts as the anchor for the user's intent: it shows how many items are selected, breaks them down by safety classification, displays running storage figures, and presents the primary action button to initiate confirmation.

The tray never floats over or occludes finding rows. The scrollable content container above it always provides dedicated clearance space matching the tray's height plus margins.

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Finding Row 41: ~/Library/Caches/com.synthetic.developer/DerivedData    2.10 GB [Rebuildable]│
│ Finding Row 42: ~/Library/Caches/com.synthetic.developer/ModuleCache    1.40 GB [Rebuildable]│
│ Finding Row 43: ~/Library/Application Support/SyntheticStudio/Logs      2.00 GB [Review]     │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [Review Tray: Always visible at bottom, never occluding rows above]                          │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Running dual storage figures

The review tray displays two storage figures that must **never merge**:

1. **Ready to move to Trash:** The exact sum of bytes in currently selected items (e.g. `5.50 GB`). Moving these items to the macOS Trash changes their location, but does not free disk space on the volume.
2. **Space available after Trash is emptied:** The combined reclaimable storage if the user empties the macOS Trash following this operation (e.g. `13.90 GB`). This figure decomposes inline: `8.40 GB already in Trash · 5.50 GB from this selection`.

These figures are distinct because claiming that moving files to Trash immediately reclaims storage is factually false. macOS only reclaims volume space when the Trash directory is purged. Merging these figures into a single total deceives the user about current disk capacity.

### Class breakdown badge

The review tray exposes the exact composition of the current selection:

```text
43 items selected (40 Rebuildable, 3 Review, 0 Protected)
```

This breakdown ensures a user notices if they have selected items marked `Review` alongside items marked `Rebuildable`. Protected items cannot be selected and always display `0 Protected`.

### Visual layout: 1100 × 720 px (Expanded)

At `1100 × 720 px`, the review tray renders as a single horizontal toolbar spanning the content canvas:

```text
┌────────────────────────────────────────────────────────────────────────────────────────────┐
│  43 items selected  │  Ready for Trash: 5.50 GB            │ After Trash empty: 13.90 GB   │
│  [40 Rebuildable]   │  8.40 GB in Trash · 5.50 GB selected │                               │
│  [ 3 Review]        │  [Clear selection]                   │        [ Review Selection… ]  │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: 760 × 560 px (Compact)

At minimum width (`760 × 560 px`), the tray reflows into a two-row structured bar, keeping all metrics and the primary button accessible without horizontal overflow:

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Ready for Trash: 5.50 GB        │ After Trash empty: 13.90 GB                │
│ 8.40 GB in Trash · 5.50 GB new  │ (Reclaims disk space only after emptying)  │
├─────────────────────────────────┴────────────────────────────────────────────┤
│ 43 selected (40 Rebuildable, 3 Review)                 [ Review Selection… ] │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Styling and tokens

The review tray uses raised surface elevation to delineate itself from the scrollable list:

- **Surface backdrop:** `--surface-raised` (`#ffffff` light / `#2a3632` dark).
- **Border top:** 1px solid `--divider` (`#dfe5e2` light / `#2e3a36` dark).
- **Box shadow:** `0 -4px 16px rgba(0, 0, 0, 0.04)` light / `0 -4px 16px rgba(0, 0, 0, 0.24)` dark.
- **Primary action button (`Review Selection…`):**
  - Background: `--accent` (`#3f7567` light / `#82b8a8` dark).
  - Text: `#ffffff` light / `#19211f` dark (WCAG AA compliant contrast > 4.5:1).
  - Minimum height: `var(--target-primary)` (`40px`).
  - Border-radius: `var(--radius-control)` (`8px`).
- **Clear selection control:**
  - Text: `--text-secondary` (`#5e6d68` light / `#9eaca6` dark).
  - Hover state: `--text-primary` (`#21312d` light / `#e4ece8` dark) with underline.
- **Figures:** `font-variant-numeric: tabular-nums;` via `--text-primary`.

---

## Surface 2: Confirmation sheet

### Two non-variant actions

When the user activates `[ Review Selection… ]`, DiskClearance opens a modal confirmation sheet. The sheet presents two choices that are **not** interchangeable visual variants:

1. **Move to Trash (Recoverable — The Default):**
   - Moves files to `~/.Trash` on the same APFS volume.
   - Files remain inspectable and restorable via Finder until the user explicitly empties Trash.
   - States explicitly in copy: _"Files will be moved to macOS Trash. Disk space is not freed until Trash is emptied."_
   - Styled in calm, default accent chrome.
   - Confirmation is activated with a single click or by pressing `Return`.
2. **Delete Now (Irreversible — Deliberate Friction):**
   - Unlinks files immediately from the filesystem.
   - Reclaims disk space immediately.
   - States explicitly in copy: _"Files will be deleted immediately and permanently. This action cannot be undone and these files cannot be recovered."_
   - Contains **zero recovery language**: no mention of history restoration or backup recovery.
   - Styled with warning chrome: `--class-irreversible-bg` and `--class-irreversible-fg`.
   - Requires deliberate selection and an explicit confirmation checkbox gate before the destructive button enables.

### Stating consequence in words, not button colour

In accordance with `docs/design/ACCESSIBILITY.md`, safety status and consequence must never depend on colour alone. Both modes provide clear, multi-sentence explanatory copy describing:

1. What physical operation will occur.
2. When storage is reclaimed.
3. Whether recovery is possible.

### Visual layout: Move to Trash (Default) — 1100 × 720 px

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Review Deletion Plan                                                        [Close ✕]  │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ Operation Mode:                                                                        │
│   (●) Move to Trash (Recoverable)          ( ) Delete Now (Permanently Irreversible)   │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ Target Scope: 43 items selected                                                        │
│ • Rebuildable items: 40 items (3.50 GB)                                                │
│ • Review items:       3 items (2.00 GB)                                                │
│                                                                                        │
│ Consequence:                                                                           │
│ Files will be moved to macOS Trash. Disk space is not freed until Trash is emptied.    │
│ Items remain recoverable in Finder until Trash is emptied.                             │
│                                                                                        │
│ Storage impact:                                                                        │
│ • Ready to move to Trash:  5.50 GB                                                     │
│ • Reclaimed immediately:   0 B                                                         │
│ • Space after Trash empty: 13.90 GB (8.40 GB existing in Trash + 5.50 GB new)          │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Cancel ]                                                        [ Move to Trash ]    │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: Move to Trash (Default) — 760 × 560 px

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Review Deletion Plan                                                [Close ✕]│
├──────────────────────────────────────────────────────────────────────────────┤
│ Mode: (●) Move to Trash (Recoverable)   ( ) Delete Now (Irreversible)        │
├──────────────────────────────────────────────────────────────────────────────┤
│ Selection: 43 items (40 Rebuildable, 3 Review)                               │
│                                                                              │
│ Consequence:                                                                 │
│ Files will be moved to macOS Trash. Disk space is not freed until            │
│ Trash is emptied. Items remain recoverable in Finder until emptied.          │
│                                                                              │
│ • Ready for Trash: 5.50 GB  • Immediate free: 0 B  • After empty: 13.90 GB   │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ Cancel ]                                               [ Move to Trash ]   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: Delete Now (Irreversible) — 1100 × 720 px

When the user selects the `Delete Now` mode selector, the sheet shifts into the irreversible destruction state. Red appears on this surface because destruction is permanent:

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Review Deletion Plan                                                        [Close ✕]  │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ Operation Mode:                                                                        │
│   ( ) Move to Trash (Recoverable)          (●) Delete Now (Permanently Irreversible)   │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ ⚠ Irreversible Permanent Destruction                                                   │
│                                                                                        │
│ Target Scope: 43 items selected (40 Rebuildable, 3 Review)                             │
│                                                                                        │
│ Consequence:                                                                           │
│ Files will be deleted immediately and permanently. This action cannot be undone        │
│ and these files cannot be recovered.                                                   │
│                                                                                        │
│ Storage impact:                                                                        │
│ • Permanently reclaimed immediately: 5.50 GB                                           │
│ • Pending in Trash:                  0 B                                               │
│                                                                                        │
│ Mandatory acknowledgment:                                                              │
│ [ ] I understand that these files will be permanently destroyed and cannot be recovered│
├────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Cancel ]                                             [ Delete Now Permanently ]      │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

_Note on button state:_ The `[ Delete Now Permanently ]` button is rendered in disabled styling (`opacity: 0.5; cursor: not-allowed;`) until the mandatory acknowledgment checkbox is checked by the user.

### Visual layout: Delete Now (Irreversible) — 760 × 560 px

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Review Deletion Plan                                                [Close ✕]│
├──────────────────────────────────────────────────────────────────────────────┤
│ Mode: ( ) Move to Trash (Recoverable)   (●) Delete Now (Irreversible)        │
├──────────────────────────────────────────────────────────────────────────────┤
│ ⚠ Irreversible Permanent Destruction                                         │
│                                                                              │
│ Consequence:                                                                 │
│ Files will be deleted immediately and permanently. This action               │
│ cannot be undone and these files cannot be recovered.                        │
│                                                                              │
│ Storage reclaimed immediately: 5.50 GB (Permanently destroyed)               │
│                                                                              │
│ [ ] I understand that these files will be permanently destroyed              │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ Cancel ]                                        [ Delete Now Permanently ] │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Surface 2b: Last-moment blocked-item state

### Revalidation race conditions

Between the time a user reviews findings and the time they confirm deletion, the state of the local filesystem may change. For example:

- An application or compiler wrote new files into a build cache directory.
- A running background process acquired a lock on a log file.
- Permissions on a target path were revoked or altered.
- A target path was replaced by a symbolic link pointing elsewhere.

DiskClearance executes a pre-flight revalidation pass in the Rust core immediately before any filesystem mutation occurs. If revalidation detects that items have changed, been locked, or violated safety boundaries, execution halts before touching any file.

### UI presentation of revalidation rejection

When items are rejected during pre-flight revalidation:

1. The sheet transitions to the **Blocked Items State**.
2. A warning banner identifies exactly how many items were blocked.
3. An itemized table enumerates the blocked paths and the exact reason for rejection.
4. The remaining valid items and their adjusted storage totals are presented.
5. **Fresh confirmation is required:** The user cannot proceed without actively reviewing the adjusted scope and confirming again.

### Dependency declaration

The blocked-item state depends on a revalidation contract that is specified in the core revalidation interface and does not exist yet (planned in the safety revalidation subsystem). This specification establishes the UI presentation and interaction contract that will consume that engine once implemented; it does not invent the engine contract or present it as settled.

### Visual layout: Blocked-after-review state — 1100 × 720 px

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Pre-flight Check: 2 Items Blocked                                           [Close ✕]  │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ 2 items cannot be removed and were blocked during pre-flight revalidation:             │
│                                                                                        │
│ ┌── Blocked Targets ─────────────────────────────────────────────────────────────────┐ │
│ │ • Xcode ModuleCache (120 MB)                                                       │ │
│ │   Reason: Target modified after initial review (mtime changed 14s ago)             │ │
│ │ • SyntheticStudio Session Lock (80 MB)                                             │ │
│ │   Reason: Target locked by running process (PID 49102 SyntheticStudio)             │ │
│ └────────────────────────────────────────────────────────────────────────────────────┘ │
│                                                                                        │
│ Revised Deletion Plan:                                                                 │
│ • Remaining valid items: 41 items (5.30 GB)                                            │
│ • Blocked items excluded: 2 items (200 MB)                                             │
│                                                                                        │
│ A fresh confirmation is required to proceed with the remaining 41 items.               │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Cancel All ]                                      [ Continue with Remaining 41 ]     │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: Blocked-after-review state — 760 × 560 px

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Pre-flight Check: 2 Items Blocked                                   [Close ✕]│
├──────────────────────────────────────────────────────────────────────────────┤
│ 2 items were blocked during pre-flight verification:                         │
│                                                                              │
│ • Xcode ModuleCache (120 MB): Target modified after review                   │
│ • SyntheticStudio Lock (80 MB): Locked by running process                    │
│                                                                              │
│ Revised scope: 41 remaining items (5.30 GB). Blocked items skipped.          │
│ A fresh confirmation is required to proceed.                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ Cancel All ]                                  [ Continue with Remaining ]  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Surface 3: Completion summary

### Four segregated outcome counts

When the operation completes, DiskClearance presents a detailed completion summary. The summary breaks results into four strict categories:

1. **Succeeded:** Items successfully relocated to Trash or permanently deleted.
2. **Failed:** Items that encountered I/O, filesystem, or permission errors during execution.
3. **Skipped:** Items bypassed due to dependency constraints or user exclusion.
4. **Blocked:** Items halted by pre-flight revalidation guards.

### The mixed-run rule

**A run with failures is never presented as success.** If even one item fails or is blocked:

- The summary headline does not say `"Cleanup Complete!"`.
- No checkmark badge is displayed.
- The headline honestly reports: `"Operation completed with 2 failures and 2 blocked items"`.
- The status pill uses `--class-review-bg` / `--class-review-fg` (for blocked/skipped) or `--class-irreversible-bg` / `--class-irreversible-fg` (for errors/failures).

### Segregated storage totals

The summary keeps storage figures strictly segregated:

- **Pending in Trash:** Storage awaiting manual purge in Finder (e.g. `4.80 GB`). Accompanied by the instruction: _"Empty Trash in Finder to reclaim this disk space."_
- **Permanently Reclaimed:** Storage unlinked directly from the volume (e.g. `0 B` for Trash operations, or exact bytes for permanent deletion).

These two figures are **never summed into a single total**.

### Per-item audit table reachable directly

Per-item execution records are not hidden in a log file. A navigable results list sits directly within the summary view, featuring filter chips:

```text
[ All (43) ]  [ Succeeded (38) ]  [ Failed (2) ]  [ Blocked (2) ]  [ Skipped (1) ]
```

Selecting any failed or blocked item immediately reveals its diagnostic message, target path, and error code.

### Calm, dignified completion

If all items succeed without error, the screen remains calm and measured:

- Headline: `"38 items moved to Trash"`.
- Subtitle: `"4.80 GB ready to reclaim. Empty macOS Trash in Finder to free disk space."`.
- No confetti, no celebratory animations, no congratulatory praise over deleted files.

### Visual layout: Mixed run — 1100 × 720 px (Expanded)

```text
┌────────────────────────────────────────────────────────────────────────────────────────────┐
│ Operation completed with 2 failures and 2 blocked items                                    │
│ Target volume: Macintosh HD                                                                │
├────────────────────────────────────────────────────────────────────────────────────────────┤
│ ┌── Succeeded ──────┐  ┌── Failed ─────────┐  ┌── Blocked ────────┐  ┌── Skipped ────────┐ │
│ │ 38 items          │  │ 2 items           │  │ 2 items           │  │ 1 item            │ │
│ │ 4.80 GB moved     │  │ 400 MB failed     │  │ 200 MB blocked    │  │ 100 MB skipped    │ │
│ └───────────────────┘  └───────────────────┘  └───────────────────┘  └───────────────────┘ │
│                                                                                            │
│ Storage Summary:                                                                           │
│ • Pending in macOS Trash:  4.80 GB (Reclaimed after Trash is emptied in Finder)            │
│ • Permanently reclaimed:   0 B                                                             │
│                                                                                            │
│ Per-item Results:                                                                          │
│ [ All (43) ]  [ Succeeded (38) ]  [● Failed (2) ]  [● Blocked (2) ]  [ Skipped (1) ]       │
│ ┌────────────────────────────────────────────────────────────────────────────────────────┐ │
│ │ [✕ Failed]   SyntheticStudio Cache (280 MB)  Permission denied (EACCES)                │ │
│ │ [✕ Failed]   Homebrew Staging (120 MB)       Device busy or locked                     │ │
│ │ [◇ Blocked]  Xcode ModuleCache (120 MB)      Modified after review                     │ │
│ │ [◇ Blocked]  SyntheticStudio Lock (80 MB)    Locked by active process                  │ │
│ └────────────────────────────────────────────────────────────────────────────────────────┘ │
├────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Export Audit Log ]                                                   [ Back to Cleanup ] │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: Mixed run — 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Operation completed with 2 failures and 2 blocked items                      │
├──────────────────────────────────────────────────────────────────────────────┤
│ • Succeeded: 38 items (4.80 GB)       • Failed:  2 items (400 MB)            │
│ • Blocked:    2 items (200 MB)        • Skipped: 1 item  (100 MB)            │
│                                                                              │
│ Storage:                                                                     │
│ Pending in Trash: 4.80 GB (Empty Trash in Finder to reclaim)                 │
│ Permanently freed: 0 B                                                       │
├──────────────────────────────────────────────────────────────────────────────┤
│ Results: [ All (43) ] [ Failed (2) ] [ Blocked (2) ]                         │
│ • [✕ Failed]  SyntheticStudio Cache (280 MB): Permission denied              │
│ • [✕ Failed]  Homebrew Staging (120 MB): Device busy                         │
│ • [◇ Blocked] Xcode ModuleCache (120 MB): Modified after review              │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ Export Log ]                                           [ Back to Cleanup ] │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: Clean run — 1100 × 720 px (Expanded)

```text
┌────────────────────────────────────────────────────────────────────────────────────────────┐
│ 38 items moved to Trash                                                                    │
│ Target volume: Macintosh HD                                                                │
├────────────────────────────────────────────────────────────────────────────────────────────┤
│ ┌── Succeeded ──────┐  ┌── Failed ─────────┐  ┌── Blocked ────────┐  ┌── Skipped ────────┐ │
│ │ 38 items          │  │ 0 items           │  │ 0 items           │  │ 0 items           │ │
│ │ 4.80 GB moved     │  │ 0 B failed        │  │ 0 B blocked       │  │ 0 B skipped       │ │
│ └───────────────────┘  └───────────────────┘  └───────────────────┘  └───────────────────┘ │
│                                                                                            │
│ Storage Status:                                                                            │
│ • Pending in macOS Trash:  4.80 GB                                                         │
│ • Permanently reclaimed:   0 B                                                             │
│                                                                                            │
│ To reclaim this storage, empty the Trash in macOS Finder.                                  │
├────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ View Reclaimed Files ]                                               [ Back to Cleanup ] │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Visual layout: Clean run — 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ 38 items moved to Trash                                                      │
├──────────────────────────────────────────────────────────────────────────────┤
│ 38 items (4.80 GB) moved to macOS Trash. 0 failures.                         │
│                                                                              │
│ Storage:                                                                     │
│ • Pending in Trash: 4.80 GB                                                  │
│ • Permanently reclaimed: 0 B                                                 │
│                                                                              │
│ Empty macOS Trash in Finder to free this disk space.                         │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ View Files ]                                           [ Back to Cleanup ] │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Colour-blind simulation and visual differentiation

### Analysis under red-green deficiency (Deuteranopia)

Deuteranopia affects approximately 8% of male users. Under deuteranopia:

- Red (`--class-irreversible-bg` / `--class-irreversible-fg`) and green (`--accent` / `--class-rebuildable-fg`) shift toward desaturated brownish-yellowish shades.
- Chromatic separation alone is insufficient to signal danger vs safety.

### How Move to Trash and Delete Now remain distinct without hue

DiskClearance differentiates the two actions through five non-chromatic structural layers:

1. **Spatial segregation:** `Move to Trash` sits in the standard action button position at the bottom right. `Delete Now` requires switching the segmented mode selector to a distinct view, isolating the irreversible action into its own separate panel.
2. **Iconographic signaling:**
   - Move to Trash uses the recycling loop: `↺` (Loop) or `[Trash]`.
   - Delete Now uses the warning triangle: `⚠` (Alert) or `[!]`.
3. **Typographic copy:**
   - Move to Trash is labelled: `"Move to Trash (Recoverable)"`.
   - Delete Now is labelled: `"Delete Now (Permanently Irreversible)"`.
   - The consequences are written out in unambiguous sentences in both modes.
4. **Interaction friction gate:**
   - Move to Trash can be confirmed immediately with a single click.
   - Delete Now displays a disabled action button until the user deliberately checks the mandatory confirmation checkbox: `[ ] I understand that these files will be permanently destroyed and cannot be recovered`.
5. **Border styling and surface structure:**
   - Move to Trash is rendered on a clean `--surface-raised` background.
   - Delete Now is encapsulated within a high-contrast bordered alert callout box featuring prominent warning iconography.

### Analysis under total colour blindness (Achromatopsia)

Under achromatopsia (monochrome vision, zero hue):

- Lightness contrast ratios between text and background remain above 4.5:1 (WCAG AA) across both modes.
- The visual hierarchy is carried entirely by spatial layout, typography weights (`font-weight: 700`), borders, and the mandatory checkbox interaction barrier.

---

## Final copy reference

The following copy is locked across all interfaces:

| Surface / Action                            | Exact Interface Copy                                                                                                                                           |
| :------------------------------------------ | :------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Review Tray: Running Figures**            | `"Ready for Trash: 5.50 GB"`<br>`"After Trash empty: 13.90 GB (8.40 GB in Trash · 5.50 GB selected)"`                                                          |
| **Review Tray: Class Breakdown**            | `"43 items selected (40 Rebuildable, 3 Review)"`                                                                                                               |
| **Review Tray: Primary Button**             | `"[ Review Selection… ]"`                                                                                                                                      |
| **Confirmation: Move to Trash Description** | `"Files will be moved to macOS Trash. Disk space is not freed until Trash is emptied. Items remain recoverable in Finder until Trash is emptied."`             |
| **Confirmation: Move to Trash Button**      | `"[ Move to Trash ]"`                                                                                                                                          |
| **Confirmation: Delete Now Description**    | `"Files will be deleted immediately and permanently. This action cannot be undone and these files cannot be recovered."`                                       |
| **Confirmation: Delete Now Checkbox**       | `"[ ] I understand that these files will be permanently destroyed and cannot be recovered"`                                                                    |
| **Confirmation: Delete Now Button**         | `"[ Delete Now Permanently ]"`                                                                                                                                 |
| **Confirmation: Blocked Items Warning**     | `"2 items cannot be removed and were blocked during pre-flight revalidation."`<br>`"A fresh confirmation is required to proceed with the remaining 41 items."` |
| **Completion: Clean Run Headline**          | `"38 items moved to Trash"`                                                                                                                                    |
| **Completion: Clean Run Subtitle**          | `"4.80 GB ready to reclaim. Empty macOS Trash in Finder to free disk space."`                                                                                  |
| **Completion: Mixed Run Headline**          | `"Operation completed with 2 failures and 2 blocked items"`                                                                                                    |
| **Completion: Segregated Storage**          | `"Pending in macOS Trash: 4.80 GB (Reclaimed after Trash is emptied in Finder)"`<br>`"Permanently reclaimed: 0 B"`                                             |

---

## Keyboard interaction walkthrough

Keyboard navigation strictly conforms to macOS accessibility conventions:

### Initial focus on dialog open

1. **Confirmation sheet (Move to Trash mode):**
   - Initial focus is placed on the safe primary action button: `[ Move to Trash ]`.
   - The user can press `Return` immediately to safely execute the recoverable operation.
   - Pressing `Escape` or `Tab` moves to `[ Cancel ]` or dismisses the dialog.
2. **Confirmation sheet (Delete Now mode):**
   - If the user switches to `Delete Now`, focus moves directly to the mandatory acknowledgment checkbox: `[ ] I understand that these files will be permanently destroyed`.
   - Pressing `Return` while focus is on the unchecked checkbox does nothing.
   - Pressing `Space` checks the box and enables the `[ Delete Now Permanently ]` button.
   - Pressing `Tab` then moves focus to `[ Delete Now Permanently ]`.
3. **Blocked-items revalidation sheet:**
   - Initial focus is placed on `[ Cancel All ]` to ensure an accidental keystroke retreats safely.
   - The user must deliberately press `Tab` to reach `[ Continue with Remaining 41 ]`.

### Return key rules

- When focus is on `[ Move to Trash ]`: Pressing `Return` triggers the Trash operation.
- When focus is on `[ Cancel ]`: Pressing `Return` dismisses the dialog without taking action.
- When `Delete Now` mode is active but the checkbox is unchecked: Pressing `Return` does not execute deletion.

### Escape key rules

- Pressing `Escape` at any point in any modal sheet immediately dismisses the sheet.
- Zero filesystem changes are made.
- The user is returned to the review screen.

### Focus trapping and restoration

- While a modal sheet is open, keyboard focus is trapped within the sheet. `Tab` and `Shift+Tab` cycle between the close button, mode selector, acknowledgment checkbox (if present), cancel button, and confirm button. Focus never leaks into the background window.
- When the sheet closes (via confirmation, cancellation, or `Escape`), focus is restored directly to the trigger button in the review tray (`#review-tray-action`).

---

## Reduced Motion specifications

In accordance with `docs/design/ACCESSIBILITY.md` and `src/App.css`, DiskClearance provides complete support for `prefers-reduced-motion: reduce`.

### Motion mapping table

| Component / Interaction     | Standard Motion Experience                                          | Reduced Motion Experience       | Rationale                                                 |
| :-------------------------- | :------------------------------------------------------------------ | :------------------------------ | :-------------------------------------------------------- |
| **Confirmation Sheet Open** | 160ms scale and fade-in (`opacity: 0` to `1`, `scale: 0.98` to `1`) | Instant cut (0ms transition)    | Removes vestibular displacement upon modal launch         |
| **Review Tray Docking**     | 120ms slide-up from bottom (`translateY(100%)` to `0`)              | Static instant appearance (0ms) | Prevents moving elements from distracting during scanning |
| **Mode Selector Switch**    | 140ms cross-fade between Trash and Delete Now panels                | Instant cut (0ms)               | Eliminates flicker and content displacement               |
| **Blocked Items Alert**     | 160ms accordion expansion                                           | Immediate display (0ms)         | Reflow occurs instantly without layout motion             |
| **Completion View Switch**  | 180ms cross-fade from review view to summary                        | Instant cut (0ms)               | Clean view replacement without ocular strain              |

### CSS implementation pattern

```css
@media (prefers-reduced-motion: reduce) {
  .confirmation-sheet,
  .review-tray,
  .mode-panel,
  .blocked-callout,
  .completion-summary {
    transition: none !important;
    animation: none !important;
    transform: none !important;
  }
}
```

---

## Token and styling reference

All colours and visual styles use the semantic design tokens defined in `src/App.css`:

### Tokens across light and dark appearances

| Token Name                | Light Appearance | Dark Appearance | Component Role                                           |
| :------------------------ | :--------------- | :-------------- | :------------------------------------------------------- |
| `--window`                | `#f4f2ec`        | `#19211f`       | Window canvas background, modal backdrop scrim           |
| `--surface`               | `#faf9f5`        | `#222c29`       | Review list background, summary container                |
| `--surface-raised`        | `#ffffff`        | `#2a3632`       | Review tray surface, confirmation sheet body             |
| `--divider`               | `#dfe5e2`        | `#2e3a36`       | Tray top border, modal header separator                  |
| `--text-primary`          | `#21312d`        | `#e4ece8`       | Primary headlines, tabular numerals, item names          |
| `--text-secondary`        | `#5e6d68`        | `#9eaca6`       | Explanatory copy, secondary metrics, timestamps          |
| `--accent`                | `#3f7567`        | `#82b8a8`       | Primary button fill, Move to Trash highlight, focus ring |
| `--accent-soft`           | `#dce9e3`        | `#263d36`       | Selected row tint, class breakdown container             |
| `--accent-fg`             | `#24483e`        | `#cae6dc`       | Active mode selector text, high-contrast label           |
| `--class-rebuildable-bg`  | `#daf0e4`        | `#1e3b2e`       | Rebuildable class count background                       |
| `--class-rebuildable-fg`  | `#164e3a`        | `#8ce4bd`       | Rebuildable class count text                             |
| `--class-review-bg`       | `#faecc6`        | `#382b13`       | Review class count background, blocked status pill       |
| `--class-review-fg`       | `#5c3e00`        | `#ffd78a`       | Review class count text, blocked status glyph            |
| `--class-protected-bg`    | `#e1e6e5`        | `#26302e`       | Protected lock background                                |
| `--class-protected-fg`    | `#283632`        | `#ccd5d1`       | Protected lock icon and text                             |
| `--class-irreversible-bg` | `#fae3e3`        | `#3b191b`       | Delete Now alert callout background, error pill          |
| `--class-irreversible-fg` | `#7f1d1d`        | `#fca5a5`       | Delete Now alert text, failure status text               |

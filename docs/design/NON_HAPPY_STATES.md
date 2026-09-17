# Non-happy states, permission refusals, degraded totals, and error handling specification

## Overview

The standard operation of DiskClearance assumes an ideal environment: permissions are granted, files remain stationary on disk, operations proceed to completion without interruption, and volumes remain mounted. In real-world operation on macOS, these assumptions routinely fail.

DiskClearance is designed for the reality of hostile filesystem conditions:

- Full Disk Access is frequently refused or restricted by macOS privacy controls (TCC).
- Items reviewed in one second may be modified, locked, or deleted by another background daemon in the next.
- External APFS volumes or disk images may be abruptly disconnected mid-operation.
- Users cancel scans and deletions mid-flight and need immediate accounting of what happened.
- Traversal encounters unreadable directories, broken permissions, or full disk volumes.

The primary design principle governing these states is **truthfulness over comfort**:

1. **An incomplete scan never pretends completion.** If an area is inaccessible, totals are explicitly qualified to state what was excluded.
2. **An error explains the situation in plain English.** Never a raw numeric code alone, and never a generic "something went wrong". Every error communicates what occurred, what was left untouched, and how to proceed.
3. **A cancelled operation accounts for every byte touched.** If a user cancels a multi-item delete, the system immediately states what was already moved to Trash, what was left untouched, and where the completed items reside.
4. **Empty states communicate health, not failure.** A clean disk or an empty filter result is a neutral, reassuring finding and is never styled with warning amber or error red.

This specification locks the architecture, interaction patterns, visual design, exact copy, and screen-reader announcements for every non-happy state across light and dark appearances, at both the standard expanded window size (`1100 × 720 px`) and the minimum supported window size (`760 × 560 px`).

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│                                NON-HAPPY STATE ARCHITECTURE                                  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ 1. Permission Boundaries  ──► Refused / Scoped / Revoked mid-scan                            │
│ 2. Scan Interruptions     ──► Scanning / Paused / Cancelled / Failed / Errors / Clean        │
│ 3. Empty Views            ──► Clean disk / No filter matches / Tool absent                   │
│ 4. Execution Errors       ──► Vanished / EACCES / Disk Full / EBUSY / Volume Unmounted       │
│ 5. Degraded Totals        ──► Partial scan qualification / Stale cache / Interrupted         │
│ 6. Transitional States    ──► Cold paint / Long scan throttled / Action in progress          │
│ 7. Cancelled Mutation     ──► Exact 4-way post-cancel accounting (Trash vs Untouched)        │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## Locked design constraints

The following constraints are locked in this specification:

1. **An incomplete scan never presents its totals as complete.** Every figure derived from partial coverage says what it excluded. This is the same rule the first-run document applies to its two summary figures. Totals computed without complete visibility must be visually qualified inline (e.g. `6.30 GB verified (Partial · excludes 2 restricted scopes)`) alongside an itemized list of inaccessible directories.
2. **An error says what happened, what was not done, and what the user can do.** Never a code alone, never "something went wrong". Every error banner, modal, or inline row communicates the concrete failure cause, clarifies which filesystem entities were skipped or left intact, and provides an actionable remedy or safe exit.
3. **A cancelled operation states exactly what completed before cancelling.** A user who cancels a delete must know what was already deleted. The post-cancellation report segregates succeeded items, skipped items, and failed items, detailing where completed items went (macOS Trash or unlinked) and confirming that skipped items remain untouched.
4. **Red only for genuine failure and the irreversible action. An empty state is not an error and is not red.** Empty disk states, cleared filter results, neutral lock badges, and informatively scoped permission notices use neutral, slate, or calm accent tokens. Red is strictly reserved for permanent unrecoverable destruction and unrecoverable I/O or filesystem failures.
5. **Every state is reachable and legible by keyboard and screen reader, with its message written out, not left to implementation.** VoiceOver spoken announcements and ARIA attributes (`role="alert"`, `role="status"`, `aria-live`) are specified verbatim for every state. Focus placement and restoration adhere to strict keyboard reading order.
6. **Reduced Motion: a spinner becomes a static status with a readable percentage.** Continuous spinner animations, pulsing bars, and indeterminate sweep effects are prohibited under `prefers-reduced-motion: reduce`. They are replaced with static text percentages, discrete numerical counts, and immediate (0ms) state transitions.

---

## Window viewports and responsive layout

DiskClearance supports macOS 13 Ventura and later across two canonical window viewports:

1. **Minimum window (compact breakpoint):** `760 × 560 px`
   - Content pane width: `560 px` (`760 px` window minus `200 px` sidebar rail).
   - Layout reflow: Side-by-side metric boxes reflow into vertically stacked cards. Banners collapse secondary explanatory text into structured bullet points or disclosure links. Primary action buttons take full available container width or right-aligned stacked clusters.
   - Text density: Padding reduces from `--space-32` / `--space-48` to `--space-16` / `--space-24`.
2. **Common window (expanded breakpoint):** `1100 × 720 px`
   - Content pane width: `880 px` (`1100 px` window minus `220 px` sidebar; maximum readable line length `790 px`).
   - Layout reflow: Ample horizontal breathing room. Explanatory summaries, running metrics, and actions sit side-by-side in balanced horizontal toolbars and grid callouts.

---

## Surface 1: Permission states

### 1.1 Full Disk Access refused (Standard Access mode)

#### Context and capabilities

macOS restricts access to user sandboxes (`~/Library/Containers`), system caches (`/Library/Caches`), Time Machine snapshots, and root logs unless the application has been granted Full Disk Access (FDA) in System Settings. When a user declines to grant FDA—either during first run or by clicking "Continue without access"—DiskClearance does not halt or show an error. It operates in **Standard Access mode**.

In Standard Access mode, DiskClearance can honestly:

- Inspect user application caches (`~/Library/Caches`).
- Inspect user developer tool caches and build artifacts (e.g. Xcode DerivedData, CocoaPods, Cargo, npm, pip).
- Inspect user Trash (`~/.Trash`).
- Inspect user-accessible temporary files and non-sandboxed application logs.

DiskClearance cannot inspect:

- System-wide caches (`/Library/Caches`) and daemon logs (`/var/log`).
- Sandboxed application storage (`~/Library/Containers`, `~/Library/Group Containers`).
- Mail, Messages, Safari history, and protected user data stores.

#### Visual presentation

A persistent, calm banner spans the top of the Home and Cleanup screens. The banner uses `--surface-raised` with a neutral border and the lock glyph (`🔒`). It does not use red or warning yellow, because running with standard privileges is a legitimate, supported operating mode.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ [Brand Mark] DiskClearance                Macintosh HD · 494.20 GB available         [?]     │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ 🔒 Standard Access Mode                                                [ Grant Access… ]      │
│    Operating with standard permissions. System caches and sandboxed containers are excluded. │
│    DiskClearance inspects user build artifacts, caches, and trash without Full Disk Access.  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Scan Scope Overview                                                                          │
│ • User Caches:        Accessible (~/Library/Caches, Developer tools)                         │
│ • User Trash:         Accessible (~/.Trash)                                                  │
│ • System Caches:      Excluded   (/Library/Caches · Requires Full Disk Access)               │
│ • App Sandboxes:      Excluded   (~/Library/Containers · Requires Full Disk Access)          │
│                                                                                              │
│ [ Start Scan with Standard Access ]                                                          │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ 🔒 Standard Access Mode                                    [ Grant… ]         │
│    Operating with standard permissions. System caches and sandboxed          │
│    containers are excluded from inspection.                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│ • Accessible: User build artifacts, caches, and trash                        │
│ • Excluded:   /Library/Caches and sandboxed app containers                   │
│                                                                              │
│ [ Start Scan with Standard Access ]                                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens

- **Surface:** `--surface-raised` (`#ffffff` light / `#2a3632` dark).
- **Border:** `1px solid var(--divider)` (`#dfe5e2` light / `#2e3a36` dark).
- **Icon / Lock:** `--class-protected-fg` (`#283632` light / `#ccd5d1` dark).
- **Primary action button (`Grant Access…`):** `--accent` background (`#3f7567` light / `#82b8a8` dark). Opens `x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles`.
- **Secondary action button:** `--surface` background with `--text-primary` border.

---

### 1.2 Scoped / Partially granted permissions

#### Context

macOS Privacy & Security allows granting access to specific folders (e.g. "Files and Folders" permission for Downloads, Documents, or Desktop) without granting Full Disk Access. In this state, DiskClearance inspects precisely those authorized folders while omitting unauthorized roots.

#### Visual presentation

The interface presents an informational callout specifying exact scope boundaries:

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ℹ Scoped Folder Access Active                                          [ Modify Access… ]    │
│   DiskClearance has access to 4 authorized folder scopes. Root and container paths are       │
│   omitted from analysis. Scanned totals reflect only these authorized directories.           │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Scopes: [✓] ~/Downloads  [✓] ~/Library/Caches  [✕] /Library/Caches  [✕] ~/Library/Containers │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ℹ Scoped Folder Access Active                              [ Modify… ]       │
│   Access granted to 4 folder scopes. System paths are omitted.               │
│   • Included: ~/Downloads, ~/Library/Caches                                  │
│   • Omitted:  /Library/Caches, ~/Library/Containers                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 1.3 Permission revoked mid-scan

#### Context and behavior

A user may open macOS System Settings and toggle off Full Disk Access or remove a folder permission while a scan is actively traversing the filesystem. The Rust core traversal loop receives `EPERM` or `EACCES` on subsequent directory reads.

DiskClearance adheres to the following safety behavior:

1. **The application never crashes or halts ungracefully.** Traversal catches permission errors per directory.
2. **Already verified findings are retained.** Every item verified prior to revocation remains in memory and SQLite.
3. **The traversal loop emits a `CoverageWarning` event** specifying the blocked path.
4. **The scan transitions to "Completed with Restricted Scopes".**
5. **The storage total is visibly qualified** to indicate that subsequent folders were inaccessible.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Permission Revoked During Scan                                       [ Check Settings… ]   │
│   Access permissions were changed while scanning ~/Library/Containers. Traversal stopped     │
│   safely. 142 items verified before revocation are preserved.                                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Scan Status: Completed with restricted scopes                                                │
│ Verified reclaimable: 3.40 GB (Partial · excludes 3 restricted scopes)                       │
│ Inaccessible scope:   ~/Library/Containers/com.synthetic.sandbox/Data (Permission denied)    │
│                                                                                              │
│ [ Review 142 Verified Items ]                                   [ Run Fresh Scan ]           │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Permission Revoked During Scan                           [ Settings… ]     │
│   Permissions changed mid-scan. 142 items verified before revocation         │
│   are preserved. Totals are partial.                                         │
├──────────────────────────────────────────────────────────────────────────────┤
│ Verified: 3.40 GB (Partial · excludes 3 scopes)                              │
│ Blocked:  ~/Library/Containers (Permission denied)                           │
│                                                                              │
│ [ Review 142 Items ]                                   [ Fresh Scan ]        │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens

- **Callout background:** `--class-review-bg` (`#faecc6` light / `#382b13` dark).
- **Callout foreground:** `--class-review-fg` (`#5c3e00` light / `#ffd78a` dark).
- **Border:** `1px solid var(--divider)`.

---

## Surface 2: Scan states

### 2.1 Scanning (Active streaming)

#### Context

During an active scan, DiskClearance streams real-time telemetry from the Rust core: active phase (`discovering`, `scanning`, `classifying`, `coalescing`), currently visited filesystem scope, items visited, and bytes inspected.

The view maintains a non-jittering tabular display and provides an immediate, prominent cancel button.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Scanning Macintosh HD…                                                  [ Cancel Scan ]      │
│ Phase: Scanning build directories and application caches                                     │
│ Current scope: ~/Library/Caches/com.synthetic.developer/DerivedData                          │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 42%                      │
│                                                                                              │
│ Visited items:     18,420 entries                                                            │
│ Inspected data:    24.18 GB                                                                  │
│ Reclaimable found: 142 items (3.40 GB verified)                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Scanning Macintosh HD…                                     [ Cancel ]        │
│ Phase: Scanning caches (42%)                                                 │
│ Scope: ~/Library/Caches/com.synthetic.developer/DerivedData                  │
├──────────────────────────────────────────────────────────────────────────────┤
│ [████████████████████░░░░░░░░░░░░░░░░] 42%                                   │
│ Visited: 18,420 entries · Reclaimable: 142 items (3.40 GB)                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 2.2 Paused scan

#### Context

A scan may be paused by user request to temporarily free disk I/O (e.g. while launching a compiler or heavy process) or when thermal/battery constraints dictate throttling. Pausing suspends traversal threads at the next directory boundary without dropping session state or verified findings.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Scan Paused                                                             [ Resume Scan ]      │
│ Traversal suspended at ~/Library/Caches/com.synthetic.developer                              │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Current session progress:                                                                    │
│ • 18,420 items visited across 8 scopes                                                       │
│ • 142 reclaimable items verified (3.40 GB)                                                   │
│ • Memory and session state preserved                                                         │
│                                                                                              │
│ [ Resume Scan ]                                             [ Stop & Review Findings ]       │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Scan Paused                                                [ Resume ]        │
│ Suspended at: ~/Library/Caches/com.synthetic.developer                       │
├──────────────────────────────────────────────────────────────────────────────┤
│ Progress: 18,420 visited · 142 reclaimable (3.40 GB)                         │
│ State preserved in memory.                                                   │
│                                                                              │
│ [ Resume Scan ]                                     [ Review 142 ]           │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 2.3 Cancelled mid-scan

#### Context and behavior

When a user clicks `[ Cancel Scan ]`, traversal halts cleanly within a bounded execution budget. Crucially, **verified findings found before cancellation are retained and presented for review**. The UI explicitly notes that totals are partial and do not reflect the complete volume.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Scan Stopped by User                                                                         │
│ Traversal was halted before checking all scopes. 142 verified items are ready to review.     │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Verified Partial Scope:                                                                      │
│ • 142 reclaimable items verified (3.40 GB)                                                   │
│ • Note: Totals are partial. 4 configured scopes were not scanned.                            │
│                                                                                              │
│ [ Review 142 Verified Items ]                                   [ Start New Scan ]           │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Scan Stopped by User                                                         │
│ Traversal halted. 142 items verified before stopping are preserved.          │
├──────────────────────────────────────────────────────────────────────────────┤
│ Partial scope: 142 items (3.40 GB). 4 scopes were not scanned.               │
│                                                                              │
│ [ Review 142 Items ]                                   [ New Scan ]          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 2.4 Scan failed (Fatal scanner error)

#### Context

A scan may fail due to hardware I/O faults, sudden drive unmounting, or database corruption. In accordance with the locked constraint:

1. It explains what happened: The underlying error cause.
2. It explains what was not done: Which scopes were aborted.
3. It explains what the user can do: Remedial steps and diagnostics.

Red (`--class-irreversible-bg` / `--class-irreversible-fg`) appears here because this represents genuine, unrecoverable runtime failure.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ✕ Scan Failed: Filesystem I/O Error                                     [ Try Again ]        │
│ Target volume: Macintosh HD                                                                  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ What happened:                                                                               │
│ The scan could not complete because the filesystem reported a read error while traversing    │
│ ~/Library/Caches/com.synthetic.corrupt.                                                      │
│                                                                                              │
│ What was not done:                                                                           │
│ Analysis of developer caches and system logs was aborted. No findings were saved.            │
│                                                                                              │
│ What you can do:                                                                             │
│ 1. Verify disk integrity in macOS Disk Utility (First Aid).                                  │
│ 2. Ensure your user account has read permissions for the target cache directory.             │
│ 3. Export diagnostics to inspect the low-level failure trace.                                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Export Diagnostic Report ]                                    [ Try Scanning Again ]       │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ✕ Scan Failed: Filesystem I/O Error                        [ Retry ]         │
│ Macintosh HD read failure at ~/Library/Caches/com.synthetic.corrupt          │
├──────────────────────────────────────────────────────────────────────────────┤
│ What happened: Traversal aborted due to an operating system I/O error.       │
│ What was not done: Scan was aborted; no new findings were saved.             │
│ What you can do: Run Disk Utility First Aid, then retry scan.                │
│                                                                              │
│ [ Export Diagnostics ]                                 [ Try Again ]         │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens

- **Header background:** `--class-irreversible-bg` (`#fae3e3` light / `#3b191b` dark).
- **Header text / Icon:** `--class-irreversible-fg` (`#7f1d1d` light / `#fca5a5` dark).
- **Body surface:** `--surface-raised`.

---

### 2.5 Completed with errors (Restricted / Skipped scopes)

#### Context

The scan successfully traversed all accessible directories, but encountered restricted or unreadable paths (e.g. `EACCES`, locked files). Rather than reporting blanket success, DiskClearance presents a **completed with errors** summary.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Scan Completed with 3 Inaccessible Locations                                                 │
│ 142 items verified (3.40 GB). 3 directories could not be read due to permissions.            │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Inaccessible Scopes:                                                                         │
│ • /Library/Caches/com.synthetic.system               Permission denied (EACCES)              │
│ • ~/Library/Containers/com.synthetic.sandbox/Data   Requires Full Disk Access                │
│ • /var/log/synthetic_daemon.log                     Protected system path                    │
│                                                                                              │
│ Note: Totals exclude these inaccessible locations.                                           │
│                                                                                              │
│ [ Review 142 Verified Items ]                                   [ Inspect Permissions ]      │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Scan Completed with 3 Inaccessible Locations                                 │
│ 142 items verified (3.40 GB). 3 locations were skipped.                      │
├──────────────────────────────────────────────────────────────────────────────┤
│ Inaccessible:                                                                │
│ • /Library/Caches/com.synthetic.system (Permission denied)                   │
│ • ~/Library/Containers/com.synthetic.sandbox (FDA required)                  │
│ • /var/log/synthetic_daemon.log (Protected system path)                      │
│                                                                              │
│ [ Review 142 Items ]                               [ Permissions… ]          │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 2.6 Found nothing (Clean disk / Zero findings)

#### Context

The scan inspected all requested scopes and found zero safe cleanup opportunities.
**Crucial rule:** This is not an error and is never red. It communicates reassurance in calm neutral and accent tones.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Your Mac Is Clean                                                                            │
│ No safe cleanup opportunities were found in the scanned scopes.                              │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Verification Summary:                                                                        │
│ • Scanned data: 42.10 GB across 12 standard locations                                        │
│ • Reclaimable caches: 0 B                                                                    │
│ • Obsolete build outputs: 0 B                                                                │
│                                                                                              │
│ All inspected files are currently active, protected, or below reclaimable thresholds.        │
│                                                                                              │
│ [ Explore Storage Structure ]                                   [ Done ]                     │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Your Mac Is Clean                                                            │
│ No cleanup opportunities found in scanned scopes.                            │
├──────────────────────────────────────────────────────────────────────────────┤
│ Scanned: 42.10 GB across 12 locations.                                       │
│ Reclaimable: 0 B found. All files are active or protected.                   │
│                                                                              │
│ [ Explore Storage ]                                            [ Done ]      │
└──────────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens

- **Surface:** `--surface-raised`.
- **Heading color:** `--text-primary`.
- **Icon / Badge:** `--accent` loop icon (`↺`) or check mark. Never red.

---

## Surface 3: Empty states

### 3.1 Clean disk (Home & Cleanup view)

When a user navigates to Cleanup but no items were detected or all items have been cleaned, the view confirms disk cleanliness without alarm.

#### Copy and reassurance

- Headline: `"No Cleanup Items Found"`
- Subtitle: `"All scanned directories are clear of temporary build outputs, stale caches, and disposable logs."`
- Primary button: `[ Run New Scan ]`

---

### 3.2 Filtered view with no matches

#### Context

A user types a query in the search field (e.g. `"docker"`) or activates filter chips (`Class: Review`, `Size > 1 GB`) that result in zero matching rows.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Cleanup                                                Search: [ docker               ] ✕    │
│ Active filters: [ Class: Review ✕ ] [ Size > 1 GB ✕ ]                                        │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                              │
│                           No findings match 'docker'                                         │
│   No items in the current scan match your search term and active filter criteria.            │
│                                                                                              │
│                           [ Reset Filters and Search ]                                       │
│                                                                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Cleanup                                         Search: [ docker     ]       │
│ Filters: [ Review ✕ ] [ > 1 GB ✕ ]                                           │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│                     No findings match 'docker'                               │
│         No items match current filters and search criteria.                  │
│                                                                              │
│                     [ Reset Filters ]                                        │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 3.3 Area / Category with no findings (e.g. Developer & AI)

#### Context

When viewing a specific category (such as Developer & AI) on a Mac that does not have development tools installed, DiskClearance explains the absence factually rather than presenting a generic blank screen.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Developer & AI Storage                                                                       │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ No Developer or AI Tools Detected                                                            │
│ DiskClearance looked for build caches, containers, and model weights from:                   │
│ • Xcode DerivedData, Archives, and CoreSimulator devices                                     │
│ • Docker, Podman, and containerd image caches                                                │
│ • HuggingFace, Ollama, and PyTorch model stores                                              │
│ • Cargo, npm, yarn, pip, and CocoaPods package caches                                        │
│                                                                                              │
│ None of these tools were detected on Macintosh HD. If you install them later, run a new      │
│ scan to inspect their storage footprints.                                                    │
│                                                                                              │
│ [ View All Findings in Cleanup ]                                                             │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Developer & AI Storage                                                       │
├──────────────────────────────────────────────────────────────────────────────┤
│ No Developer or AI Tools Detected                                            │
│ Looked for Xcode, Docker, package caches, and local AI model stores.         │
│ None were detected on this Mac.                                              │
│                                                                              │
│ [ View All Findings ]                                                        │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Surface 4: Error states (Action and Runtime errors)

Constraint: **An error says what happened, what was not done, and what the user can do. Never a code alone, never "something went wrong".**

### 4.1 Path vanished between scan and action (`pathVanished`)

#### Context

Between initial scan and deletion confirmation, a background process or another user deleted the target file (ENOENT). Pre-flight revalidation or Trash execution catches this mismatch.

#### Presentation

- **What happened:** `"Xcode ModuleCache was deleted or moved by another process after the scan."`
- **What was not done:** `"This item was skipped. No modifications were attempted."`
- **What you can do:** `"No action needed. Remaining items in your selection were processed normally."`

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Item Skipped: Target No Longer Exists on Disk                                                │
│ ~/Library/Caches/com.synthetic.developer/ModuleCache                                         │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  The target path vanished before it could be moved to Trash. Another        │
│                   process (such as Xcode clean) likely removed it.                           │
│ • What was done:  The item was skipped safely. No filesystem errors occurred.                │
│ • Action:         No action required. Your selection plan has been adjusted.                 │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Continue Review ]                                                                          │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Item Skipped: Target No Longer Exists                                        │
│ ~/Library/Caches/com.synthetic.developer/ModuleCache                         │
├──────────────────────────────────────────────────────────────────────────────┤
│ • What happened: Path vanished before deletion; another process removed it   │
│ • What was done: Skipped safely without modification.                        │
│ • Action:        No action needed.                                           │
│                                                                              │
│ [ Continue ]                                                                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 4.2 Permission denied on delete (`permissionDenied` / EACCES)

#### Context

DiskClearance attempted to move a file to Trash or permanently delete it, but macOS returned `EACCES` (write/execute permission denied or immutable flag set).

#### Presentation

- **What happened:** `"macOS denied permission to delete SyntheticService.log."`
- **What was not done:** `"The file was left untouched in /Library/Logs/SyntheticService.log."`
- **What you can do:** `"Check file ownership in Finder or grant Full Disk Access in System Settings."`

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ✕ Deletion Failed: Permission Denied (EACCES)                                                │
│ Target: /Library/Logs/SyntheticService.log                                                   │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  macOS refused write permission to unlink this file.                        │
│ • What was done:  The file was left untouched in its original location.                      │
│ • What you can do:Verify file permissions in Finder ('Get Info') or grant Full Disk          │
│                   Access to DiskClearance in macOS System Settings.                          │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Open System Settings… ]                                       [ Dismiss ]                  │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ✕ Deletion Failed: Permission Denied (EACCES)                                │
│ Target: /Library/Logs/SyntheticService.log                                   │
├──────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  Write permission refused by macOS.                         │
│ • What was done:  File was left untouched in original location.              │
│ • What you can do:Check Finder permissions or grant Full Disk Access.        │
│                                                                              │
│ [ System Settings… ]                                       [ Dismiss ]       │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 4.3 Disk full during Trash move (ENOSPC / Storage exhaustion)

#### Context

Moving files to macOS Trash on an APFS volume requires allocating directory entries and metadata. When a volume is 100% full (0 bytes free), moving to Trash fails with `ENOSPC`.

#### Presentation

- **What happened:** `"The volume Macintosh HD has 0 bytes available. macOS cannot allocate metadata to move items to Trash."`
- **What was not done:** `"18 items (2.40 GB) were not moved to Trash."`
- **What you can do:** `"Use 'Delete Now' for direct permanent removal without Trash allocation, or free space manually in Finder."`

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ✕ Cannot Move to Trash: Volume is Completely Full                                            │
│ Macintosh HD has 0 bytes available                                                           │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  macOS requires free disk space to create directory records in Trash.       │
│                   Because the volume is exhausted, moving to Trash was rejected.             │
│ • What was done:  18 selected items (2.40 GB) were left intact in their original places.     │
│ • What you can do:Option 1: Use 'Delete Now' from the overflow menu to permanently unlink    │
│                   files immediately without using Trash storage.                             │
│                   Option 2: Manually delete large files in Finder to free workspace.         │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ View Remaining 18 Items ]                                     [ Back to Cleanup ]          │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ✕ Cannot Move to Trash: Volume Full                                          │
│ Macintosh HD has 0 bytes available                                           │
├──────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  macOS cannot allocate Trash records on a full disk.        │
│ • What was done:  18 items (2.40 GB) were left untouched.                    │
│ • What you can do:Use Delete Now to permanently unlink without Trash,        │
│                   or free space manually.                                    │
│                                                                              │
│ [ View Items ]                                            [ Dismiss ]        │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 4.4 Item locked by running process (`EBUSY` / Active file handle)

#### Context

A running application or system daemon holds an active, exclusive lock on a log, database, or cache file.

#### Presentation

- **What happened:** `"SyntheticStudio (PID 49102) is actively using session.db."`
- **What was not done:** `"The locked database and its parent folder were skipped."`
- **What you can do:** `"Quit SyntheticStudio, then try removing the item again."`

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Item Locked by Running Application                                                         │
│ Target: ~/Library/Application Support/SyntheticStudio/session.db                             │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  This file is held open with an exclusive lock by SyntheticStudio           │
│                   (Process ID: 49102).                                                       │
│ • What was done:  The locked file was left untouched.                                        │
│ • What you can do:Quit SyntheticStudio completely, then retry deletion.                      │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Retry After Quitting App ]                                    [ Skip This Item ]           │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Item Locked by Running Application                                         │
│ Target: SyntheticStudio session.db                                           │
├──────────────────────────────────────────────────────────────────────────────┤
│ • What happened: File is locked by SyntheticStudio (PID 49102).              │
│ • What was done: Skipped without modification.                               │
│ • What you can do: Quit SyntheticStudio, then retry.                         │
│                                                                              │
│ [ Retry ]                                              [ Skip Item ]         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 4.5 Volume unmounted mid-operation

#### Context

An external APFS drive or disk image (`/Volumes/SyntheticBackup`) was unplugged or unmounted while DiskClearance was executing a deletion plan.

#### Presentation

- **What happened:** `"The volume 'SyntheticBackup' was disconnected during execution."`
- **What was not done:** `"18 items located on this volume were not processed."`
- **What you can do:** `"Reconnect 'SyntheticBackup' and refresh the review plan."`

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ✕ Volume Disconnected Mid-Operation                                                          │
│ Volume: /Volumes/SyntheticBackup                                                             │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  The volume was unmounted while items were being moved to Trash.            │
│ • What was done:  25 items completed before disconnection (3.10 GB moved).                   │
│                   18 items on SyntheticBackup were skipped (2.40 GB untouched).              │
│ • What you can do:Reconnect the external drive to inspect or resume remaining items.         │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ View Completed Items ]                                        [ Return to Cleanup ]        │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ✕ Volume Disconnected Mid-Operation                                          │
│ Volume: /Volumes/SyntheticBackup                                             │
├──────────────────────────────────────────────────────────────────────────────┤
│ • What happened:  Volume unmounted during deletion.                          │
│ • What was done:  25 items moved (3.10 GB); 18 items skipped (2.40 GB).      │
│ • What you can do:Reconnect drive to inspect or resume.                      │
│                                                                              │
│ [ View Completed ]                                     [ Return ]            │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Surface 5: Degraded states and worked examples

### 5.1 Partial scan with qualified totals (Worked Example 1)

#### The rule

**An incomplete scan never presents its totals as complete.** Every figure derived from partial coverage says what it excluded. This is the same rule the first-run document applies to its two summary figures.

#### Demonstrating the claim: Complete vs Partial scan

To demonstrate this claim truthfully, we examine the same synthetic Mac under two conditions:

1. **Complete scan (All 3 scopes accessible):**
   - User Caches (`~/Library/Caches`): `3.40 GB` verified.
   - System Caches (`/Library/Caches`): `5.80 GB` verified.
   - Developer & AI Storage (`~/Library/Developer`): `5.00 GB` verified.
   - **Total verified reclaimable: `14.20 GB`** (Clean, unconditional total: `14.20 GB across 3 of 3 scopes`).

2. **Partial scan (2 scopes restricted due to Standard Access mode / permissions):**
   - User Caches (`~/Library/Caches`): `3.40 GB` verified.
   - Developer Storage (`~/Library/Developer/DerivedData` only): `2.90 GB` verified.
   - System Caches (`/Library/Caches`): **Excluded** (Permission denied).
   - Sandboxed Containers (`~/Library/Containers`): **Excluded** (FDA required).
   - **Total verified reclaimable: `6.30 GB`** (Qualified total: `6.30 GB (Partial · excludes 2 restricted scopes)`).

Notice that the two figures differ: `14.20 GB` (complete) versus `6.30 GB` (partial).
If the partial scan simply displayed `"Reclaimable: 6.30 GB"`, the user would be deceived into believing that their entire Mac contains only `6.30 GB` of reclaimable storage. DiskClearance strictly renders the qualification inline and displays the exclusion breakdown directly beneath the total.

#### Visual layout: Partial scan qualified total — 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Cleanup Overview                                                       [ Standard Access ]   │
│ Target volume: Macintosh HD                                                                  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Verified Reclaimable: 6.30 GB (Partial · excludes 2 restricted scopes)                       │
│ Breakdown: 3.40 GB User Caches · 2.90 GB Developer Data                                      │
│ Excluded scopes:                                                                             │
│ 🔒 /Library/Caches             Permission denied (System-wide cache)                          │
│ 🔒 ~/Library/Containers        Requires Full Disk Access (App sandboxes)                      │
│                                                                                              │
│ Note: Figures reflect only accessible directories. Grant Full Disk Access to scan all.       │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Grant Full Disk Access… ]                                     [ Review 6.30 GB ]           │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: Partial scan qualified total — 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Cleanup Overview                                           [ Standard ]      │
│ Macintosh HD                                                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│ Verified: 6.30 GB (Partial · excludes 2 scopes)                              │
│ Excluded: /Library/Caches and ~/Library/Containers (Access required)         │
│ Figures omit restricted directories.                                         │
│                                                                              │
│ [ Grant Full Disk Access… ]                          [ Review 6.30 GB ]      │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 5.2 Stale results warning

#### Context

A user completed a scan 3 days ago. The cache remains in SQLite, but the local filesystem has likely changed since the scan occurred (new build artifacts created, caches cleared by tools).

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Stale Scan Results · Scanned 3 days ago (Sep 15, 2026)               [ Run Fresh Scan ]    │
│   Files on disk may have changed since this scan. Actions from stale results will undergo    │
│   pre-flight revalidation to catch modified or vanished targets.                             │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Cached Findings: 142 items (3.40 GB)                                                         │
│                                                                                              │
│ [ Run Fresh Scan (Recommended) ]                                [ Review Cached Findings]    │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ ⚠ Stale Scan Results (Sep 15, 2026)                        [ Rescan ]        │
│   Files on disk may have changed. Revalidation will check targets.           │
├──────────────────────────────────────────────────────────────────────────────┤
│ Cached findings: 142 items (3.40 GB)                                         │
│                                                                              │
│ [ Run Fresh Scan ]                                  [ Review Cached ]        │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 5.3 Interrupted operation resumed

#### Context

The computer shut down or the application was terminated while a 43-item deletion operation was in flight. On relaunch, DiskClearance discovers an incomplete operation record in SQLite and presents a recovery audit.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Interrupted Operation Recovery                                                               │
│ An operation moving 43 items to Trash was interrupted by system shutdown.                    │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Operation Accounting:                                                                        │
│ • Completed before interruption: 25 items (3.10 GB) safely moved to Trash                    │
│ • Interrupted target:            1 item (200 MB) left in place                               │
│ • Not attempted:                 17 items (2.20 GB) untouched in original locations          │
│                                                                                              │
│ Status of Completed Items:                                                                   │
│ 25 items reside safely in macOS Trash and can be restored in Finder if desired.              │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Dismiss & Keep Current State ]                                [ Resume Remaining 18 ]      │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Interrupted Operation Recovery                                               │
│ Moving 43 items to Trash was interrupted by shutdown.                        │
├──────────────────────────────────────────────────────────────────────────────┤
│ • Completed: 25 items (3.10 GB moved to Trash)                               │
│ • Interrupted: 1 item (200 MB untouched)                                     │
│ • Untouched: 17 items (2.20 GB untouched)                                    │
│                                                                              │
│ [ Dismiss ]                                         [ Resume 18 ]            │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Surface 6: Loading and transitional states

### 6.1 First paint (Cold launch)

- **Render target:** Immediate paint under 80ms.
- **Visual wireframe:** Window chrome and sidebar render immediately with skeleton cards in the content area using `--surface-raised` and `1px solid var(--divider)`.
- **Numerical placeholders:** Storage figures render as quiet em-dashes (`-- GB`) using `--text-secondary`. No spinning loaders on cold launch.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ [Brand Mark] DiskClearance                Macintosh HD · Reading volume…             [?]     │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ Initializing Diagnostic Core…                                                                │
│ Loading local database and discovering default filesystem roots…                             │
│                                                                                              │
│ ┌───────────────────────────┐  ┌───────────────────────────┐  ┌────────────────────────┐     │
│ │ User Caches               │  │ Developer Storage         │  │ System Logs            │     │
│ │ -- GB                     │  │ -- GB                     │  │ -- GB                  │     │
│ └───────────────────────────┘  └───────────────────────────┘  └────────────────────────┘     │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ DiskClearance                               Reading volume…                  │
├──────────────────────────────────────────────────────────────────────────────┤
│ Initializing core and discovering storage roots…                             │
│ • User Caches:        -- GB                                                  │
│ • Developer Storage:  -- GB                                                  │
│ • System Logs:        -- GB                                                  │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

### 6.2 Long scan in progress

- **Throttled streaming:** Telemetry events are throttled at the IPC boundary to at most 10 updates per second.
- **Assistive technology rate limit:** Screen-reader updates (`aria-live="polite"`) occur every 5 seconds or every 5,000 items, preventing buffer floods.
- **Reduced motion mapping:** Animated sweeping progress bars become static percentage indicators.

---

### 6.3 Action in progress (Mutation execution)

When moving items to Trash or deleting permanently, a non-dismissible modal or docked sheet displays progress with a live cancel button.

#### Visual layout: 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Moving Items to Trash…                                                  [ Cancel Action ]    │
│ Target volume: Macintosh HD                                                                  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 58%                      │
│                                                                                              │
│ Moving item 25 of 43 items (3.10 GB of 5.50 GB)                                              │
│ Current target: ~/Library/Caches/com.synthetic.developer/ModuleCache                         │
│                                                                                              │
│ Press Cancel to halt safely between items. Items already moved will remain in Trash.         │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Moving Items to Trash…                                     [ Cancel ]        │
│ Progress: Item 25 of 43 (3.10 GB of 5.50 GB)                                 │
├──────────────────────────────────────────────────────────────────────────────┤
│ [████████████████████░░░░░░░░░░░░░░░░] 58%                                   │
│ Current: ~/Library/Caches/com.synthetic.developer/ModuleCache                │
│ Cancel halts safely between items.                                           │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Surface 7: The cancelled-mid-delete state (Worked Example 2)

### The rule

**A cancelled operation states exactly what completed before cancelling. A user who cancels a delete must know what was already deleted.**

### Demonstrating the claim: Concrete cancelled execution

To satisfy this requirement completely, we trace a concrete worked example:

1. **Initial confirmation scope:**
   - Total selected: `43 items` (`5.50 GB`).
   - Operation requested: `Move to Trash`.
2. **Cancellation trigger:**
   - The user presses `[ Cancel Action ]` during the processing of item 25.
   - The engine halts execution cleanly before mutating item 25.
3. **Execution accounting results:**
   - **Succeeded before cancellation:** `24 items` (`3.10 GB`) moved to Trash.
   - **Skipped due to cancellation:** `19 items` (`2.40 GB`) untouched in their original locations.
   - **Failed:** `0 items`.
   - **Sum verification:** `24 succeeded + 19 skipped = 43 total items`.
   - **Byte verification:** `3.10 GB moved + 2.40 GB untouched = 5.50 GB total`.
4. **Storage accounting results:**
   - **Pending in macOS Trash:** `3.10 GB` (from the 24 completed items).
   - **Permanently reclaimed immediately:** `0 B` (Trash operations do not free disk space until emptied).
5. **Reassurance in copy:**
   - Explicit confirmation that the remaining 19 items were left completely untouched.
   - Clear guidance that the 24 items in Trash are inspectable and restorable in Finder.

#### Visual layout: Cancelled-mid-delete state — 1100 × 720 px (Expanded)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ Operation Stopped by User                                                                    │
│ Target volume: Macintosh HD                                                                  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ ┌── Succeeded ──────┐  ┌── Untouched ──────┐  ┌── Failed ─────────┐  ┌── Blocked ────────┐   │
│ │ 24 items          │  │ 19 items          │  │ 0 items           │  │ 0 items           │   │
│ │ 3.10 GB moved     │  │ 2.40 GB skipped   │  │ 0 B failed        │  │ 0 B blocked       │   │
│ └───────────────────┘  └───────────────────┘  └───────────────────┘  └───────────────────┘   │
│                                                                                              │
│ Storage Status:                                                                              │
│ • Moved to macOS Trash: 3.10 GB (Recoverable in Finder; disk space freed only when empty)    │
│ • Left in place:        2.40 GB (19 items were not modified)                                 │
│ • Reclaimed on disk:    0 B                                                                  │
│                                                                                              │
│ The 24 items moved before cancellation remain in macOS Trash. The remaining 19 items         │
│ were not touched.                                                                            │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [ Open Trash in Finder ]                                         [ Back to Cleanup ]         │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout: Cancelled-mid-delete state — 760 × 560 px (Compact)

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Operation Stopped by User                                                    │
│ Target volume: Macintosh HD                                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│ • Succeeded: 24 items moved to Trash (3.10 GB)                               │
│ • Untouched: 19 items left in original location (2.40 GB)                    │
│ • Failed:    0 items                                                         │
│                                                                              │
│ Storage: 3.10 GB pending in Trash · 0 B freed on volume                      │
│ 24 items in Trash can be restored in Finder. 19 items untouched.             │
├──────────────────────────────────────────────────────────────────────────────┤
│ [ Open Trash in Finder ]                                  [ Return ]         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Final copy reference catalogue

The following copy is locked across all non-happy states:

| Surface / State                  | Exact Interface Copy                                                                                                                                                                                      |
| :------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Permission: Refused (Header)** | `"Standard Access Mode"`                                                                                                                                                                                  |
| **Permission: Refused (Body)**   | `"Operating with standard permissions. System caches and sandboxed containers are excluded from inspection. DiskClearance can inspect user build artifacts, caches, and trash without Full Disk Access."` |
| **Permission: Refused (Action)** | `"[ Grant Access… ]"`                                                                                                                                                                                     |
| **Permission: Scoped (Header)**  | `"Scoped Folder Access Active"`                                                                                                                                                                           |
| **Permission: Scoped (Body)**    | `"DiskClearance has access to authorized folder scopes. Root and container paths are omitted from analysis. Scanned totals reflect only these authorized directories."`                                   |
| **Permission: Revoked (Header)** | `"Permission Revoked During Scan"`                                                                                                                                                                        |
| **Permission: Revoked (Body)**   | `"Access permissions were changed while scanning. Traversal stopped safely. Verified items found before revocation are preserved."`                                                                       |
| **Scan: Paused (Header)**        | `"Scan Paused"`                                                                                                                                                                                           |
| **Scan: Paused (Body)**          | `"Traversal suspended at current scope. Progress and verified findings are preserved in memory."`                                                                                                         |
| **Scan: Cancelled (Header)**     | `"Scan Stopped by User"`                                                                                                                                                                                  |
| **Scan: Cancelled (Body)**       | `"Traversal was halted before checking all scopes. Verified items found before stopping are ready to review."`                                                                                            |
| **Scan: Failed (Header)**        | `"Scan Failed: Filesystem I/O Error"`                                                                                                                                                                     |
| **Scan: Failed (Body)**          | `"The scan could not complete because the filesystem reported a read error while traversing the target directory."`                                                                                       |
| **Scan: Errors (Header)**        | `"Scan Completed with Inaccessible Locations"`                                                                                                                                                            |
| **Scan: Errors (Body)**          | `"Scan finished with inaccessible locations. Totals exclude these skipped directories."`                                                                                                                  |
| **Empty: Clean Disk (Header)**   | `"Your Mac Is Clean"`                                                                                                                                                                                     |
| **Empty: Clean Disk (Body)**     | `"No safe cleanup opportunities were found in the scanned scopes. All inspected files are currently active or protected."`                                                                                |
| **Empty: Filtered (Header)**     | `"No findings match your filters"`                                                                                                                                                                        |
| **Empty: Filtered (Body)**       | `"No items in the current scan match your search term and active filter criteria."`                                                                                                                       |
| **Empty: Category (Header)**     | `"No Developer or AI Tools Detected"`                                                                                                                                                                     |
| **Empty: Category (Body)**       | `"Build caches, containers, and local AI model weights were not found on this volume."`                                                                                                                   |
| **Error: Vanished (Header)**     | `"Item Skipped: Target No Longer Exists on Disk"`                                                                                                                                                         |
| **Error: Vanished (Body)**       | `"The target path vanished before it could be moved to Trash. Another process likely removed it. The item was skipped safely."`                                                                           |
| **Error: Permission Denied**     | `"macOS refused write permission to unlink this file. The file was left untouched in its original location."`                                                                                             |
| **Error: Disk Full (Header)**    | `"Cannot Move to Trash: Volume is Completely Full"`                                                                                                                                                       |
| **Error: Disk Full (Body)**      | `"macOS requires free disk space to create directory records in Trash. 0 bytes remain on the volume. Use Delete Now or free space manually."`                                                             |
| **Error: Process Lock (Body)**   | `"This file is held open with an exclusive lock by an active application. Quit the application completely, then retry."`                                                                                  |
| **Error: Volume Unmounted**      | `"The volume was disconnected while an operation was in progress. Completed items were preserved; remaining items were skipped."`                                                                         |
| **Degraded: Partial Total**      | `"Verified Reclaimable: 6.30 GB (Partial · excludes 2 restricted scopes)"`                                                                                                                                |
| **Degraded: Stale Warning**      | `"Stale Scan Results · Scanned 3 days ago. Files on disk may have changed since this scan."`                                                                                                              |
| **Degraded: Interrupted**        | `"An operation moving items to Trash was interrupted by system shutdown. 25 items moved to Trash; 18 items left untouched."`                                                                              |
| **Cancelled Mutation (Body)**    | `"The 24 items moved before cancellation remain in macOS Trash. The remaining 19 items were not touched."`                                                                                                |

---

## Screen-reader and assistive technology catalogue

Every non-happy state is fully announced to VoiceOver and assistive technology using semantic ARIA live roles:

| Surface / State             | Role            | Live Region             | Spoken Announcement Text                                                                                              |
| :-------------------------- | :-------------- | :---------------------- | :-------------------------------------------------------------------------------------------------------------------- |
| **Permission Refused**      | `role="region"` | `aria-live="polite"`    | "Standard Access Mode active. System caches and sandboxed containers excluded from scan. Button: Grant Access."       |
| **Permission Revoked**      | `role="alert"`  | `aria-live="assertive"` | "Alert: Permission revoked during scan. Traversal stopped safely. 142 items verified before revocation preserved."    |
| **Scan Paused**             | `role="status"` | `aria-live="polite"`    | "Scan paused at 18,420 entries. 142 reclaimable items verified. Traversal suspended."                                 |
| **Scan Cancelled**          | `role="status"` | `aria-live="polite"`    | "Scan stopped by user. 142 verified items preserved for review. Totals are partial."                                  |
| **Scan Failed**             | `role="alert"`  | `aria-live="assertive"` | "Scan failed: Filesystem I/O error on Macintosh HD. Traversal aborted. No findings saved."                            |
| **Completed with Errors**   | `role="status"` | `aria-live="polite"`    | "Scan completed with 3 inaccessible locations. 142 items verified, 3.40 GB total. Totals exclude restricted scopes."  |
| **Clean Disk**              | `role="status"` | `aria-live="polite"`    | "Your Mac is clean. No cleanup opportunities found across 12 scanned locations. All files active or protected."       |
| **Filtered View Empty**     | `role="status"` | `aria-live="polite"`    | "No findings match filter docker. Zero results. Button: Reset Filters."                                               |
| **Category Empty**          | `role="region"` | `aria-live="polite"`    | "No developer or AI tools detected on Macintosh HD."                                                                  |
| **Path Vanished**           | `role="status"` | `aria-live="polite"`    | "Notice: Target path vanished before deletion. Item skipped safely."                                                  |
| **Permission Denied**       | `role="alert"`  | `aria-live="assertive"` | "Error: Permission denied on delete for SyntheticService.log. File left untouched."                                   |
| **Disk Full Error**         | `role="alert"`  | `aria-live="assertive"` | "Error: Cannot move to Trash. Volume Macintosh HD is completely full with 0 bytes free."                              |
| **Process Locked**          | `role="alert"`  | `aria-live="assertive"` | "Warning: File locked by SyntheticStudio process ID 49102. Skipped without modification."                             |
| **Volume Disconnected**     | `role="alert"`  | `aria-live="assertive"` | "Error: Volume disconnected mid-operation. 25 items moved to Trash, 18 items skipped."                                |
| **Partial Total Announced** | `role="text"`   | `aria-label`            | "Verified reclaimable: 6.30 GB partial total, excludes 2 restricted scopes."                                          |
| **Cancelled Delete**        | `role="status"` | `aria-live="assertive"` | "Operation stopped by user. 24 items moved to Trash, 3.10 GB. 19 items left untouched in original location, 2.40 GB." |

---

## Reduced Motion specifications

In accordance with `docs/design/ACCESSIBILITY.md` and `src/App.css`, DiskClearance provides comprehensive adaptations for `prefers-reduced-motion: reduce`:

| Component / State                | Standard Motion Experience                         | Reduced Motion Experience                            | Accessibility Rationale                            |
| :------------------------------- | :------------------------------------------------- | :--------------------------------------------------- | :------------------------------------------------- |
| **Scanning Indicator**           | Continuous pulsing progress bar and activity sweep | Static progress bar with explicit percentage (`42%`) | Prevents ocular fatigue and vestibular distraction |
| **Permission Banner Reveal**     | 160ms slide-down and fade (`translateY(-8px)`)     | Instant appearance (0ms duration)                    | Eliminates layout shifting during screen paint     |
| **Filter Empty State**           | 120ms fade-in transition                           | Instant cut (0ms)                                    | Direct state representation without flicker        |
| **Error Callout Alert**          | 140ms scale bounce (`scale: 0.98` to `1.0`)        | Static bordered callout with immediate display       | Avoids sudden jarring motion on error occurrence   |
| **Deletion Progress Bar**        | Smooth CSS transition tween between steps          | Step-based numerical update (`Item 25 of 43`)        | Removes rapid layout animations                    |
| **Cancelled Summary Transition** | 180ms cross-fade to outcome table                  | Instant replacement (0ms)                            | Clean, stable post-cancellation audit view         |

### CSS implementation pattern

```css
@media (prefers-reduced-motion: reduce) {
  .scan-progress-bar,
  .permission-banner,
  .empty-state-view,
  .error-callout,
  .cancelled-summary-panel {
    transition: none !important;
    animation: none !important;
    transform: none !important;
  }
}
```

---

## Design tokens and styling reference

All visual styles use the semantic design tokens defined in `src/App.css` and documented in `docs/design/TOKENS.md`:

| Token Name                | Light Appearance | Dark Appearance | Component Role                                      |
| :------------------------ | :--------------- | :-------------- | :-------------------------------------------------- |
| `--window`                | `#f4f2ec`        | `#19211f`       | Window background canvas, modal backdrop scrim      |
| `--surface`               | `#faf9f5`        | `#222c29`       | List background, view surface                       |
| `--surface-raised`        | `#ffffff`        | `#2a3632`       | Callout bodies, cards, banner surfaces              |
| `--divider`               | `#dfe5e2`        | `#2e3a36`       | Border lines, card dividers, table separators       |
| `--text-primary`          | `#21312d`        | `#e4ece8`       | Primary headlines, item names, tabular numerals     |
| `--text-secondary`        | `#5e6d68`        | `#9eaca6`       | Explanatory copy, secondary metadata, notes         |
| `--accent`                | `#3f7567`        | `#82b8a8`       | Primary actions, clean disk status icon             |
| `--accent-soft`           | `#dce9e3`        | `#263d36`       | Informational callouts, neutral status pills        |
| `--accent-fg`             | `#24483e`        | `#cae6dc`       | High-contrast accent labels on soft backgrounds     |
| `--class-rebuildable-bg`  | `#daf0e4`        | `#1e3b2e`       | Rebuildable item badge background                   |
| `--class-rebuildable-fg`  | `#164e3a`        | `#8ce4bd`       | Rebuildable item badge text                         |
| `--class-review-bg`       | `#faecc6`        | `#382b13`       | Review badge, partial/degraded callout background   |
| `--class-review-fg`       | `#5c3e00`        | `#ffd78a`       | Review badge, partial/degraded callout text         |
| `--class-protected-bg`    | `#e1e6e5`        | `#26302e`       | Protected lock background, standard access badge    |
| `--class-protected-fg`    | `#283632`        | `#ccd5d1`       | Protected lock icon and text                        |
| `--class-irreversible-bg` | `#fae3e3`        | `#3b191b`       | Fatal error banner background, unrecoverable action |
| `--class-irreversible-fg` | `#7f1d1d`        | `#fca5a5`       | Fatal error banner text, unrecoverable action text  |

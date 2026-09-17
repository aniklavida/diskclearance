# Calm first-run flow specification

## Overview

The first run establishes user trust on macOS. It introduces DiskClearance not as an aggressive cleaner or security suite, but as a calm, read-only diagnostic tool that never mutates the filesystem without explicit review and approval.

Every layout, colour token, spacing rule, and control specified here uses the design tokens defined in `docs/design/TOKENS.md` and implemented in `src/App.css`.

---

## Window and viewport specifications

DiskClearance supports macOS 13 Ventura and later across two canonical window sizes:

1. **Minimum window (compact breakpoint):** `760 × 560 px`
   - Sidebar width: `200px` (or collapsed icon-label rail when required).
   - Content area: `560 × 560 px` (padding: `--space-24` = `24px`).
   - Vertical density: tighter padding (`--space-16` / `--space-24`), stacked metrics where horizontal span is tight.
2. **Common window (expanded breakpoint):** `1100 × 720 px`
   - Sidebar width: `220px`.
   - Content area: `880 × 720 px` (max readable width: `790px`, padding: `--space-48` = `48px`).
   - Two-column or side-by-side metric layouts where appropriate.

---

## The seven-step locked sequence

```text
1. Welcome ──► 2. Scan Mac ──► 3. Contextual permission (if needed)
                                         │
                                         ▼
7. Honest completion ◄── 6. Review ◄── 5. Three calm sections ◄── 4. Streaming results
```

---

### Step 1: Welcome

#### Purpose
Establish immediate reassurance in a single sentence: DiskClearance inspects storage and removes nothing on its own. It is neither a feature tour nor an onboarding pitch.

#### Visual layout (760 × 560 px)
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├───────────────────┬────────────────────────────────────────────────────────┤
│ [Brand Mark]      │                                                        │
│ DiskClearance     │  [Status: Read-only]                                  │
│                   │                                                        │
│ • Home (active)   │  # See what goes.                                      │
│ • Cleanup         │    Keep what matters.                                  │
│ • Explore         │                                                        │
│ • Applications    │  A scan inspects disk usage and removes nothing.       │
│ • History         │                                                        │
│                   │  ┌──────────────────────────────────────────────────┐  │
│                   │  │ [ Primary Action: Continue ]                     │  │
│                   │  └──────────────────────────────────────────────────┘  │
│                   │                                                        │
│                   │  Calm, inspectable, local to this Mac.                 │
└───────────────────┴────────────────────────────────────────────────────────┘
```

#### Visual layout (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │                                                                           │
│ DiskClearance        │  [Status Pill: Read-only diagnostic]                                     │
│                      │                                                                           │
│ • Home (active)      │  # See what goes.                                                         │
│ • Cleanup            │    Keep what matters.                                                     │
│ • Explore            │                                                                           │
│ • Applications       │  A scan inspects disk usage and removes nothing.                          │
│ • History            │                                                                           │
│                      │  ┌─────────────────────────────────┐                                      │
│                      │  │ [ Continue ]                    │                                      │
│                      │  └─────────────────────────────────┘                                      │
│                      │                                                                           │
│                      │  All analysis stays local. Nothing is deleted without your confirmation. │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens
- **Surface:** `--window` (`#f4f2ec` light / `#19211f` dark).
- **Status pill:**
  - Light: background `--accent-soft` (`#dce9e3`), foreground `--accent-fg` (`#24483e`).
  - Dark: background `--accent-soft` (`#263d36`), foreground `--accent-fg` (`#cae6dc`).
  - Typography: 12px, font-weight 700, letter-spacing 0.04em, uppercase.
- **Headline (h1):**
  - Font size: clamp(36px, 5vw, 64px), line-height 1.05, tracking -0.04em.
  - Color: `--text-primary` (`#21312d` light / `#e4ece8` dark).
- **Lede / reassurance text:**
  - Font size: 18px, line-height 1.5, color: `--text-secondary` (`#5e6d68` light / `#9eaca6` dark).
  - Margin: `--space-16` top, `--space-32` bottom.
- **Primary action button (`Continue`):**
  - Min height: `--target-primary` (`40px`), padding: `0 var(--space-24)`.
  - Border radius: `--radius-control` (`8px`).
  - Light: background `--accent` (`#3f7567`), text `#ffffff`.
  - Dark: background `--accent` (`#82b8a8`), text `#19211f`.
  - Focus ring: 2px solid `--accent` with 2px offset.

---

### Step 2: Scan Mac

#### Purpose
Provide a single, uncluttered primary action. No scope pickers, no checkbox matrices, and no advanced settings disclosures competing for attention.

#### Visual layout (760 × 560 px)
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├───────────────────┬────────────────────────────────────────────────────────┤
│ [Brand Mark]      │                                                        │
│ DiskClearance     │  Home                                                  │
│                   │                                                        │
│ • Home (active)   │  ## Storage overview                                   │
│ • Cleanup         │                                                        │
│ • Explore         │  ┌──────────────────────────────────────────────────┐  │
│ • Applications    │  │ Macintosh HD                                     │  │
│ • History         │  │ 245.2 GB used of 494.4 GB                        │  │
│                   │  │ [■■■■■■■■■■■■■□□□□□□□□□□□□□]                     │  │
│                   │  │ Capacity: 494.4 GB · Available: 249.2 GB         │  │
│                   │  └──────────────────────────────────────────────────┘  │
│                   │                                                        │
│                   │  ┌──────────────────────────────────────────────────┐  │
│                   │  │ [ Primary Action: Scan Mac ]                     │  │
│                   │  └──────────────────────────────────────────────────┘  │
│                   │  Read-only traversal · Bounded system impact           │
└───────────────────┴────────────────────────────────────────────────────────┘
```

#### Visual layout (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │                                                                           │
│ DiskClearance        │  Home                                                                     │
│                      │                                                                           │
│ • Home (active)      │  ## Storage overview                                                      │
│ • Cleanup            │                                                                           │
│ • Explore            │  ┌─────────────────────────────────────────────────────────────────────┐  │
│ • Applications       │  │ Macintosh HD                                                        │  │
│ • History            │  │ 245.2 GB used of 494.4 GB (50% free)                                │  │
│                      │  │ [■■■■■■■■■■■■■■■■■■■■■□□□□□□□□□□□□□□□□□□□□]                         │  │
│                      │  │ System, developer caches, and user data                             │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
│                      │                                                                           │
│                      │  ┌─────────────────────────────────┐                                      │
│                      │  │ [ Scan Mac ]                    │                                      │
│                      │  └─────────────────────────────────┘                                      │
│                      │                                                                           │
│                      │  Read-only inspection. Never modifies or removes files during scan.       │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens
- **Drive capacity card:**
  - Background: `--surface` (`#faf9f5` light / `#222c29` dark).
  - Border: 1px solid `--divider` (`#dfe5e2` light / `#2e3a36` dark).
  - Radius: `--radius-card` (`12px`), padding: `--space-24`.
- **Capacity bar:**
  - Height: `8px`, border-radius: `--radius-full` (`999px`).
  - Track: `--divider`. Fill: `--accent`.
- **Scan Mac button:**
  - Height: `44px`, font-size: 15px, font-weight: 600.
  - Background: `--accent`, text color: accessible contrast foreground.

---

### Step 3: Contextual permission

#### Purpose
Permissions are requested **only when an active scan scope strictly requires them**, never at initial application launch. The dialog explains both sides honestly: what access allows, and what the application continues to inspect without it.

#### Visual layout at minimum window (760 × 560 px)
In the 760 × 560 px window, the permission explanation renders as a centered overlay sheet with reduced margin:
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├────────────────────────────────────────────────────────────────────────────┤
│ ┌── Centered Sheet (520 × 440 px) ───────────────────────────────────────┐ │
│ │ Full Disk Access for developer and system caches                       │ │
│ │                                                                        │ │
│ │ DiskClearance can inspect standard user caches right now. Access lets  │ │
│ │ us inspect protected application containers and Xcode simulator data. │ │
│ │                                                                        │ │
│ │ ┌─ What access adds ─────────────────────────────────────────────────┐ │ │
│ │ │ • Xcode simulators and device support caches                       │ │ │
│ │ │ • Isolated container caches in ~/Library/Containers                │ │ │
│ │ │ • Homebrew and system package manager caches                       │ │ │
│ │ ├─ What works without it ────────────────────────────────────────────┤ │ │
│ │ │ • Standard user caches (~/Library/Caches)                          │ │ │
│ │ │ • Application build outputs in your home directory                 │ │ │
│ │ │ • Trash inspection and user download folder review                 │ │ │
│ │ └────────────────────────────────────────────────────────────────────┘ │ │
│ │                                                                        │ │
│ │ ┌───────────────────────────┐  ┌─────────────────────────────────────┐ │ │
│ │ │ Continue with limited scan│  │ Open System Settings                │ │ │
│ │ └───────────────────────────┘  └─────────────────────────────────────┘ │ │
│ └────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────┘
```

#### Visual layout at common window (1100 × 720 px)
In the 1100 × 720 px window, the sheet is centered over the dim backdrop of the Home view, allowing surrounding context to remain faintly visible:
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │                                                                           │
│ DiskClearance        │  ┌── Contextual Permission Sheet (540 × 460 px) ────────────────────────┐ │
│                      │  │ Full Disk Access for developer and system caches                     │ │
│ • Home (active)      │  │                                                                      │ │
│ • Cleanup            │  │ DiskClearance can inspect standard user caches right now.            │ │
│ • Explore            │  │ Access allows inspection of protected application containers,        │ │
│ • Applications       │  │ Xcode simulator caches, and system build artifacts.                  │ │
│ • History            │  │                                                                      │ │
│                      │  │ ┌─ What access adds ───────────────────────────────────────────────┐ │ │
│                      │  │ │ • Xcode simulators and device support caches                     │ │ │
│                      │  │ │ • Isolated container caches in ~/Library/Containers              │ │ │
│                      │  │ │ • Homebrew and system package manager caches                     │ │ │
│                      │  │ ├─ What works without it ──────────────────────────────────────────┤ │ │
│                      │  │ │ • Standard user caches (~/Library/Caches)                        │ │ │
│                      │  │ │ • Application build outputs in your home directory               │ │ │
│                      │  │ │ • Trash inspection and user download folder review               │ │ │
│                      │  │ └──────────────────────────────────────────────────────────────────┘ │ │
│                      │  │                                                                      │ │
│                      │  │ Open System Settings to grant access, or continue with standard      │ │
│                      │  │ user coverage.                                                       │ │
│                      │  │                                                                      │ │
│                      │  │ ┌─────────────────────────┐     ┌──────────────────────────────────┐ │ │
│                      │  │ │ Continue limited scan   │     │ Open System Settings             │ │ │
│                      │  │ └─────────────────────────┘     └──────────────────────────────────┘ │ │
│                      │  └──────────────────────────────────────────────────────────────────────┘ │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens
- **Overlay backdrop:** `rgba(0, 0, 0, 0.4)` light, `rgba(0, 0, 0, 0.65)` dark.
- **Sheet surface:**
  - Light: background `--surface-raised` (`#ffffff`), border 1px solid `--divider` (`#dfe5e2`), shadow `--card-shadow` (`0 18px 50px rgba(49, 71, 64, 0.08)`).
  - Dark: background `--surface-raised` (`#2a3632`), border 1px solid `--divider` (`#2e3a36`), shadow `--card-shadow` (`0 18px 50px rgba(0, 0, 0, 0.35)`).
  - Radius: `--radius-surface` (`16px`), padding: `--space-32`.
- **Feature comparison box:**
  - Background: `--surface` (`#faf9f5` light / `#222c29` dark).
  - Internal divider: 1px solid `--divider`.
  - Radius: `--radius-control` (`8px`), padding: `--space-16`.
  - Headings: 13px, font-weight 650, color `--text-primary`.
  - Bullet items: 13px, color `--text-secondary`, line-height 1.45.
- **Buttons:**
  - Primary (`Open System Settings`): height `--target-primary` (`40px`), padding: `0 var(--space-24)`, radius `--radius-control` (`8px`), background `--accent`, text accessible contrast.
  - Secondary (`Continue with limited scan`): height `--target-primary` (`40px`), padding: `0 var(--space-16)`, radius `--radius-control` (`8px`), border 1px solid `--divider`, background transparent, text `--text-primary`.

---

### Step 4: Streaming results

#### Purpose
Provide truthful, calm progress reporting: current traversal phase, active filesystem scope, and verified items found. Cancellation is immediate, thread-safe, and retains all verified findings without discarding work.

#### Visual layout at minimum window (760 × 560 px)
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├───────────────────┬────────────────────────────────────────────────────────┤
│ [Brand Mark]      │                                                        │
│ DiskClearance     │  Scanning Mac…                                         │
│                   │                                                        │
│ • Home (active)   │  Phase: User caches and build outputs                  │
│ • Cleanup         │  Scope: ~/Library/Caches/com.example.developer         │
│ • Explore         │                                                        │
│ • Applications    │  [■■■■■■■■■■■■■■■■■■■■□□□□□□□□□□□□] 62%                │
│ • History         │                                                        │
│                   │  ┌──────────────────────────────────────────────────┐  │
│                   │  │ Verified candidates: 142 items (3.84 GB)         │  │
│                   │  │ Inaccessible paths: 3 (skipped safely)           │  │
│                   │  └──────────────────────────────────────────────────┘  │
│                   │                                                        │
│                   │  ┌────────────────────┐                                │
│                   │  │ [ Stop scan ]      │                                │
│                   │  └────────────────────┘                                │
│                   │  Stopping preserves already verified items.            │
└───────────────────┴────────────────────────────────────────────────────────┘
```

#### Visual layout at common window (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │                                                                           │
│ DiskClearance        │  Scanning Mac…                                                            │
│                      │                                                                           │
│ • Home (active)      │  Phase: User caches, simulator artifacts, and build outputs               │
│ • Cleanup            │  Scope: ~/Library/Caches/com.example.developer/DerivedData                │
│ • Explore            │                                                                           │
│ • Applications       │  [■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■□□□□□□□□□□□□□□□□□□] 62%                   │
│ • History            │                                                                           │
│                      │  ┌── Live Verified Findings ───────────────────────────────────────────┐  │
│                      │  │ Verified items: 142 items          Verified size: 3.84 GB           │  │
│                      │  │ Inaccessible paths: 3 (skipped)    Traversal rate: 840 files/sec    │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
│                      │                                                                           │
│                      │  ┌────────────────────┐                                                   │
│                      │  │ [ Stop scan ]      │                                                   │
│                      │  └────────────────────┘                                                   │
│                      │  Stopping preserves already verified items and advances to review.        │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens
- **Progress track and bar:**
  - Track: background `--divider` (`#dfe5e2` light / `#2e3a36` dark).
  - Fill: background `--accent` (`#3f7567` light / `#82b8a8` dark).
  - Height: `6px`, border-radius: `--radius-full` (`999px`).
  - Transition: `width 150ms ease-out` (disabled under `prefers-reduced-motion: reduce`).
- **Live metrics container:**
  - Background: `--surface` (`#faf9f5` light / `#222c29` dark).
  - Border: 1px solid `--divider`.
  - Radius: `--radius-card` (`12px`), padding: `--space-16`.
- **Numeric display:**
  - `font-variant-numeric: tabular-nums;` (class `.tabular-nums` / `.storage-figure`).
  - Weight: 650.
- **Stop scan button:**
  - Height: `--target-primary` (`40px`), padding: `0 var(--space-24)`.
  - Border: 1px solid `--divider`.
  - Background: transparent; on hover: `--accent-soft`.
  - Text color: `--text-primary`.

---

### Step 5: Three calm sections

#### Purpose
Present the scan results cleanly structured into three distinct safety categories:
1. **Ready to clear** (rebuildable caches, disposable build artifacts).
2. **Needs your review** (duplicates, orphan app support, uncertain related files).
3. **Protected** (system core, configuration files, active application bundles).

Their visual order and weight emphasize safety and reassurance, avoiding an alarming wall of items.

#### Visual layout at minimum window (760 × 560 px)
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├───────────────────┬────────────────────────────────────────────────────────┤
│ [Brand Mark]      │  Results summary                                       │
│ DiskClearance     │  Coverage: 84% of disk read (standard permissions)     │
│                   │                                                        │
│ • Home            │  [ Summary Metric Cards ]                              │
│ • Cleanup (active)│  Ready: 3.40 GB  │ Space after empty: 3.40 GB          │
│ • Explore         │                                                        │
│ • Applications    │  ▼ Ready to clear (Rebuildable)               3.40 GB  │
│ • History         │    ┌────────────────────────────────────────────────┐  │
│                   │    │ [✓] Xcode DerivedData                  2.10 GB │  │
│                   │    │ [✓] CocoaPods cache                    1.30 GB │  │
│                   │    └────────────────────────────────────────────────┘  │
│                   │                                                        │
│                   │  ▶ Needs your review (Review)                 1.80 GB  │
│                   │    2 items · Not selected automatically                │
│                   │                                                        │
│                   │  ▶ Protected (Safety lock)                   48.20 GB  │
│                   │    System files, git repositories, active apps         │
│                   │                                                        │
│                   │  ┌──────────────────────────────────────────────────┐  │
│                   │  │ [ Review selection: 3.40 GB ]                    │  │
│                   │  └──────────────────────────────────────────────────┘  │
└───────────────────┴────────────────────────────────────────────────────────┘
```

#### Visual layout at common window (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │  Cleanup > Scan results                                                   │
│ DiskClearance        │                                                                           │
│                      │  ┌── Dual Summary Metric Bar ──────────────────────────────────────────┐  │
│ • Home               │  │ [Icon: Trash] Ready to move to Trash: 3.40 GB (2 items)             │  │
│ • Cleanup (active)   │  │ [Icon: Disk]  Space available after Trash is emptied: 3.40 GB       │  │
│ • Explore            │  │ Coverage: At least 3.40 GB, from the 84% of your disk we could read │  │
│ • Applications       │  └─────────────────────────────────────────────────────────────────────┘  │
│ • History            │                                                                           │
│                      │  ▼ Ready to clear (Rebuildable cache)                          3.40 GB    │
│                      │    ┌─────────────────────────────────────────────────────────────────┐    │
│                      │    │ [✓] Xcode DerivedData (Rebuildable)                    2.10 GB  │    │
│                      │    │     Rebuildable cache · Safe to remove; rebuilt on build        │    │
│                      │    ├─────────────────────────────────────────────────────────────────┤    │
│                      │    │ [✓] CocoaPods cache (Rebuildable)                      1.30 GB  │    │
│                      │    │     Rebuildable cache · Downloaded package copies               │    │
│                      │    └─────────────────────────────────────────────────────────────────┘    │
│                      │                                                                           │
│                      │  ▶ Needs your review (Review needed)                           1.80 GB    │
│                      │    2 items · Kept unselected until you choose to inspect them             │
│                      │                                                                           │
│                      │  ▶ Protected (Protected by system policy)                     48.20 GB    │
│                      │    Core system files, versioned repositories, and installed applications  │
│                      │                                                                           │
│                      │  ┌── Floating Footer Tray ─────────────────────────────────────────────┐  │
│                      │  │ 2 items selected (3.40 GB)              [ Review selection ]        │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Safety class token assignment
From `src/App.css`:

| Section | Tokens (Light) | Tokens (Dark) | Visual Cue | Selection Default |
|---|---|---|---|---|
| **Ready to clear** | bg: `--class-rebuildable-bg` (`#daf0e4`)<br>fg: `--class-rebuildable-fg` (`#164e3a`) | bg: `--class-rebuildable-bg` (`#1e3b2e`)<br>fg: `--class-rebuildable-fg` (`#8ce4bd`) | Soft green badge + "Rebuildable" tag + checkable | Checked by default |
| **Needs your review** | bg: `--class-review-bg` (`#faecc6`)<br>fg: `--class-review-fg` (`#5c3e00`) | bg: `--class-review-bg` (`#382b13`)<br>fg: `--class-review-fg` (`#ffd78a`) | Amber badge + "Review" tag + warning cue | **Never** checked by default |
| **Protected** | bg: `--class-protected-bg` (`#e1e6e5`)<br>fg: `--class-protected-fg` (`#283632`) | bg: `--class-protected-bg` (`#26302e`)<br>fg: `--class-protected-fg` (`#ccd5d1`) | Neutral slate badge + lock icon + "Protected" | **Disabled / Uncheckable** |

---

### Step 6: Review

#### Purpose
Allow the user to inspect the planned operation with complete transparency. Items uncertain or marked as Review remain unselected unless explicitly toggled by the user.

#### Visual layout at minimum window (760 × 560 px)
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├───────────────────┬────────────────────────────────────────────────────────┤
│ [Brand Mark]      │  Review plan                                           │
│ DiskClearance     │                                                        │
│                   │  ## 2 items selected                                   │
│ • Home            │  Selected items will be moved to macOS Trash.          │
│ • Cleanup (active)│                                                        │
│ • Explore         │  ┌──────────────────────────────────────────────────┐  │
│ • Applications    │  │ [✓] Xcode DerivedData                   2.10 GB  │  │
│ • History         │  │     Rebuildable cache                            │  │
│                   │  ├──────────────────────────────────────────────────┤  │
│                   │  │ [✓] Package manager cache               1.30 GB  │  │
│                   │  │     Rebuildable cache                            │  │
│                   │  └──────────────────────────────────────────────────┘  │
│                   │                                                        │
│                   │  ┌── Sticky Action Tray ────────────────────────────┐  │
│                   │  │ Ready to move to Trash: 3.40 GB                  │  │
│                   │  │ [ Cancel ]       [ Move to Trash (3.40 GB) ]     │  │
│                   │  └──────────────────────────────────────────────────┘  │
└───────────────────┴────────────────────────────────────────────────────────┘
```

#### Visual layout at common window (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │  Cleanup > Review plan                                                    │
│ DiskClearance        │                                                                           │
│                      │  ## Review items to clear                                                 │
│ • Home               │  All selected items move to macOS Trash. Nothing is permanently removed   │
│ • Cleanup (active)   │  unless you choose Delete Now.                                            │
│ • Explore            │                                                                           │
│ • Applications       │  ┌─────────────────────────────────────────────────────────────────────┐  │
│ • History            │  │ [✓] Xcode DerivedData                                       2.10 GB │  │
│                      │  │     Rebuildable cache · ~/Library/Developer/Xcode/DerivedData        │  │
│                      │  ├─────────────────────────────────────────────────────────────────────┤  │
│                      │  │ [✓] Package manager cache                                   1.30 GB │  │
│                      │  │     Rebuildable cache · ~/Library/Caches/Homebrew                    │  │
│                      │  ├─────────────────────────────────────────────────────────────────────┤  │
│                      │  │ [ ] Unused iOS Simulators (Review)                         1.80 GB │  │
│                      │  │     Requires manual selection · Contains runtime image              │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
│                      │                                                                           │
│                      │  ┌── Action Selection Tray ────────────────────────────────────────────┐  │
│                      │  │ Ready to move to Trash:  3.40 GB (2 items)                          │  │
│                      │  │ Space available after Trash is emptied: 3.40 GB                     │  │
│                      │  │                                                                     │  │
│                      │  │ [ Back ]               [ Delete Now (Irreversible) ] [ Move to Trash ] │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens
- **Action selection tray:**
  - Border-top: 1px solid `--divider`. Background: `--surface-raised`.
  - Padding: `--space-16` `--space-24`.
- **Primary action:** `Move to Trash` (`--accent` fill, default action).
- **Secondary destructive action:** `Delete Now`
  - Uses `--class-irreversible-bg` (`#fae3e3` light / `#3b191b` dark) and `--class-irreversible-fg` (`#7f1d1d` light / `#fca5a5` dark).
  - Explicit irreversible warning label attached.

---

### Step 7: Honest completion

#### Purpose
Transparently report what changed, what was moved to Trash (which does not free disk space until emptied), and any errors or skipped files.

#### Visual layout at minimum window (760 × 560 px)
```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                      │
├───────────────────┬────────────────────────────────────────────────────────┤
│ [Brand Mark]      │                                                        │
│ DiskClearance     │  Cleanup complete                                      │
│                   │                                                        │
│ • Home            │  ## 2 items moved to Trash                             │
│ • Cleanup         │                                                        │
│ • Explore         │  ┌──────────────────────────────────────────────────┐  │
│ • Applications    │  │ Space moved to Trash: 3.40 GB                    │  │
│ • History (active)│  │ Space freed immediately: 0 B                     │  │
│                   │  │ (Trash must be emptied in macOS Finder to free)   │  │
│                   │  ├──────────────────────────────────────────────────┤  │
│                   │  │ Failures: 0 items                                │  │
│                   │  │ Skipped (changed during review): 0 items         │  │
│                   │  └──────────────────────────────────────────────────┘  │
│                   │                                                        │
│                   │  ┌───────────────────────┐  ┌───────────────────────┐  │
│                   │  │ [ View in History ]   │  │ [ Done ]              │  │
│                   │  └───────────────────────┘  └───────────────────────┘  │
└───────────────────┴────────────────────────────────────────────────────────┘
```

#### Visual layout at common window (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │  History > Operation record                                               │
│ DiskClearance        │                                                                           │
│                      │  ## Cleanup complete                                                      │
│ • Home               │  The planned items have been moved to macOS Trash.                        │
│ • Cleanup            │                                                                           │
│ • Explore            │  ┌── Operation Manifest Summary ───────────────────────────────────────┐  │
│ • Applications       │  │ Space moved to Trash: 3.40 GB (2 items)                             │  │
│ • History (active)   │  │ Disk space freed immediately: 0 B                                   │  │
│                      │  │ Space recoverable once Trash is emptied: 3.40 GB                    │  │
│                      │  ├─────────────────────────────────────────────────────────────────────┤  │
│                      │  │ Failed items: 0                                                     │  │
│                      │  │ Skipped items: 0 (no files vanished or changed after review)        │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
│                      │                                                                           │
│                      │  Items remain safely in your macOS Trash until you empty Trash in       │
│                      │  Finder. You can restore them from History at any time before then.    │
│                      │                                                                           │
│                      │  ┌──────────────────────┐                     ┌────────────────────────┐  │
│                      │  │ [ View audit log ]    │                     │ [ Done ]               │  │
│                      │  └──────────────────────┘                     └────────────────────────┘  │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

#### Styling and tokens
- **Summary card:**
  - Border: 1px solid `--divider`.
  - Background: `--surface`.
  - Number callouts: `.tabular-nums`, font-size 20px, font-weight 650.
- **Trash disclaimer:**
  - Notice text in `--text-secondary`, font-size 13px.
- **History reference:**
  - Allows full audit trail verification in accordance with `docs/SPEC.md`.

---

## The two summary figures

The two storage figures must never be combined into a single hopeful headline:
1. **Ready to move to Trash** (or *Items moved to Trash*): Space occupied by items that will be placed into the macOS Trash folder.
2. **Space available after Trash is emptied**: The real disk space that will be reclaimed once the user empties Trash.

Both figures explicitly qualify coverage whenever access is incomplete (e.g., *"At least 3.40 GB, from the 84% of your disk we could read"*).

---

## Verbatim copy catalogue

Every user-facing string in the first-run flow is locked below in quotation marks. Implementers must use these strings verbatim. Where multiple phrasing options were evaluated, both are provided with the rationale for the recommended choice.

### Step 1: Welcome

- **Eyebrow status pill:**
  - `"Read-only diagnostic"`
- **Heading (h1):**
  - `"See what goes. Keep what matters."`
- **Reassurance body sentence:**
  - Primary recommendation: `"A scan inspects disk usage and removes nothing."`
    - *Rationale:* Concise, unambiguous, and immediately removes the anxiety of accidental deletion before the user even begins.
  - Evaluated alternative: `"DiskClearance examines your storage to find safe cleanup opportunities, without altering any files."`
    - *Comparison:* The alternative introduces unnecessary cleaner jargon ("cleanup opportunities") and dilutes the core safety promise.
- **Secondary caption / note:**
  - `"All analysis stays local. Nothing is deleted without your confirmation."`
- **Primary action button:**
  - Primary recommendation: `"Continue"`
    - *Rationale:* Standard calm macOS onboarding progression.
  - Evaluated alternative: `"Get Started"`
    - *Comparison:* "Get Started" sounds like an account setup or marketing wizard.

---

### Step 2: Scan Mac

- **Section eyebrow / navigation context:**
  - `"Home"`
- **Section heading (h2):**
  - `"Storage overview"`
- **Drive card title:**
  - `"Macintosh HD"`
- **Drive capacity and usage labels:**
  - Standard representation: `"{used} used of {total} ({free_pct}% free)"` (e.g., `"245.2 GB used of 494.4 GB (50% free)"`)
  - Subtitle: `"System, developer caches, and user data"`
- **Primary action button:**
  - Primary recommendation: `"Scan Mac"`
    - *Rationale:* Active, decisive, and sets clear scope without unnecessary words.
  - Evaluated alternative: `"Start Scan"`
    - *Comparison:* "Scan Mac" grounds the action in the device context and feels more native on macOS.
- **Footnote / safety reminder:**
  - `"Read-only inspection. Traversal runs at background priority and alters no files."`

---

### Step 3: Contextual permission

- **Sheet modal title:**
  - `"Full Disk Access for developer and system caches"`
- **Primary explanation paragraph:**
  - `"DiskClearance can inspect standard user caches right now. Granting access allows inspection of protected application containers, Xcode simulator runtimes, and package manager caches."`
- **Feature comparison section 1 heading:**
  - `"What access adds:"`
- **Feature comparison section 1 items:**
  - Item 1: `"Xcode simulators and device support caches"`
  - Item 2: `"Isolated application container caches in ~/Library/Containers"`
  - Item 3: `"Homebrew and system package manager caches"`
- **Feature comparison section 2 heading:**
  - `"What works without it:"`
- **Feature comparison section 2 items:**
  - Item 1: `"Standard user caches (~/Library/Caches)"`
  - Item 2: `"Application build outputs in your home folder"`
  - Item 3: `"macOS Trash inspection and user download reviews"`
- **Instructions paragraph:**
  - `"Open System Settings to grant access, or continue with standard user coverage."`
- **Primary action button:**
  - `"Open System Settings"`
- **Secondary action button:**
  - Primary recommendation: `"Continue with limited scan"`
    - *Rationale:* Plainly tells the user that the scan will proceed immediately with whatever permissions they already have.
  - Evaluated alternative: `"Skip for now"`
    - *Comparison:* "Skip for now" fails to clarify that the scan will still proceed and still be genuinely useful.

---

### Step 4: Streaming results

- **Screen heading:**
  - `"Scanning Mac…"`
- **Phase indicator strings:**
  - Phase 1: `"Phase: Standard user caches"`
  - Phase 2: `"Phase: Developer build outputs and package caches"`
  - Phase 3: `"Phase: Simulator runtimes and container caches"`
  - Phase 4: `"Phase: Reconciling verified findings"`
- **Current scope format:**
  - `"Scope: {path}"` (e.g., `"Scope: ~/Library/Caches/com.example.developer/DerivedData"`)
- **Live metric labels:**
  - Verified items: `"Verified items: {count}"` (e.g., `"Verified items: 142 items"`)
  - Verified size: `"Verified size: {size}"` (e.g., `"Verified size: 3.84 GB"`)
  - Inaccessible paths: `"Inaccessible paths: {count} (skipped)"` (e.g., `"Inaccessible paths: 3 (skipped)"`)
  - Traversal speed: `"Traversal rate: {count} files/sec"` (e.g., `"Traversal rate: 840 files/sec"`)
- **Stop button label:**
  - Default state: `"Stop scan"`
  - Hover / active feedback: `"Stop scan"`
  - Cancellation in progress: `"Stopping…"`
- **Cancellation explanation footnote:**
  - `"Stopping preserves already verified items and advances to review."`
- **Cancelled confirmation notification banner:**
  - `"Scan stopped by user. {count} verified items ({size}) retained for review."`

---

### Step 5: Three calm sections

#### Summary header & coverage qualification
- **Summary header:**
  - `"Scan results"`
- **Full coverage badge:**
  - `"Full disk read (100% accessible)"`
- **Partial coverage disclaimer:**
  - `"At least {reclaimable_size}, from the {coverage_pct}% of your disk we could read"` (e.g., `"At least 3.40 GB, from the 84% of your disk we could read"`)

#### Summary figure 1: Ready to move to Trash
- **Label:**
  - `"Ready to move to Trash"`
- **Quantity format:**
  - `"{size} ({count} items)"` (e.g., `"3.40 GB (2 items)"`)
- **Contextual footnote:**
  - `"Items moved to Trash remain restorable until you empty Trash in Finder."`

#### Summary figure 2: Space available after Trash is emptied
- **Label:**
  - `"Space available after Trash is emptied"`
- **Quantity format:**
  - `"{size}"` (e.g., `"3.40 GB"`)
- **Contextual footnote:**
  - `"Disk capacity increases only after Trash is emptied in macOS Finder."`

#### Section 1: Ready to clear
- **Section title:**
  - `"Ready to clear"`
- **Category badge text:**
  - `"Rebuildable"`
- **Section explanatory description:**
  - `"Rebuildable caches and disposable build artifacts. Safe to remove; applications recreate these automatically when needed."`
- **Default state:**
  - All items in this section are selected by default.
- **Empty state copy:**
  - Title: `"No rebuildable items found"`
  - Subtitle: `"Your application and developer caches are already tidy."`

#### Section 2: Needs your review
- **Section title:**
  - `"Needs your review"`
- **Category badge text:**
  - `"Review"`
- **Section explanatory description:**
  - `"Items requiring your explicit decision. Kept unselected until you choose to inspect them."`
- **Default state:**
  - All items in this section are unselected by default.
- **Empty state copy:**
  - Title: `"No items need review"`
  - Subtitle: `"No duplicate files or orphan application data were detected."`

#### Section 3: Protected
- **Section title:**
  - `"Protected"`
- **Category badge text:**
  - `"Protected"`
- **Section explanatory description:**
  - `"Core system files, version control repositories, and active applications. Never selected or modified."`
- **Default state:**
  - Locked and uncheckable.
- **Summary copy inside card:**
  - `"{size} protected by safety policy ({count} items)"` (e.g., `"48.20 GB protected by safety policy (1,420 items)"`)

#### Primary review action button
- **Button label:**
  - `"Review selection ({selected_size})"` (e.g., `"Review selection (3.40 GB)"`)
- **Disabled state label (when 0 items selected):**
  - `"Select items to review"`

---

### Step 6: Review

- **Screen navigation context:**
  - `"Cleanup > Review plan"`
- **Screen heading (h2):**
  - `"Review items to clear"`
- **Instructional description:**
  - `"All selected items will be moved to macOS Trash. Nothing is permanently removed unless you choose Delete Now."`
- **Table column headers:**
  - Column 1: `"Item"`
  - Column 2: `"Safety Class"`
  - Column 3: `"Location"`
  - Column 4: `"Size"`
- **Uncertain item badge / warning tooltip:**
  - `"Manual review required · Unselected by default"`
- **Review tray summary metrics:**
  - Line 1: `"Ready to move to Trash: {size} ({count} items)"`
  - Line 2: `"Space available after Trash is emptied: {size}"`
- **Action buttons in review tray:**
  - Navigation return button: `"Back"`
  - Default recoverable button: `"Move to Trash ({selected_size})"` (e.g., `"Move to Trash (3.40 GB)"`)
  - Irreversible secondary button: `"Delete Now (Irreversible)"`

#### Destructive modal: Delete Now confirmation
- **Dialog heading:**
  - `"Permanently delete {count} items?"`
- **Warning text:**
  - `"These files will be removed immediately from your disk without using the macOS Trash. You cannot undo this action or restore these files."`
- **Confirm button:**
  - `"Delete Immediately"`
- **Cancel button:**
  - `"Cancel"`

---

### Step 7: Honest completion

#### Variant A: Standard Trash cleanup (Default path)
- **Screen navigation context:**
  - `"History > Operation record"`
- **Heading (h2):**
  - `"Cleanup complete"`
- **Subheading / primary result:**
  - `"{count} items moved to Trash"` (e.g., `"2 items moved to Trash"`)
- **Metric breakdowns in manifest summary:**
  - Metric 1 label: `"Space moved to Trash"`
    - Metric 1 value: `"{size}"` (e.g., `"3.40 GB"`)
  - Metric 2 label: `"Disk space freed immediately"`
    - Metric 2 value: `"0 B"`
  - Metric 3 label: `"Space recoverable once Trash is emptied"`
    - Metric 3 value: `"{size}"` (e.g., `"3.40 GB"`)
- **Audit line:**
  - `"Failed items: 0 · Skipped items: 0 (no files vanished or changed during review)"`
- **Reassurance reminder body:**
  - `"Items remain safely in your macOS Trash until you empty Trash in Finder. You can restore them from History at any time before then."`
- **Action buttons:**
  - Secondary: `"View audit log"`
  - Primary: `"Done"`

#### Variant B: Permanent deletion (Delete Now path)
- **Screen navigation context:**
  - `"History > Operation record"`
- **Heading (h2):**
  - `"Permanent deletion complete"`
- **Subheading / primary result:**
  - `"{count} items permanently deleted"` (e.g., `"2 items permanently deleted"`)
- **Metric breakdowns in manifest summary:**
  - Metric 1 label: `"Disk space freed immediately"`
    - Metric 1 value: `"{size}"` (e.g., `"3.40 GB"`)
  - Metric 2 label: `"Pending in Trash"`
    - Metric 2 value: `"0 B"`
- **Audit line:**
  - `"Failed items: 0 · Skipped items: 0"`
- **Irreversible reminder body:**
  - `"Items were permanently removed from disk as requested. This operation cannot be undone. Full cryptographic manifest saved in local History."`
- **Action buttons:**
  - Secondary: `"View audit log"`
  - Primary: `"Done"`

#### Variant C: Partial failure or skipped items
- **Heading (h2):**
  - `"Cleanup completed with warnings"`
- **Subheading / primary result:**
  - `"{success_count} moved to Trash · {skipped_count} skipped"`
- **Explanatory text:**
  - `"{skipped_count} item changed on disk after your review and was skipped to protect your data."`
- **Skipped item detail line:**
  - `"{path} — modified after review snapshot"`
- **Audit line:**
  - `"Failed: 0 · Skipped: {skipped_count} · Succeeded: {success_count}"`
- **Action buttons:**
  - Secondary: `"View audit log"`
  - Primary: `"Done"`

---

## The two summary figures: visual distinction without reading labels

A critical failure mode in Mac storage tools is showing a single oversized number that conflates items placed into the Trash with immediately freed disk space. DiskClearance strictly treats **Ready to move to Trash** and **Space available after Trash is emptied** as two distinct physical concepts.

To ensure they are distinguishable **without reading the labels**, the design enforces three structural differentiators:

### 1. Spatial separation and physical container hierarchy
- **Ready to move to Trash** is an *action-bound staging figure*. It lives directly inside the interactive review and action controls (the floating action tray at the bottom of the viewport or adjacent to the primary action button). It moves with the user's immediate selection.
- **Space available after Trash is emptied** is a *system-bound volume figure*. It lives exclusively inside the drive storage overview card at the top of the screen, visually locked to the Macintosh HD capacity bar.

A person looking at the screen immediately perceives one number as belonging to the files on their tray, and the other as belonging to the persistent disk drive.

### 2. Iconographic and glyph cues
- **Ready to move to Trash** is always preceded by the macOS **Trash glyph** (a wireframe wastebasket) rendered in `--text-secondary`.
- **Space available after Trash is emptied** is always preceded by a **Drive Volume glyph** (a hardware disk icon with an outward reclaim arrow) or rendered directly inside a capacity bar gauge.

### 3. Visual weight, typography, and token contrast
- **Ready to move to Trash**:
  - Rendered in bold tabular numbers (`font-weight: 700; font-size: 20px;`).
  - Uses the soft rebuildable accent tint: background `--class-rebuildable-bg`, text `--class-rebuildable-fg`.
  - Accompanied by a count pill (e.g. `2 items`) indicating it is a collection of discrete files.
- **Space available after Trash is emptied**:
  - Rendered in neutral secondary typography (`font-weight: 500; font-size: 14px;`).
  - Uses `--text-secondary` with a subtle dashed underline or parenthetical condition `(after emptying Trash)`.
  - Never styled with action buttons or green accent tints, preventing the false belief that clicking "Move to Trash" instantly expands free space on the drive.

### Truthful handling of partial coverage
Whenever permissions prevent complete traversal of the disk (the standard macOS state), neither figure is presented as a confident exact total.
- The headline figure is prefixed with a mathematical lower-bound indicator: `"At least 3.40 GB"`.
- It is immediately paired with the coverage qualification token badge:
  ```text
  ┌────────────────────────────────────────────────────────────────────────┐
  │ [!] At least 3.40 GB, from the 84% of your disk we could read         │
  └────────────────────────────────────────────────────────────────────────┘
  ```
- The coverage percentage is calculated as `(scanned_bytes / total_used_bytes) * 100`, rounded down to avoid overstating coverage.
- If coverage is incomplete, an info pill using `--class-review-bg` and `--class-review-fg` explicitly informs the user: `"Standard coverage · Container and root caches not inspected"`.

---

## The permission-limited first run: primary macOS experience

On macOS, running without Full Disk Access (FDA) is the **common default case**, not an edge-case failure. Designing only the fully-privileged path results in brittle, intimidating experiences. DiskClearance treats standard permissions as a first-class, fully capable operating mode.

### User journey in standard permissions

```text
[Launch App]
     │
     ▼
[Step 1: Welcome] ──► Explains read-only nature
     │
     ▼
[Step 2: Scan Mac] ──► Starts scan with standard user scope
     │
     ▼
[Scope Inspection] ──► Encounters protected paths (~/Library/Containers, Xcode simulators)
     │
     ├─────────────────────────────────────────┐
     ▼                                         ▼
[Step 3: Contextual Permission Sheet]     [User chooses "Continue with limited scan"]
     │                                         │
     ▼                                         ▼
[Step 4: Streaming Results] ◄──────────────────┘
  • Inaccessible paths recorded calmly as skipped (e.g. "3 paths skipped safely")
  • No alarming red banners or system crash alerts
     │
     ▼
[Step 5: Results & Three Calm Sections]
  • Coverage badge: "At least 3.40 GB, from the 84% of your disk we could read"
  • Info banner: "Standard permission scope. Container caches were skipped."
  • Ready to clear shows verified user-level caches (~/Library/Caches)
     │
     ▼
[Step 6: Review] ──► Clear action on verified items only
     │
     ▼
[Step 7: Completion] ──► Explains exactly what was moved to Trash, noting standard scope
```

### Visual layout of the permission-limited results screen (1100 × 720 px)
```text
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [● ● ●] DiskClearance                                                                            │
├──────────────────────┬───────────────────────────────────────────────────────────────────────────┤
│ [Brand Mark]         │  Cleanup > Scan results                                                   │
│ DiskClearance        │                                                                           │
│                      │  ┌── Storage Overview (Standard Permission Scope) ─────────────────────┐  │
│ • Home               │  │ Macintosh HD: 245.2 GB used of 494.4 GB                                │  │
│ • Cleanup (active)   │  │ [Coverage: 84% read] At least 3.40 GB can be cleared                   │  │
│ • Explore            │  │ Space available after Trash is emptied: at least 3.40 GB               │  │
│ • Applications       │  ├─────────────────────────────────────────────────────────────────────┤  │
│ • History            │  │ ⓘ Standard permission coverage. System container and simulator      │  │
│                      │  │   caches were skipped. [Open System Settings to expand coverage]    │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
│                      │                                                                           │
│                      │  ▼ Ready to clear (Rebuildable)                                 3.40 GB    │
│                      │    ┌─────────────────────────────────────────────────────────────────┐    │
│                      │    │ [✓] User Application Caches                            2.10 GB  │    │
│                      │    │     Rebuildable cache · ~/Library/Caches                        │    │
│                      │    ├─────────────────────────────────────────────────────────────────┤    │
│                      │    │ [✓] Homebrew Package Cache                             1.30 GB  │    │
│                      │    │     Rebuildable cache · Safe to remove                          │    │
│                      │    └─────────────────────────────────────────────────────────────────┘    │
│                      │                                                                           │
│                      │  ▶ Needs your review (Review needed)                           0 items    │
│                      │    No ambiguous items detected in standard scope                          │
│                      │                                                                           │
│                      │  ▶ Protected (Safety policy)                                  48.20 GB    │
│                      │    System libraries, git repositories, and standard applications          │
│                      │                                                                           │
│                      │  ┌── Action Tray ──────────────────────────────────────────────────────┐  │
│                      │  │ 2 items selected (3.40 GB)              [ Review selection ]        │  │
│                      │  └─────────────────────────────────────────────────────────────────────┘  │
└──────────────────────┴───────────────────────────────────────────────────────────────────────────┘
```

### Key principles of the permission-limited flow
1. **Never nag or block:** If the user selects "Continue with limited scan", the application will never prompt them again during the current scan session.
2. **Truthful bounding:** All totals are labeled as lower bounds (`"At least…"`). The app never guesses or extrapolates what might be inside unread directories.
3. **Graceful in-line recovery:** An inline affordance (`[Open System Settings to expand coverage]`) allows the user to grant permissions later if they wish. When granted, the app re-scans only the previously skipped scopes and merges the results without restarting from scratch.

---

## Reduced Motion specifications

In accordance with macOS accessibility guidelines and the token principles in `docs/design/TOKENS.md`, DiskClearance provides comprehensive support for `prefers-reduced-motion: reduce`.

Reduced motion in DiskClearance does not merely remove CSS timing functions; it systematically replaces animated, distracting, or vestibular-triggering visual treatments with calm, stable, and instant visual state transitions.

### Component-by-component animation mapping

| Component / Action | Standard Motion Experience | Reduced Motion Experience | Rationale |
|---|---|---|---|
| **Window & View Transitions** | 180ms ease-out opacity fade and 4px vertical slide | Instantaneous cut (0ms transition) | Eliminates spatial displacement and perceived lag |
| **Scan Progress Bar** | Continuously animated CSS width fill with subtle shimmer | Discrete stepped width changes without smoothing; shimmer animation disabled | Continuous linear motion and shimmering can trigger vestibular distress |
| **Scan Progress Spinner** | Rotating indeterminate circular spinner | Static status glyph + tabular percentage readout (`"Scanning… 62%"`) | Constant circular rotation is removed |
| **Item Count & Size Tickers** | Rapidly counting numeric odometer effect | Discrete updates locked to `font-variant-numeric: tabular-nums` | Prevents continuous flashing and number flickering |
| **Section Accordions** (`Ready to clear`, `Protected`) | 200ms ease-out expand/collapse of container height | Instant toggle between expanded and collapsed states | Avoids large reflows and moving viewport boundaries |
| **Review Tray Appearance** | Smooth vertical slide-up from bottom (`translateY(100%)` to `translateY(0)`) | Fixed static position or immediate cut-in | Eliminates movement across the reading field |
| **Contextual Permission Sheet & Modals** | 200ms scale-up (`scale(0.96)` to `scale(1)`) and fade-in | Immediate visibility without scaling or opacity blending | Scaling modals trigger depth distortion |
| **Deletion Progress Indicator** | Continuous sweep progress bar | Stepped block progress bar with exact item count (`"Processed 3 of 12 items"`) | Replaces continuous sweep with factual discrete counts |

### CSS implementation pattern

The application enforces these rules via global stylesheet directives:

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
    scroll-behavior: auto !important;
  }

  /* Replace animated progress shimmers with static high-contrast fills */
  .progress-bar-fill {
    transition: none !important;
    background-image: none !important;
  }

  /* Disable rotational animation on spinners */
  .spinner {
    animation: none !important;
    display: none;
  }

  /* Ensure static indicators are displayed instead */
  .spinner-fallback {
    display: inline-block;
  }

  /* Modals appear instantly */
  .modal-sheet,
  .review-tray {
    transform: none !important;
    transition: none !important;
  }
}
```



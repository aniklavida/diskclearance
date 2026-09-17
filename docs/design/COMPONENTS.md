# Components

- App shell, sidebar, titlebar controls, and responsive compact navigation.
- Storage summary, capacity bar, reclaim summary, and scan coverage.
- Scan progress with phase, current scope, verified count, and cancel.
- Finding group, finding row, classification badge, evidence disclosure, and protected lock.
- File/folder table, breadcrumb, treemap, filter bar, and inspector.
- Application row, footprint summary, related-file evidence, and confidence label.
- Review tray, selection total, action-mode selector, and confirmation sheet.
- Operation summary, per-item result, pending-Trash notice, and restore control.
- Permission, empty, partial, cancelled, and error panels.

Components consume typed domain states. They must not infer safety classification from colour, file extension, or path text in the UI.

Detailed specifications for finding rows, groups, classification badges, evidence disclosures, application rows, and the Explore treemap/table pair are locked in `docs/design/FINDING_ROW_AND_TREEMAP.md`.

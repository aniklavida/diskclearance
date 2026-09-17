# Interface states

- **Empty:** explain what a scan does and that it removes nothing.
- **Scanning:** truthful indeterminate or measured progress with immediate cancellation.
- **Partial:** show verified results and name inaccessible scopes without pretending completion.
- **Permission-limited:** explain why access matters and allow continued limited use.
- **Cancelled:** preserve verified results and offer resume/new scan.
- **No opportunities:** confirm the scan scope and avoid artificial recommendations.
- **Changed after review:** block the item, explain the change, and require re-scan.
- **Partial operation failure:** separate success, failure, and skipped totals per item.
- **Pending in Trash:** show recoverable space separately from freed space.
- **Restore conflict:** never overwrite; offer a safe alternate name or cancel.
- **Database recovery:** preserve operation evidence where possible and provide a safe reset path.

Detailed specifications for non-happy states across permission refusals, scan interruptions, empty scopes, operation errors, degraded totals, and recovery are locked in `docs/design/NON_HAPPY_STATES.md`.

# Bug: Removing All Sources Does Not Clear Import Review

## Status
Resolved on 2026-04-02.

## Summary
When all source folders are removed from a workspace, the `Import Review` page could still show old mapping rows instead of becoming empty.

## Root Cause
Frontend cache and page state were only invalidated, not explicitly cleared, so the user could briefly re-enter `Import Review` and still see stale rows.

## Fix
- clear `import-review` query cache immediately when the last source is removed
- reset Browser-side filter and selection state when the workspace has no remaining sources
- clear cached Browser payload when the last source is removed
- show an explicit empty state in `Import Review`

## Expected Behavior After Fix
After the last source folder is removed:
- `Import Review` becomes empty immediately
- Browser filters and selection state are reset
- returning to `Import Review` does not show stale mapping rows

## Related Files
- `src/features/workspace/pages/SourcesPage.tsx`
- `src/features/workspace/pages/ImportReviewPage.tsx`
- `src/state/useWorkspaceStore.ts`

## Verification
- remove one source and confirm only related rows disappear after refetch
- remove all sources and confirm `Import Review` becomes empty immediately
- reopen the workspace and confirm no stale rows return

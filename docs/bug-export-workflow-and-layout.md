# Export UX And Layout Issues

## Status
Resolved on 2026-04-02.

## Bug 1: Export Flow Was Misaligned With Browser-First Workflow

### Summary
Export scope was previously redefined inside the `Export` page, which did not match the intended Browser-first workflow.

### Fix
- Browser now owns export scope
- users can start export from `Export Filtered` or `Export Selected` in `Browser`
- `Export` now focuses on format, split ratios, output path, conflict handling, and summary
- export preview and export execution now accept explicit `imageIds` scope from Browser

### Result
Users can now export exactly the current Browser result set or current Browser selection without rebuilding filters in `Export`.

### Related Files
- `src/features/workspace/pages/BrowserPage.tsx`
- `src/features/workspace/pages/ExportPage.tsx`
- `src/features/workspace/browserScope.ts`
- `src/types/workspace.ts`
- `src-tauri/src/models.rs`
- `src-tauri/src/workspace_service.rs`

## Bug 2: Export Page Conflict Handling Area Had Layout/Text Wrapping Problems

### Summary
The `Conflict Handling` section and nearby text had layout and wrapping issues.

### Fix
- removed the old duplicated export filter area from `Export`
- introduced export-specific layout sections for scope summary and conflict handling
- updated wrapping rules so long conflict paths and longer checkbox text no longer break layout

### Result
The `Export` page now reads as a settings-and-summary step, and the conflict area wraps cleanly on narrower layouts.

### Related Files
- `src/features/workspace/pages/ExportPage.tsx`
- `src/styles/index.css`

## Verification
- users can export based on Browser-visible scope without redefining filters on Export page
- users can export selected or filtered images clearly
- conflict handling and nearby helper text wrap cleanly on desktop and mobile widths

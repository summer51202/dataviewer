# DataViewer v0.1.0 Release Notes

## Summary
This release focuses on three areas: smoother browsing performance, a more practical selection/export workflow, and better annotation inspection visibility.

## Highlights
- Improved Browser and CVAT-related page responsiveness by reducing repeated heavy calculations and unnecessary re-renders under larger image sets.
- Added clearer guidance on the `Add Source` page for choosing the correct folder level for `COCO`, `YOLO`, and `RAW images` datasets.
- Fixed `Import Review` mapping not persisting correctly after save, and improved error feedback.
- Refined the `Export` workflow to be Browser-first.
  Users now filter or select images in Browser first, then export from `Export Selected`.
- Fixed Export page layout issues around `conflict handling` and nearby text wrapping.
- Fixed stale `Import Review` content remaining after removing all sources.
- Added stronger scan-time feedback when source scanning takes unusually long.
- Improved Browser image selection UX:
  - click once to select, click again to unselect
  - `Select All Filtered` / `Deselect Filtered`
  - `Clear Selection`
  - stronger selected-state visual feedback
- Added two annotation-aware Browser filters:
  - `Annotation Count`: fixed buckets `0 / 1 / 2 / 3 / 4 / 5+`
  - `Minimum Largest Box Ratio`
- Added richer thumbnail metadata:
  - annotation count
  - largest box ratio
  - per-box category and size ratio
- Added `Box Details` to the Image Detail metadata panel, showing each box category and area ratio.
- Added Windows packaging and teammate-installation docs, and fixed the Tauri Windows icon packaging issue.

## Behavior Changes
- `Export Filtered` has been removed.
- To export the current filtered result set, first use `Select All Filtered`, then use `Export Selected`.
- `Export Selected` is now the main export entry point.

## Documentation
- Windows packaging SOP:
  [windows-release-sop.md](/C:/EdwardLee/Project/DataViewer/docs/windows-release-sop.md)
- Teammate installation guide:
  [teammate-install-guide-zh-TW.md](/C:/EdwardLee/Project/DataViewer/docs/teammate-install-guide-zh-TW.md)

## Verification
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`

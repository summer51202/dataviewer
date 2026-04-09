# DataViewer v0.1.3 Release Notes

## Summary
This release packages a round of user-feedback-driven improvements across export workflow clarity, selection tools, and home page polish.

## Highlights
- Export default split ratios are now `80 / 15 / 5` for `train / valid / test`.
- Export now lets users browse for a custom output folder instead of typing the path manually.
- `Conflict Handling` now defaults to auto rename, while keeping a warning message visible when filename conflicts exist.
- `Export Scope` now shows category image ratios based on exportable annotated images in the current scope.
- `Export Summary` split counts now update with the current ratio settings instead of staying fixed at the old preview ratio.
- Home page now shows the app version beside the `DataViewer` title.
- Browser now supports `Shift` range selection and box selection on the currently loaded thumbnail grid.

## Behavior Changes
- New export split defaults are `train 80`, `valid 15`, `test 5`.
- Box selection applies only to thumbnails currently loaded in the Browser grid.
- Category ratio display in `Export Scope` is based on exportable annotated images, not the full scope image count.

## Upgrade Notes
- Existing workspace data does not need to be recreated for this update.
- Users can install the new `0.1.3` package over the previous version.
- If Windows reports the old version is in use or the upgrade fails, uninstall the previous version first and then install `0.1.3`.

## Recommended Verification
- Open `Export` and confirm the default split is `80 / 15 / 5`.
- Change split ratios and confirm `Export Summary` split counts update immediately.
- Use `Browse Folder` in `Export` and confirm a custom output path can be selected.
- Confirm filename conflicts still show a warning while auto rename is enabled by default.
- Confirm `Export Scope` shows category image ratios.
- Confirm the Home page shows `v0.1.3` beside the title.
- In Browser, test click select, `Shift` range select, and box selection on loaded thumbnails.

## Verification
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --tests`
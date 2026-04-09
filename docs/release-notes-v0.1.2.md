# DataViewer v0.1.2 Release Notes

## Summary
This release focuses on import resilience for mixed-quality COCO datasets and clearer project onboarding documentation.

## Highlights
- COCO import now accepts numeric strings inside `bbox` values, such as `"444.00"`, and converts them into numeric values during parsing.
- The same numeric-string tolerance is also applied to COCO-style annotation sync handling, reducing failures from inconsistent upstream exports.
- Added a new user-facing guide document that explains the product purpose, main workflow, and each primary page in the workspace.

## Behavior Changes
- COCO annotations with `bbox` arrays that mix JSON numbers and numeric strings are now treated as valid when the string values can be parsed as numbers.
- Non-numeric string values still remain invalid and are skipped.

## Upgrade Notes
- Existing workspace data does not need to be recreated for this update.
- Users can install the new `0.1.2` package over the previous version.
- If Windows reports the old version is in use or the upgrade fails, uninstall the previous version first and then install `0.1.2`.

## Recommended Verification
- Import a COCO dataset whose `bbox` contains numeric strings and confirm annotations still appear correctly.
- Confirm normal COCO datasets with numeric `bbox` values still import as expected.
- Open the new user guide and confirm the markdown renders correctly.

## Verification
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --tests`

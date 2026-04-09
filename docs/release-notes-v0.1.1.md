# DataViewer v0.1.1 Release Notes

## Summary
This release focuses on smoother workspace onboarding, clearer source identification, and a cleaner Windows app-launch experience.

## Highlights
- New workspaces now open directly to `Sources`, so users can start adding datasets immediately after creation.
- Opening an existing workspace still goes to `Browser`, preserving the current review flow for returning users.
- `Sources` now shows both `Source Name` and full `Source Path`, making it easier to distinguish folders that share the same name.
- Long source paths are truncated with ellipsis in the table and reveal the full path on hover.
- `Import Review` now includes the source path for each category row.
- `Import Review` count display now shows `matched image count / total images in that source`, giving better context during category review.
- Fixed the Windows packaged app opening with an extra `cmd` window beside the main app window.

## Behavior Changes
- `Create Workspace` now navigates to `Sources` instead of `Browser`.
- `Open Existing Workspace` continues to navigate to `Browser`.

## Upgrade Notes
- Existing workspace data does not need to be recreated for this update.
- Users can install the new `0.1.1` package over the previous version.
- If Windows reports the old version is in use or the upgrade fails, uninstall the previous version first and then install `0.1.1`.

## Recommended Verification
- Create a new workspace and confirm it opens on `Sources`.
- Open an existing workspace and confirm it still opens on `Browser`.
- Check that `Sources` shows truncated source paths with full-path hover text.
- Check that `Import Review` shows source path and `current/total` count format.
- Launch the packaged Windows app and confirm no extra console window appears.

## Verification
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`

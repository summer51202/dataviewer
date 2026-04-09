# Windows Release SOP

## Purpose
This document describes the standard release flow for packaging `DataViewer` for Windows teammates.

## Release Files
- App name: `DataViewer`
- Tauri identifier: `com.edwardlee.dataviewer`
- Frontend package version: [package.json](/C:/EdwardLee/Project/DataViewer/package.json)
- App bundle version: [tauri.conf.json](/C:/EdwardLee/Project/DataViewer/src-tauri/tauri.conf.json)

## Pre-Release Checklist
Before creating a release build:

1. Make sure the working tree only contains intended changes.
2. Confirm the app can build successfully:
   ```powershell
   npm run build
   cargo check --manifest-path src-tauri/Cargo.toml
   ```
3. If this is a new release, update the version in both files:
   - [package.json](/C:/EdwardLee/Project/DataViewer/package.json)
   - [tauri.conf.json](/C:/EdwardLee/Project/DataViewer/src-tauri/tauri.conf.json)
4. Prepare short release notes for teammates:
   - What changed
   - Known limitations
   - Whether reinstall is needed

## Build Command
Run this in the repo root:

```powershell
npm run tauri build
```

This command will:
- build the frontend through Tauri's `beforeBuildCommand`
- build the Rust app
- generate Windows bundle artifacts

## Output Location
Release artifacts are typically generated under:

- [bundle](/C:/EdwardLee/Project/DataViewer/src-tauri/target/release/bundle)

Common Windows outputs may include:
- `.msi`
- setup `.exe`

## Recommended Distribution Flow
For internal teammate usage, prefer this order:

1. Share the installer package from the release bundle folder.
2. Include the version number in the file name or release message.
3. Include a short install note:
   - uninstall old version first if needed
   - keep workspace data outside the app install directory
   - run the app once and verify folder access permissions

## Smoke Test Before Sharing
Before sending the installer to teammates, verify at least once on a clean Windows machine or a machine without the dev environment.

Recommended smoke test flow:

1. Install the packaged app.
2. Launch `DataViewer`.
3. Create or open a workspace.
4. Add a source folder and confirm scan completes.
5. Open Browser and confirm thumbnails load.
6. Open one image detail page and confirm metadata loads.
7. Try `Export Selected`.
8. If your team uses CVAT, verify the CVAT page still opens and task flow is reachable.

## Teammate Installation Notes
Tell teammates:

1. Install via the provided `.msi` or setup `.exe`.
2. Do not place datasets inside the app install folder.
3. If Windows SmartScreen appears, use your internal trust process before continuing.
4. If they are upgrading from an older build, confirm whether they should uninstall first.

## Known Packaging Notes
- The project currently uses Tauri bundle target `all`, configured in [tauri.conf.json](/C:/EdwardLee/Project/DataViewer/src-tauri/tauri.conf.json).
- `rusqlite` is built with the `bundled` feature, which helps reduce external SQLite dependency issues on teammate machines.
- Build output size and installer format may vary by local packaging toolchain.

## Suggested Release Message Template
Use this when sending a build to teammates:

```text
DataViewer vX.Y.Z is ready.

Highlights:
- <feature 1>
- <feature 2>
- <bug fix 1>

Installer:
- <paste installer path or shared link>

Notes:
- Please test add source, browser, image detail, and export.
- Report any install or permission issues.
```

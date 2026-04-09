# DataViewer Product Highlights

## Product Positioning
`DataViewer` is a desktop data inspection and dataset workflow tool for image annotation projects. It is designed to help teams organize source datasets, review labels, filter images by annotation characteristics, prepare export sets, and connect selected images to CVAT workflows.

## Core Strengths
- Desktop-first workflow built with Tauri, suitable for local dataset operations without requiring a browser-hosted backend for core usage.
- Supports common dataset intake paths including `COCO`, `YOLO`, and `RAW images`.
- Centralizes source ingestion, category review, browsing, image inspection, export, and CVAT handoff in one tool.

## Key Features
### 1. Source Ingestion
- Add dataset sources from different formats.
- Scan local folders and extract image/category information.
- Provide format-specific guidance so users choose the correct dataset root.

### 2. Import Review
- Review incoming source categories before merging them into unified workspace-level categories.
- Save mapping decisions for merge, create-new, or ignore actions.
- Reduce category duplication and inconsistent naming across datasets.

### 3. Browser-Centered Dataset Exploration
- Browse large image sets inside a unified workspace.
- Filter by source, category, filename, annotation status, annotation count, and box-size ratio.
- Select individual images or bulk-select the current filtered result.

### 4. Rich Annotation Visibility
- Thumbnail cards show annotation count, largest box ratio, and per-box category summaries.
- Image Detail view visualizes bounding boxes directly on the image.
- Metadata panel exposes source path, categories, total box count, and per-box ratio details.

### 5. Export Workflow
- Export is driven from Browser selection rather than a disconnected standalone workflow.
- Teams can first filter and inspect images, then export only the selected set.
- Export preview and conflict handling help reduce packaging mistakes.

### 6. CVAT Handoff
- Selected images can be sent to CVAT from the Browser workflow.
- Supports task-oriented annotation collaboration after internal filtering and selection.

## User Experience Highlights
- Clearer selection state and bulk selection controls.
- Better feedback when long-running scans may be stalled.
- Reduced UI lag on image-heavy pages.
- More explicit guidance in source setup and review steps.

## Best-Fit Use Cases
- Internal dataset curation and cleanup before training.
- Reviewing and merging labels from multiple source datasets.
- Finding images with specific annotation density or object-size characteristics.
- Preparing a filtered subset for export or CVAT annotation work.

## Current Value Proposition
`DataViewer` helps teams move from scattered local dataset folders to a more controlled, inspectable, and repeatable dataset workflow. It is especially useful when teams need to review incoming labels, inspect annotation quality, and export targeted subsets instead of repeatedly handling full datasets manually.

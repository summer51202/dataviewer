# DataViewer MVP Product Spec

## 1. Product Overview

DataViewer is a single-user desktop tool for organizing local computer vision datasets for object detection.

The first version targets:

- Local multi-folder dataset management
- Visualization of images and bounding box annotations
- Manual category alignment across datasets
- CVAT integration for labeling unlabeled images
- Re-splitting and exporting datasets for local RF-DETR training

This product is intended for a single user working on Windows with local folders, without modifying original source data.

## 2. Product Goals

- Support importing multiple local dataset folders into one workspace
- Support COCO detection, YOLO detection, and unlabeled image folders
- Provide a clear visual workflow for browsing, filtering, and preparing data
- Allow users to manually align categories across different dataset sources
- Send selected unlabeled images to CVAT for annotation
- Sync annotations back as a new annotation version
- Export a complete standalone dataset in COCO or YOLO format for RF-DETR training

## 3. Non-Goals for MVP

- Segmentation, keypoints, pose, or video annotation
- In-app bbox editing
- Duplicate image detection
- Class-balanced split
- Multi-user collaboration
- Permission management
- Soft exclude / ignore flag on images
- Delete entire workspace inside the app

## 4. Target Users

- Single user
- Dataset engineer, ML engineer, or researcher
- Works mainly with local folders and local model training
- Needs a lightweight but structured dataset preparation tool

## 5. Platform and Technical Direction

- App type: desktop web app
- Desktop shell: Tauri
- Frontend: React
- Local database: SQLite
- Annotation tool: local CVAT deployment
- Source data mode: read-only

## 6. Core Concepts

### 6.1 Workspace

A workspace is the main unit of work.

Each workspace:

- Is created in a user-specified local folder
- Stores app-owned metadata, cache, SQLite data, CVAT temp data, export history
- Can include multiple source folders

### 6.2 Source Folder

A source folder is one imported local dataset folder.

Each source folder can be:

- COCO detection dataset
- YOLO detection dataset
- RAW unlabeled image folder

### 6.3 Unified Category

Categories from different sources are mapped into a workspace-level unified category list.

### 6.4 Annotation Version

Each sync-back from CVAT creates a new annotation version instead of overwriting current data.

### 6.5 Export Job

An export job records one export action, including:

- selected categories
- split settings
- random seed
- output format
- output path
- file naming conflict decisions

## 7. Source Data Rules

- Original source folders are always read-only
- The app does not modify original images or original annotation files
- The app stores only indexes, metadata, mappings, versions, and export records
- Images are read directly from original paths during browsing
- Files are copied only in these cases:
  - when sending selected images to CVAT temp workspace
  - when exporting a standalone dataset

## 8. Supported Input Formats

### 8.1 COCO Detection

Supported:

- COCO object detection annotations
- image files referenced by COCO annotations

Not supported in MVP:

- segmentation
- keypoints
- panoptic

### 8.2 YOLO Detection

Supported:

- standard object detection structure
- `images/`
- `labels/`
- `data.yaml`
- optional fallback to `classes.txt`
- bbox `.txt` annotations

Not supported in MVP:

- segmentation variants
- pose variants

### 8.3 RAW Images

Supported:

- regular unlabeled images such as `jpg`, `jpeg`, `png`
- recursive scanning of all subfolders

Not supported in MVP:

- camera raw formats such as `dng`, `cr2`, `nef`, `arw`

## 9. Main User Flow

1. User creates a workspace and chooses a local workspace folder
2. User adds one or more source folders
3. System detects source type for each folder
4. System indexes files and parses categories
5. User reviews and confirms category mapping during import review
6. User enters workspace browser and views images in thumbnail wall mode
7. User filters by source folder, category, annotation status, or filename
8. User selects part of the current filtered result, or selects all filtered results
9. User sends selected images to CVAT
10. User completes bbox annotation in CVAT
11. User syncs results back to DataViewer
12. System creates a new annotation version
13. User sets export categories, split ratios, random seed, and output format
14. User reviews export summary
15. User resolves filename conflicts if any
16. User exports a complete standalone dataset

## 10. Functional Requirements

### 10.1 Workspace Management

The system shall:

- support multiple workspaces
- allow user to create a workspace in a user-selected local folder
- allow user to open an existing workspace
- allow user to remove a source folder from a workspace without deleting original files

The system shall not:

- delete the entire workspace in MVP

### 10.2 Source Import

The system shall:

- allow importing multiple source folders into one workspace
- detect whether a source folder is COCO, YOLO, or RAW images
- recursively scan RAW image folders
- preserve original full path and relative path for each image

The system shall:

- perform a quick health check when opening a workspace
- not automatically perform a full rescan on open
- provide a manual `Rescan` action for full re-indexing

### 10.3 Category Mapping

The system shall:

- list all source categories and counts during import review
- compare them against existing workspace categories
- require user confirmation even when category names are identical
- allow each source category to be:
  - merged into an existing category
  - created as a new category
  - ignored from import
- save category mapping for later reuse and adjustment

### 10.4 Dataset Browser

The system shall provide:

- thumbnail wall as the default main view
- clear visual distinction between annotated and unannotated images using border color
- no bbox overlay on thumbnails in MVP

The system shall support filtering by:

- source folder
- category
- annotation status
- filename search

### 10.5 Single Image View

The system shall display:

- large image preview
- bbox overlays
- source folder
- original full file path
- category list

### 10.6 Selection and CVAT Send

The system shall:

- allow selecting individual images
- allow `Select All Current Filtered Results`
- send only selected images to CVAT
- copy selected images to a temporary workspace-owned folder before CVAT import

### 10.7 CVAT Integration

The system shall:

- integrate with local CVAT deployment
- create CVAT task/project for selected images
- track mapping between workspace, source folders, and CVAT task/project
- allow user to open the related CVAT task
- sync annotations back from CVAT
- create a new annotation version on each sync

The system shall not:

- provide in-app bbox editing
- overwrite old annotation versions

### 10.8 Split and Export

The system shall:

- ignore original dataset `train/valid/test` split from imported sources
- merge all imported data into one working pool
- re-split data into `train/valid/test`
- support configurable split ratios
- support fixed random seed for reproducible random split
- export in COCO or YOLO format
- export a complete standalone dataset with copied images and generated labels
- automatically exclude images with no bbox annotations from export

### 10.9 Export Review

Before export, the system shall show a summary including:

- final selected categories
- image counts by category
- bbox counts by category
- `train/valid/test` counts
- count of images excluded due to missing annotations
- count of filename conflicts that require confirmation

### 10.10 Filename Conflict Handling

If output filename conflicts occur, the system shall require manual confirmation.

The user shall be able to choose:

- auto add unique suffix
- manually rename
- skip the image

The system should also support:

- apply same decision to similar remaining conflicts

## 11. UI Modules

### 11.1 Workspace Home

- create workspace
- open workspace
- recent workspaces

### 11.2 Source Folder Management

- add source folder
- detect source type
- show import/index status
- remove source folder from workspace
- manual rescan

### 11.3 Import Review

- source category list
- existing workspace category list
- merge / create / ignore actions

### 11.4 Image Browser

- thumbnail wall
- filters
- search bar
- selection actions

### 11.5 Single Image View

- large preview
- bbox overlay
- source info
- file path
- category list

### 11.6 CVAT Task Center

- create send task
- open CVAT task
- sync back annotations
- view sync history

### 11.7 Annotation Version View

- list versions
- show version source
- show creation time

### 11.8 Export Center

- choose output format
- choose categories
- set split ratios
- set random seed
- choose output folder
- preview export summary
- resolve filename conflicts

## 12. Data Integrity and Risk Notes

- No duplicate detection in MVP, so data leakage between train and valid/test remains a known risk
- Same-name images from different sources may require manual export conflict handling
- Source files may be moved or deleted outside the app, so quick health checks must warn the user
- Category names may look identical but still represent different semantics, so import review must always require confirmation

## 13. Acceptance Criteria

The MVP is successful if the user can:

- create multiple workspaces
- import multiple local source folders into one workspace
- combine COCO, YOLO, and RAW image sources
- manually align categories across sources
- browse and filter images visually
- send selected filtered images to CVAT
- sync CVAT annotations back into a new annotation version
- re-split the combined dataset using random seed
- export a complete standalone COCO or YOLO dataset for RF-DETR training

## 14. Future Directions

Potential future versions may include:

- duplicate detection
- in-app bbox editing
- segmentation support
- annotation QA workflow
- class-aware or stratified split
- soft exclusion flags
- richer dataset analytics
- shared workspace / multi-user support

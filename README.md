# DataViewer

DataViewer 是一個以 `Tauri + React` 為方向的桌面化資料整理工具，目標是把多個本機 `COCO`、`YOLO`、未標記圖片資料夾整合到同一個 workspace，並串接 `CVAT` 與匯出 RF-DETR 可訓練資料集。

目前這個 repo 已經包含：

- 產品規格：[spec.md](./spec.md)
- 設計文件包：[docs/README.md](./docs/README.md)
- 第一版前端頁面骨架
- 第一版 Tauri / Rust command contract 骨架
- mock data 與 Tauri fallback，方便先看 UI 與資料流

## 目前專案結構

```text
.
|- docs/
|- src/
|- src-tauri/
|- spec.md
|- package.json
|- vite.config.ts
|- tsconfig.json
```

## 前端頁面骨架

- Workspace Home
- Sources
- Import Review
- Browser
- Image Detail
- CVAT Tasks
- Annotation Versions
- Export

## Tauri / Rust 骨架

目前 Rust 端先提供 mock-backed commands：

- `list_recent_workspaces`
- `get_workspace_overview`
- `get_source_folders`
- `get_import_review`
- `get_browser_payload`
- `get_cvat_tasks`
- `get_annotation_versions`
- `get_export_preview`

這樣前端可以先跟穩定的 command contract 對接，之後再逐步換成真實實作。

## 本機工具需求

這台目前工作環境沒有在 PATH 中找到：

- `node`
- `npm`
- `cargo`

所以這次先完成可讀、可延伸的專案骨架，尚未實際安裝依賴或執行 build。

## 建議下一步

1. 安裝 Node.js LTS
2. 安裝 Rust stable toolchain
3. 安裝 Tauri 對應的系統前置需求
4. 在 repo 根目錄執行依賴安裝
5. 啟動前端與 Tauri dev

## RF-DETR COCO 檢查

如果你要把匯出的 COCO 資料集直接丟給 `rf-detr-advanced-aug-main` 訓練，可以先跑：

```bash
npm run validate:rfdetr-coco -- "<export-dataset-root>"
```

例如：

```bash
npm run validate:rfdetr-coco -- "C:\\EdwardLee\\Project\\DataViewer\\workspaces\\factory-defect-v1\\.dataviewer\\exports\\ppe-coco"
```

這個檢查會驗證：

- `train/valid/test` 目錄是否存在
- 每個 split 是否有 `_annotations.coco.json`
- `images / annotations / categories` 三個 COCO 關鍵欄位
- annotation 是否引用到有效 image/category
- json 內引用的圖片是否真的存在於 split 目錄中

## YOLO 匯出檢查

如果你要驗證 DataViewer 匯出的 YOLO detection 資料集，可以跑：

```bash
npm run validate:yolo-export -- "<export-dataset-root>"
```

例如：

```bash
npm run validate:yolo-export -- "C:\\EdwardLee\\Project\\DataViewer\\workspaces\\factory-defect-v1\\.dataviewer\\exports\\ppe-yolo"
```

這個檢查會驗證：

- `train/valid/test` 的 `images` 與 `labels` 目錄是否存在
- 每張圖片是否有對應 `.txt`
- label 格式是否為 `class cx cy w h`
- bbox 是否維持在 `0~1` 的 normalized 範圍

## 預期 workspace 資料夾用途

實作完成後，workspace 預計存放：

- SQLite
- cache
- CVAT temp data
- export records
- future annotation version metadata

原始來源資料夾仍維持 `read-only`。

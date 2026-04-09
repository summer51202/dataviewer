# DataViewer UI Mockup

## 1. UI 設計目標

第一版 UI 以「快速整理資料」為核心，不做過度複雜的專案管理介面。

設計原則：

- 以影像瀏覽效率優先
- 讓多資料夾來源一眼可見
- 把匯入審查、送標、同步、匯出分成清楚步驟
- 讓已標註與未標註圖片在縮圖牆中立即可辨識
- 讓使用者一直知道目前在操作哪個 workspace、哪個版本、哪批資料

## 2. 視覺方向

- 產品類型：桌面資料工具，不走行銷頁風格
- 背景：暖灰白底搭配淡色分層面板
- 主色：深藍灰做結構色
- 狀態色：
  - 已標註：綠色邊框
  - 未標註：琥珀色邊框
  - 警告或缺失：紅色
- 字體：以清晰、工程工具導向為主
- 元件語氣：乾淨、密度偏高、資訊優先

## 3. 全域資訊架構

```text
Workspace Home
|- Create Workspace
|- Open Workspace
|- Recent Workspaces

Workspace Shell
|- Browser
|- Sources
|- Import Review
|- CVAT Tasks
|- Versions
|- Export
```

## 4. Workspace Home

用途：

- 建立新的 workspace
- 開啟既有 workspace
- 顯示最近使用清單

文字版 mockup：

```text
+----------------------------------------------------------------------------------+
| DataViewer                                                                       |
| Organize local COCO / YOLO / RAW image datasets for RF-DETR                      |
+----------------------------------------------------------------------------------+
| [ Create Workspace ]         [ Open Existing Workspace ]                         |
|                                                                                  |
| Recent Workspaces                                                                |
| +------------------------------------------------------------------------------+ |
| | factory-defect-v1        C:\Datasets\DV\factory-defect-v1     Open   Remove | |
| | retail-products-v2       D:\Projects\retail-v2                Open   Remove | |
| +------------------------------------------------------------------------------+ |
+----------------------------------------------------------------------------------+
```

## 5. Create Workspace

用途：

- 指定 workspace 名稱
- 指定 workspace 本機資料夾

文字版 mockup：

```text
+---------------------------------------------------------------------+
| Create Workspace                                                    |
+---------------------------------------------------------------------+
| Workspace Name                                                      |
| [ factory-defect-v1                                             ]   |
|                                                                     |
| Workspace Folder                                                    |
| [ C:\EdwardLee\Project\DataViewer\workspaces\factory-defect-v1 ]    |
| [ Browse... ]                                                       |
|                                                                     |
| This folder will store SQLite, cache, temp CVAT data, and exports.  |
| Original source folders stay read-only.                             |
|                                                                     |
|                                   [ Cancel ] [ Create Workspace ]   |
+---------------------------------------------------------------------+
```

## 6. Workspace Main Shell

用途：

- 提供主要導覽與固定操作區

版面結構：

- 左側：功能導覽與來源、類別摘要
- 上方：workspace 名稱、健康狀態、快速操作
- 中央：當前頁面內容

文字版 mockup：

```text
+--------------------------------------------------------------------------------------------------+
| DataViewer | Workspace: factory-defect-v1                           Health: Warning   [Rescan]   |
+----------------------+-------------------------------------------------------------------------+
| Browser              | Current Page Content                                                   |
| Sources              |                                                                         |
| Import Review        |                                                                         |
| CVAT Tasks           |                                                                         |
| Versions             |                                                                         |
| Export               |                                                                         |
|----------------------|-------------------------------------------------------------------------|
| Sources Summary      |                                                                         |
| - coco_old           |                                                                         |
| - yolo_batch_2       |                                                                         |
| - raw_2026_03        |                                                                         |
|----------------------|-------------------------------------------------------------------------|
| Categories Summary   |                                                                         |
| - screw (1240)       |                                                                         |
| - nut (830)          |                                                                         |
| - washer (412)       |                                                                         |
+----------------------+-------------------------------------------------------------------------+
```

## 7. Sources Page

用途：

- 加入來源資料夾
- 顯示來源格式
- 顯示索引狀態
- 手動重新掃描
- 從 workspace 移除來源

文字版 mockup：

```text
+--------------------------------------------------------------------------------------------------+
| Sources                                                                 [ Add Source Folder ]    |
+--------------------------------------------------------------------------------------------------+
| Source Name     | Type   | Status     | Images | Categories | Last Scan         | Actions        |
|-----------------------------------------------------------------------------------------------   |
| coco_old        | COCO   | Ready      | 1450   | 6          | 2026-03-31 10:30  | Rescan Remove  |
| yolo_batch_2    | YOLO   | Ready      | 980    | 4          | 2026-03-31 10:35  | Rescan Remove  |
| raw_2026_03     | RAW    | Warning    | 620    | 0          | 2026-03-31 10:40  | Rescan Remove  |
|                                                                                                  |
| Source Health Notes                                                                               |
| - raw_2026_03: 3 files are missing since last scan                                               |
+--------------------------------------------------------------------------------------------------+
```

## 8. Import Review Page

用途：

- 檢查新來源解析出的類別
- 與 workspace 既有類別對齊
- 手動確認同名或相近名稱的合併方式

文字版 mockup：

```text
+----------------------------------------------------------------------------------------------------------------+
| Import Review: yolo_batch_2                                                                                   |
+----------------------------------------------------------------------------------------------------------------+
| Source Category   | Count | Suggested Action     | Target Unified Category   | Final Action                  |
|---------------------------------------------------------------------------------------------------------------|
| car               | 540   | same-name candidate  | car                       | [ Merge v ]                  |
| cars              | 112   | similar-name         | car                       | [ Merge v ]                  |
| vehicle_car       | 98    | similar-name         | car                       | [ Merge v ]                  |
| background_obj    | 31    | no match             | -                         | [ Ignore v ]                 |
| pallet            | 207   | new                  | pallet                    | [ Create New v ]             |
|                                                                                                                |
| Existing Unified Categories                                                                                   |
| car, truck, person, pallet                                                                                    |
|                                                                                                                |
|                                                          [ Cancel ] [ Save Mapping and Import ]               |
+----------------------------------------------------------------------------------------------------------------+
```

互動重點：

- 同名類別也必須顯示在審查表內
- 來源類別數量要顯示，避免誤合併冷門類別
- 需要支援批次操作，但仍保留逐列調整能力

## 9. Browser Page

用途：

- 以縮圖牆為主的資料瀏覽
- 進行篩選、搜尋、勾選與送標

版面結構：

- 左欄：篩選條件
- 上欄：搜尋列與批次操作
- 中央：縮圖牆

文字版 mockup：

```text
+---------------------------------------------------------------------------------------------------------------+
| Browser                                                                                                       |
+---------------------------+-----------------------------------------------------------------------------------+
| Filters                   | [ Search filename...                     ]   [Select All Filtered] [Send to CVAT] |
|---------------------------|-----------------------------------------------------------------------------------|
| Source Folder             | Results: 284 images | Selected: 120                                                |
| [x] coco_old              |-----------------------------------------------------------------------------------|
| [x] yolo_batch_2          | [ amber border ] [ green border ] [ amber border ] [ green border ]               |
| [ ] raw_2026_03           | file_001.jpg     file_002.jpg     file_003.jpg     file_004.jpg                   |
|                           | screw, nut       screw            -               pallet                          |
| Category                  |                                                                                   |
| [x] screw                 | [ green border ] [ green border ] [ amber border ] [ amber border ]               |
| [x] nut                   | file_005.jpg     file_006.jpg     file_007.jpg     file_008.jpg                   |
| [ ] pallet                |                                                                                   |
|                           |                                                                                   |
| Annotation Status         |                                                                                   |
| ( ) All                   |                                                                                   |
| (x) Annotated             |                                                                                   |
| ( ) Unannotated           |                                                                                   |
|                                                                                                               |
| Legend                    |                                                                                   |
| Green = Annotated         |                                                                                   |
| Amber = Unannotated       |                                                                                   |
+---------------------------+-----------------------------------------------------------------------------------+
```

互動重點：

- `Select All Filtered` 只作用於目前篩選結果
- 單張卡片需要 checkbox 或明確選取狀態
- 未標註圖片應能快速篩出，方便送到 CVAT

## 10. Single Image View

用途：

- 檢查單張圖片與 bbox 狀態
- 顯示來源路徑與類別資訊

文字版 mockup：

```text
+---------------------------------------------------------------------------------------------------------------+
| file_002.jpg                                                               [ Prev ] [ Next ] [ Back ]        |
+--------------------------------------------------------------+------------------------------------------------+
|                                                              | Metadata                                       |
|                      Large Image Preview                     |------------------------------------------------|
|                                                              | Source Folder                                  |
|               [ bbox overlay rendered here ]                | D:\datasets\yolo_batch_2                       |
|                                                              |                                                |
|                                                              | Original Full Path                             |
|                                                              | D:\datasets\yolo_batch_2\images\file_002.jpg   |
|                                                              |                                                |
|                                                              | Categories                                     |
|                                                              | - screw                                        |
|                                                              | - nut                                          |
+--------------------------------------------------------------+------------------------------------------------+
```

## 11. CVAT Task Center

用途：

- 從已選圖片建立 CVAT 任務
- 開啟任務頁
- 同步標註回來
- 查看同步歷史

文字版 mockup：

```text
+----------------------------------------------------------------------------------------------------------------+
| CVAT Tasks                                                                                                     |
+----------------------------------------------------------------------------------------------------------------+
| Task Name              | Image Count | Status        | CVAT Project | Last Sync         | Actions              |
|----------------------------------------------------------------------------------------------------------------|
| raw_2026_03_batch_01   | 120         | In Progress   | defect_v1    | -                 | Open CVAT            |
| raw_2026_03_batch_02   | 80          | Ready Sync    | defect_v1    | 2026-03-31 11:10  | Open Sync Back       |
|                                                                                                                |
| Create New CVAT Task                                                                                            |
| - Current selection: 56 images                                                                                 |
| - Target labels: screw, nut, washer                                                                            |
| - Temp folder: workspace\temp\cvat\task_003                                                                    |
|                                                                                                                |
|                                                                     [ Create CVAT Task from Selection ]       |
+----------------------------------------------------------------------------------------------------------------+
```

## 12. Annotation Versions Page

用途：

- 顯示每次同步回來的標註版本
- 讓使用者知道目前工作區累積的版本歷史

文字版 mockup：

```text
+---------------------------------------------------------------------------------------------------------------+
| Annotation Versions                                                                                           |
+---------------------------------------------------------------------------------------------------------------+
| Version        | Created At          | Source CVAT Task        | Images | Boxes | Notes                      |
|---------------------------------------------------------------------------------------------------------------|
| v3             | 2026-03-31 11:25    | raw_2026_03_batch_02    | 80     | 412   | sync from CVAT            |
| v2             | 2026-03-31 10:55    | raw_2026_03_batch_01    | 120    | 618   | sync from CVAT            |
| v1             | 2026-03-31 10:20    | initial imported labels | 2430   | 9184  | initial normalized import |
+---------------------------------------------------------------------------------------------------------------+
```

## 13. Export Center

用途：

- 指定輸出格式
- 指定類別
- 指定 split 與 random seed
- 顯示匯出前摘要
- 處理檔名衝突

文字版 mockup：

```text
+----------------------------------------------------------------------------------------------------------------+
| Export Dataset                                                                                                 |
+-------------------------------------------------------------+--------------------------------------------------+
| Export Settings                                             | Export Summary                                   |
|-------------------------------------------------------------|--------------------------------------------------|
| Output Format                                               | Categories Selected: 3                           |
| (x) COCO                                                    | Images Included: 1984                            |
| ( ) YOLO                                                    | Images Excluded No BBox: 231                     |
|                                                             | BBoxes Included: 8451                            |
| Categories                                                  | train: 1388                                      |
| [x] screw                                                   | valid: 298                                       |
| [x] nut                                                     | test: 298                                        |
| [x] washer                                                  | Filename Conflicts: 4                            |
|                                                             |                                                  |
| Split Ratios                                                | Conflict Preview                                 |
| Train [ 70 ] Valid [ 15 ] Test [ 15 ]                      | file_0001.jpg from coco_old                      |
|                                                             | file_0001.jpg from yolo_batch_2                  |
| Random Seed [ 42 ]                                          | [ Resolve Conflicts ]                            |
|                                                             |                                                  |
| Output Folder                                               |                                                  |
| [ D:\exports\factory-defect-v1-coco ] [ Browse... ]         |                                                  |
|                                                             |                                                  |
|                                     [ Cancel ] [ Start Export ]                                               |
+----------------------------------------------------------------------------------------------------------------+
```

## 14. Filename Conflict Dialog

用途：

- 在匯出前處理同名輸出衝突

文字版 mockup：

```text
+----------------------------------------------------------------------------------------------------------------+
| Resolve Filename Conflict                                                                                      |
+----------------------------------------------------------------------------------------------------------------+
| Conflict: file_0001.jpg                                                                                        |
|                                                                                                                |
| Item A                                                                                                         |
| - Source Folder: D:\datasets\coco_old                                                                         |
| - Original Path: D:\datasets\coco_old\images\file_0001.jpg                                                    |
|                                                                                                                |
| Item B                                                                                                         |
| - Source Folder: D:\datasets\yolo_batch_2                                                                      |
| - Original Path: D:\datasets\yolo_batch_2\images\file_0001.jpg                                                |
|                                                                                                                |
| Decision                                                                                                       |
| (x) Auto add unique suffix                                                                                     |
| ( ) Manually rename                                                                                            |
| ( ) Skip this image                                                                                            |
|                                                                                                                |
| [ ] Apply this decision to remaining similar conflicts                                                         |
|                                                                                                                |
|                                                                     [ Previous ] [ Confirm ] [ Next ]         |
+----------------------------------------------------------------------------------------------------------------+
```

## 15. UI 狀態補充

需要有明確回饋的情況：

- 匯入中
- 解析失敗
- 健康檢查發現來源遺失
- CVAT 任務建立中
- CVAT 同步中
- 匯出進行中
- 匯出完成

建議 UI 行為：

- 長任務用 progress bar 或 job status 區塊
- 錯誤訊息要帶來源資料夾或任務名稱
- 所有 destructive action 都只對 workspace metadata 生效，不可暗示會刪原始檔

## 16. 第一版畫面優先順序

1. Workspace Home
2. Sources Page
3. Import Review Page
4. Browser Page
5. Single Image View
6. CVAT Task Center
7. Export Center
8. Annotation Versions Page

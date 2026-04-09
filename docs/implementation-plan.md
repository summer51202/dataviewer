# DataViewer 實作計畫

## 1. 計畫目標

這份計畫以單人開發、MVP 優先為前提。

重點不是一次把所有功能做滿，而是盡快打通這條主路徑：

1. 建立 workspace
2. 匯入多個來源
3. 類別對齊
4. 瀏覽與篩選
5. 送到 CVAT
6. 同步回來形成新版本
7. 匯出完整獨立資料集

## 2. 實作策略

建議採用「垂直切片」而不是先把所有前端頁面畫完、再補後端。

每個里程碑都應該：

- 有可操作畫面
- 有對應資料落地
- 有一條能走通的使用路徑

## 3. 目前進度總覽

| Milestone | 狀態 | 備註 |
| --- | --- | --- |
| M0 | 已完成 | Tauri + React + Rust command + SQLite 地基已可編譯 |
| M1 | 已完成 | Workspace/Sources 已可操作，建立、開啟、recent、source actions 已可用 |
| M2 | 已完成 | RAW / YOLO / COCO 已可真正匯入 images / categories / annotations |
| M3 | 進行中 | 已有自動同名合併與手動 mapping save，仍缺更完整 UX |
| M4 | 進行中 | Browser、單張檢視、真縮圖、bbox overlay 已落地，仍缺更多效能與細節優化 |
| M5 | 暫緩 | CVAT API 串接暫緩；目前僅完成本機 staging task |
| M6 | 已完成 | COCO / YOLO export、RF-DETR 相容 COCO layout、export validation script 已落地 |
| M7 | 進行中 | 第一輪穩定化開始：Rust tests、export validators、Export UX 細修 |

## 4. 已完成內容

### M0: 專案初始化與基礎框架

已完成：

- 建立 Tauri + React + TypeScript 專案骨架
- 建立 routing、workspace layout、首頁與主要頁面 shell
- 建立 Rust command contract 與前端 API 封裝
- 建立 workspace 資料夾結構
- 建立 SQLite 初始化流程
- 建立：
  - create workspace
  - open workspace
  - list recent workspaces
  - remove recent workspace
- 補齊 Tauri dialog plugin、capability、icon

完成定義：

- App 可 build
- Rust 可 `cargo check`
- 可建立並重新開啟 workspace

### M1: Workspace 與 Sources 骨架

已完成：

- 首頁 Create/Open Workspace 真實流程
- recent workspace 清單與移除
- Workspace 內可回首頁
- Sources Page 真實讀取 workspace source list
- Add Source Folder
- Rescan Source Folder
- Remove Source Folder
- source metadata 落地到 SQLite：
  - source_type
  - status
  - image_count
  - category_count
  - last_scan_at

尚未完成：

- source health check warning UI
- workspace 級 rescan
- source action 的更完整錯誤提示與防呆

完成定義目前達成程度：

- 使用者能在一個 workspace 中管理多個來源資料夾
- 已可初步看出來源是不是 RAW / YOLO / COCO

## 5. M2: COCO / YOLO / RAW 匯入與索引

### 目前已完成

- RAW images 寫入 `images`
- YOLO detection parser
  - `data.yaml` / `classes.txt`
  - `images` / `labels`
  - split-aware label path matching
- COCO detection parser
  - `images`
  - `categories`
  - `annotations`
- normalized internal model 已落地到 SQLite：
  - `images`
  - `categories`
  - `annotations`
  - `source_category_mappings`

### 尚未完成

- 匯入後的健康檢查摘要
- 匯入錯誤報告與部分失敗提示
- 更完整的 parser 測試

## 6. M3: 類別對齊與 Import Review

### 目前已完成

- Browser 左側類別已支援自動同名合併
- Import Review 已改成真資料
- 已支援手動調整：
  - `Merge`
  - `Create New`
  - `Ignore`
- 已支援 save mapping 並回寫 unified category / annotation category reference

### 尚未完成

- 更好的 mapping UX
  - 批次套用
  - 相似名稱建議
  - 衝突提示
  - 未儲存變更提醒

## 7. M4: Browser 與單張檢視

### 目前已完成

- Browser 真實 query API
- 真縮圖載入
- 單張圖真實圖片預覽
- bbox overlay
- 依來源、類別、標註狀態、檔名搜尋
- 選圖狀態
- 基本效能優化
  - lazy image loading
  - deferred search
  - incremental render

### 尚未完成

- 更進一步的虛擬化 / windowing
- 縮圖快取
- bbox 顏色與圖例
- 單張頁更多 metadata / 導航

## 8. M5: CVAT 任務建立與同步

### 目前已完成

- Browser 可把使用者帶到 `CVAT Tasks`
- 已可用目前 selection 建立本機 staging task
- 已建立：
  - `workspace/.dataviewer/temp/cvat/<task>`
  - staged images
  - `task.json`
  - `cvat_tasks` table records

### 目前決策

- `CVAT API` 串接暫緩
- `Open CVAT` / `Sync Back` 保留到後續里程碑

### 尚未完成

- CVAT API client
- create remote CVAT task
- open remote task
- sync back annotation version
- Versions Page 真資料

## 9. M6: Split、Export、衝突處理

### 目前已完成

- export preview summary 真資料
- 排除無 bbox 圖片
- random split with seed
- COCO exporter
- RF-DETR 相容 COCO layout
- YOLO exporter
- RF-DETR COCO validation script
- YOLO export validation script

### 尚未完成

- filename conflict detection + manual resolution
- export history / records page
- 更完整的 export 錯誤提示與進度

## 10. M7: 穩定化、測試、封裝

### 目前已完成

- Export UX 細修：
  - format 切換時自動建議輸出路徑
- Rust service tests（第一輪）
  - split counts
  - source type detection
  - YOLO split label path matching
  - YOLO export label generation

### 尚未完成

- 錯誤處理與 loading 狀態補齊
- frontend interaction tests
- Windows 路徑相容性檢查
- Tauri 打包
- 使用說明
- 更多 parser / export edge-case tests

## 11. 接下來最務實的順序

目前建議的實作順序改成：

1. 補第一輪穩定化剩餘項目
2. 補 annotation version 真資料與 history
3. 補檔名衝突人工確認流程
4. 視需要再回來做 CVAT API 串接

原因：

- 現在 workspace/source 管理已經能用
- 真正的產品價值下一步在於「平台開始理解資料集本身」
- 若 image/category/annotation schema 不先穩下來，後面的 Browser、Import Review、CVAT、Export 都會反覆重做

## 12. 目前距離可真正解決問題還差什麼

雖然目前已能完成匯入、整理、瀏覽與手動 mapping，但距離真正完成你的原始需求，還差這四條主幹：

1. RF-DETR 可用的 export pipeline
2. annotation version 與歷史追蹤真資料
3. 測試、打包與穩定化
4. 後續需要時再補 CVAT API task 建立與 sync-back

## 13. 風險點與對策

### 風險 1: 路徑處理複雜

情境：

- Windows 路徑
- 中文資料夾
- 空白路徑
- Docker / WSL2 / CVAT 路徑對應

對策：

- Rust 端統一路徑處理
- 優先採用 temp copy 再送 CVAT
- path 同時保存 display form 與 internal form

### 風險 2: 資料模型不穩

情境：

- COCO / YOLO 差異
- 類別 mapping 後 reference 混亂

對策：

- 先完成 normalized annotation model
- 先把 parser 與 schema 穩住
- 類別對齊只建立在穩定 import 結果上

### 風險 3: UI 做太快，資料流沒跟上

對策：

- 持續以垂直切片推進
- 每一步都以「可操作 + 有真資料落地」為完成標準

## 14. 測試重點

優先測核心資料邏輯：

- source type detection
- RAW recursive scan
- COCO parser
- YOLO parser
- category mapping apply
- annotation version creation
- random split reproducibility
- export filename conflict handling

## 15. MVP 完成後的第一輪優化

如果 MVP 跑通，下一輪最值得補的是：

1. duplicate detection
2. 更完整的 export validation
3. 圖片縮圖快取優化
4. richer dataset analytics
5. 更順的 CVAT task 批次管理

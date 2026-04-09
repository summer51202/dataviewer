# DataViewer 0.2.0 Export-time Data Balance Analysis Backlog Checklist

## 1. 使用方式

這份 checklist 是從 `Export-time Data Balance Analysis` 設計稿往下拆出的實作 backlog。

使用原則：

- 先完成 `Foundation`，再進入分析與 UI
- 以「同一份後端 planning logic 同時供 UI preview 與 export 執行使用」為核心
- 未列入 `MVP` 的項目不要在 `0.2.0` 中途擴張

狀態欄位建議：

- `[ ]` 未開始
- `[~]` 進行中
- `[x]` 完成
- `[-]` 延後

## 2. MVP 範圍總結

`0.2.0` MVP 目標：

- 在 `ExportPage` 針對 export scope 顯示資料分析
- 以 rule-based 方式輸出 top risks 與 recommendations
- 後端提供統一的 planning preview
- export 可輸出最基本 action files

不含：

- workspace-wide dashboard
- semantic diversity
- model-based hardness
- metadata-aware temporal leakage
- 全自動 subset optimization

## 3. Epic A: Foundation and Architecture

### A1. 明確定義 planning domain model

- [ ] 在 [src/types/workspace.ts](C:/EdwardLee/Project/DataViewer/src/types/workspace.ts) 新增 `ExportPlanningInput`
- [ ] 在 [src/types/workspace.ts](C:/EdwardLee/Project/DataViewer/src/types/workspace.ts) 新增 `ExportPlanningPreview`
- [ ] 在 [src/types/workspace.ts](C:/EdwardLee/Project/DataViewer/src/types/workspace.ts) 新增 `ExportPlanningIssue`
- [ ] 在 [src/types/workspace.ts](C:/EdwardLee/Project/DataViewer/src/types/workspace.ts) 新增 `ExportCategoryAnalysisRow`
- [ ] 在 [src-tauri/src/models.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/models.rs) 新增對應 Rust structs
- [ ] 確認欄位命名與既有 `ExportPreview` 命名風格一致

驗收條件：

- 前後端型別欄位對齊
- 不需要前端自行推算 analysis 結果

### A2. 建立新的 planning command

- [ ] 在 [src/lib/api.ts](C:/EdwardLee/Project/DataViewer/src/lib/api.ts) 新增 `getExportPlanningPreview()`
- [ ] 在 [src-tauri/src/commands/export.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/commands/export.rs) 新增 `get_export_planning_preview`
- [ ] 在 [src-tauri/src/commands/mod.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/commands/mod.rs) 註冊新 command
- [ ] 在 [src-tauri/src/lib.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/lib.rs) 或對應入口註冊 invoke handler
- [ ] 在 [src/lib/mock-data.ts](C:/EdwardLee/Project/DataViewer/src/lib/mock-data.ts) 新增 mock planning payload

驗收條件：

- 前端可在 mock 與 tauri runtime 下都拿到 planning preview

### A3. 後端 service 拆分起手式

- [ ] 新增 [src-tauri/src/analysis_service.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/analysis_service.rs)
- [ ] 新增 [src-tauri/src/balance_service.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/balance_service.rs)
- [ ] 在 [src-tauri/src/main.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/main.rs) 或 [src-tauri/src/lib.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/lib.rs) 宣告 module
- [ ] 將 planning preview 主邏輯從 [src-tauri/src/workspace_service.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/workspace_service.rs) 抽到新 service
- [ ] 保持 `workspace_service.rs` 只負責 command use case orchestration

驗收條件：

- `workspace_service.rs` 不再承擔大段 analysis 細節

## 4. Epic B: Export Candidate Pool

### B1. 統一 scope 解析邏輯

- [ ] 盤點目前 `ExportPreviewInput` / `StartExportInput` 的 scope 規則
- [ ] 抽出共用的 candidate pool builder
- [ ] 支援 `imageIds` 優先於 `sourceIds` / `categoryIds` 的規則
- [ ] 對空 scope、無符合圖片、全部被排除的情況定義回應

驗收條件：

- 同一組 scope 在 preview 與 export 拿到相同的候選池

### B2. 建立 exportable candidate 過濾規則

- [ ] 排除 corrupted images
- [ ] 排除無法匯出的 annotation
- [ ] 排除沒有有效 bbox 的圖片
- [ ] 規則明確標記哪些是 hard exclusion，哪些只是 warning
- [ ] 在 preview 結果中回傳 excluded counts 與原因摘要

驗收條件：

- UI 能清楚知道「為何有些圖不會進 export analysis」

## 5. Epic C: Rule-based Analysis Engine

### C1. Class distribution

- [ ] 計算 per-class image count
- [ ] 計算 per-class instance count
- [ ] 計算 avg instances per image
- [ ] 計算 class imbalance ratio
- [ ] 標記極低樣本 class

驗收條件：

- 類別表能穩定顯示 image / instance 的統計

### C2. Source concentration

- [ ] 計算每個 class 的 top source
- [ ] 計算 top source share
- [ ] 計算 source concentration ratio
- [ ] rule-based 標記單一 source 過度主導的類別

驗收條件：

- 能指出「某 class 主要來自哪個 source，佔比多少」

### C3. Split consistency preview

- [ ] 依當前 split ratio 模擬 split counts
- [ ] 預估 per-class 在 train / valid / test 的最低可見樣本風險
- [ ] 標記 valid / test 幾乎沒有樣本的 class
- [ ] 確保 split preview 與現有 export split 計算方式一致

驗收條件：

- 使用者調整 split ratio 後，risk 與 summary 會同步更新

### C4. Co-occurrence analysis

- [ ] 計算 per-class independent occurrence rate
- [ ] 計算 dominant partner class
- [ ] 計算 dominant partner rate
- [ ] 產出 top co-occurring classes
- [ ] 規則標記 shortcut risk

驗收條件：

- 可明確指出「A 類別幾乎都和 B 類別一起出現」

### C5. Integrity and quality summary

- [ ] 統計 corrupted image count within scope
- [ ] 統計 suspicious bbox count
- [ ] 定義 tiny box 規則
- [ ] 定義 giant box 規則
- [ ] 定義 extreme aspect ratio 規則
- [ ] 將 anomaly summary 納入 risk issue list

驗收條件：

- 明確資料問題不會被總分或摘要掩蓋

### C6. Duplicate summary

- [ ] 必做：exact duplicate 檢查設計定案
- [ ] 建立 exact duplicate signature 規則
- [ ] 回傳 duplicate group count 與 affected sample count
- [ ] 將 exact duplicate 轉成 recommendation / issue
- [ ] 選做：評估 near duplicate 的時間成本與可行性

驗收條件：

- 至少能在 export scope 內列出 exact duplicate 風險

## 6. Epic D: Risk Prioritization and Recommendation

### D1. Risk issue model

- [ ] 定義 `P0` / `P1` / `P2` / `P3` 規則集
- [ ] 每個 issue 產出 `title`
- [ ] 每個 issue 產出 `reason`
- [ ] 每個 issue 產出 `recommendation`
- [ ] 每個 issue 可選擇附帶 affected categories

驗收條件：

- UI 不只顯示數字，還能解釋為什麼有風險

### D2. Training-time balancing suggestions

- [ ] 產出 class weights suggestion
- [ ] 產出 repeat factor suggestion
- [ ] 定義 minority class oversampling suggestion 文案
- [ ] 明確標註這些是 training-time 建議，不會改 workspace 原始資料

驗收條件：

- recommendation 可直接給 ML / training pipeline 使用

### D3. Candidate downsampling suggestions

- [ ] 先只做 suggestion，不自動刪除
- [ ] 針對 exact duplicates 輸出 candidate list
- [ ] 針對 majority class redundancy 輸出候選摘要
- [ ] 說明這是 balanced subset 候選，不是永久清理

驗收條件：

- 使用者能區分「建議排除」與「系統直接刪除」

### D4. Collection recommendations

- [ ] 產出 rare class 補資料建議
- [ ] 產出 small-object coverage 補資料建議
- [ ] 產出 independent occurrence 補資料建議
- [ ] 產出 source coverage 補資料建議

驗收條件：

- recommendation 不只指出問題，還能指出下一步補資料方向

## 7. Epic E: ExportPage UI Integration

### E1. 資訊架構重整

- [ ] 盤點 [ExportPage.tsx](C:/EdwardLee/Project/DataViewer/src/features/workspace/pages/ExportPage.tsx) 現有區塊順序
- [ ] 插入 `Dataset Analysis` 區塊
- [ ] 插入 `Recommendations` 區塊
- [ ] 保持 `Export Scope`、`Split / Output`、`Summary` 邏輯清晰
- [ ] 避免單頁過長無法掃讀

驗收條件：

- `ExportPage` 不會因資訊增加而變得難用

### E2. Summary cards

- [ ] 顯示 total exportable images
- [ ] 顯示 total instances
- [ ] 顯示 classes count
- [ ] 顯示 exact duplicates
- [ ] 顯示 top source concentration
- [ ] 顯示 top risk count

驗收條件：

- 使用者在頁面上方能快速掌握健康狀態

### E3. Category analysis table

- [ ] 顯示 category name
- [ ] 顯示 image count
- [ ] 顯示 instance count
- [ ] 顯示 independent occurrence rate
- [ ] 顯示 top source share
- [ ] 顯示 shortcut risk badge
- [ ] 顯示 split risk badge

驗收條件：

- 類別分析結果可掃描、可排序、可理解

### E4. Recommendations panel

- [ ] 顯示 top risks
- [ ] 顯示 why it matters
- [ ] 顯示 what to do next
- [ ] 區分 training-time / collection / downsampling 建議
- [ ] 高風險項目要有明顯視覺層級

驗收條件：

- 使用者能直接從 recommendations 採取下一步

### E5. Preview refresh behavior

- [ ] scope 改變時重新抓 planning preview
- [ ] split ratio 改變時重新抓 planning preview
- [ ] output format 改變時重新抓 planning preview
- [ ] 避免過度頻繁請求造成卡頓
- [ ] loading / empty / error state 完整

驗收條件：

- 畫面更新時機穩定且可預期

## 8. Epic F: Export Integration and Action Files

### F1. Export job metadata

- [ ] 定義 planning config 是否需要落庫
- [ ] 若需要，擴充 `export_jobs` schema 或相關 metadata 儲存方式
- [ ] 記錄本次 export 使用的 split / planning version / recommendation snapshot

驗收條件：

- 日後可以回看「這份 export 當時是依什麼分析與設定產生的」

### F2. Action file generation

- [ ] 產出 `class_weights.yaml`
- [ ] 產出 `suspicious_samples.csv`
- [ ] 產出 `collection_recommendations.md`
- [ ] 評估 `repeat_factor.yaml` 是否列為 MVP
- [ ] 評估 `exact_duplicates.csv` 是否列為 MVP

驗收條件：

- 至少有 2-3 份可交給 downstream 使用的 action files

### F3. Preview / export consistency

- [ ] export 執行時使用與 preview 相同的 candidate pool 規則
- [ ] export 執行時使用與 preview 相同的 split 規則
- [ ] 若 planning 與 export 使用條件不同，要回傳明確錯誤或重新 preview

驗收條件：

- 不再出現 preview 跟最終輸出邏輯分裂

## 9. Epic G: Mock Data, Demo State, and Documentation

### G1. Mock data

- [ ] 在 [src/lib/mock-data.ts](C:/EdwardLee/Project/DataViewer/src/lib/mock-data.ts) 補 planning preview 範例
- [ ] mock 至少包含一個長尾 class 範例
- [ ] mock 至少包含一個 source concentration 範例
- [ ] mock 至少包含一個 shortcut risk 範例
- [ ] mock 至少包含一個 duplicate risk 範例

驗收條件：

- 前端在沒有 tauri runtime 時也能演示新功能

### G2. Documentation

- [ ] 更新 [docs/functional-workflow.md](C:/EdwardLee/Project/DataViewer/docs/functional-workflow.md) 反映 export-time analysis 流程
- [ ] 視需要更新 [docs/module-architecture.md](C:/EdwardLee/Project/DataViewer/docs/module-architecture.md) 加上 analysis / balance service
- [ ] 在 [docs/README.md](C:/EdwardLee/Project/DataViewer/docs/README.md) 保持文件索引完整
- [ ] 補一份使用者導向操作說明或 release note 草稿

驗收條件：

- 文件不落後於實作

## 10. Epic H: Testing and QA

### H1. Rust unit tests

- [ ] class distribution 計算測試
- [ ] source concentration 計算測試
- [ ] split preview 小樣本測試
- [ ] co-occurrence 計算測試
- [ ] anomaly rules 測試
- [ ] exact duplicate rules 測試
- [ ] risk priority 規則測試

### H2. Frontend behavior tests / validation

- [ ] 確認 planning preview 進入頁面會載入
- [ ] 確認 scope 變更會 refresh
- [ ] 確認 split ratio 變更會 refresh
- [ ] 確認錯誤狀態與空資料狀態顯示正常
- [ ] 確認 mock mode 與 tauri mode 介面一致性

### H3. Manual QA checklist

- [ ] 空 scope
- [ ] 只有單一類別
- [ ] 多類別但高度共現
- [ ] 長尾 class distribution
- [ ] 單一 source 幾乎壟斷某類別
- [ ] 有 corrupted images
- [ ] 有 suspicious bbox
- [ ] 有 exact duplicates
- [ ] split ratio 改變後 analysis 與 summary 會同步更新
- [ ] export 後 action files 內容正確

## 11. 建議開發順序

### Sprint 1

- [ ] Epic A
- [ ] Epic B
- [ ] Epic C1
- [ ] Epic C2
- [ ] Epic C3

### Sprint 2

- [ ] Epic C4
- [ ] Epic C5
- [ ] Epic D1
- [ ] Epic E1
- [ ] Epic E2
- [ ] Epic E3

### Sprint 3

- [ ] Epic C6
- [ ] Epic D2
- [ ] Epic D3
- [ ] Epic D4
- [ ] Epic E4
- [ ] Epic E5

### Sprint 4

- [ ] Epic F1
- [ ] Epic F2
- [ ] Epic F3
- [ ] Epic G1
- [ ] Epic G2
- [ ] Epic H

## 12. 建議延後項目

- [-] near duplicate via pHash / dHash
- [-] metadata-aware temporal leakage
- [-] semantic diversity
- [-] model-based hardness
- [-] fully automatic subset optimization
- [-] Pascal VOC loader for analysis-first use case

## 13. Definition of Done

`0.2.0` 可視為完成的最低條件：

- [ ] `ExportPage` 能顯示 export-time analysis
- [ ] planning preview 由後端統一提供
- [ ] top risks 與 recommendations 可讀且可解釋
- [ ] export summary 與 planning preview 一致
- [ ] 至少輸出 2-3 份 actionable files
- [ ] 單元測試與手動 QA checklist 跑過一輪
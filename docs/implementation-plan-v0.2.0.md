# DataViewer 0.2.0 實作計畫

## 1. 版本目標

`DataViewer 0.2.0` 的核心目標，是在現有 `Export` 流程中加入 `Export-time Data Balance Analysis`，讓使用者在真正輸出訓練資料集之前，就能看見這批資料的健康狀態、失衡風險與建議修正方向。

這個版本不追求做出一個完整的全域資料分析平台，而是聚焦在：

- 使用者選定 export scope 後的分析
- rule-based 風險診斷
- recommendation 與 actionable outputs
- 與現有 `ExportPage` / `ExportPreview` / `StartExport` 流程深度整合

一句話版本：

> 在匯出前，幫使用者判斷「這批準備拿去訓練的資料是否健康，若不健康，應該先修什麼」。

## 2. 為什麼是 0.2.0 的主題

這個方向符合目前 DataViewer 的產品定位：

- DataViewer 本來就是資料整理、篩選、標註串接與匯出的中控台
- 使用者真正需要資料平衡判斷的時機，多半發生在 export 前
- 與其做全域 dashboard，不如先把 export 決策點做強

因此 `0.2.0` 不做「新的分析產品」，而是把 `Export` 進化成更有判斷力的 planning workflow。

## 3. 版本範圍

### 3.1 本版要完成的

- `ExportPage` 新增 dataset analysis 與 recommendations 區塊
- 後端新增統一的 planning preview API
- 分析母體改為 `export candidate pool`
- 顯示 class distribution、source concentration、split consistency、co-occurrence、anomaly summary、exact duplicate summary
- 顯示 top risks 與可解釋建議
- 匯出最基本的 actionable outputs
- 保證 preview 與 export 使用相同邏輯

### 3.2 本版不做的

- workspace-wide dashboard
- semantic diversity
- model-based hardness
- metadata-aware temporal leakage
- 全自動 subset optimization
- 完整近重複影像分析作為核心功能
- Pascal VOC 擴充成分析優先流程

## 4. 使用者價值

完成後，使用者在 Export 階段可以回答：

- 哪些 class 明顯失衡
- 哪些 class 過度集中於單一 source
- 哪些 class 在 valid / test 幾乎沒有樣本
- 哪些 class 可能有 shortcut risk
- 是否存在可疑標註或 exact duplicate
- 應優先補資料、調整 split，還是改 training-time strategy

## 5. 與現有架構的整合策略

### 5.1 前端

以 [ExportPage.tsx](C:/EdwardLee/Project/DataViewer/src/features/workspace/pages/ExportPage.tsx) 為主，不新增大型獨立分析頁。

建議區塊順序：

1. Export Scope
2. Dataset Analysis
3. Recommendations
4. Split and Output
5. Export Summary

### 5.2 API / Tauri Command

新增統一的 planning API，建議命名：

- `get_export_planning_preview`

由後端同時回傳：

- export summary
- analysis summary
- risk issues
- recommendation summary
- action file preview

### 5.3 Rust Core

這版開始把分析邏輯從 [workspace_service.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/workspace_service.rs) 分離。

建議新增：

- [analysis_service.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/analysis_service.rs)
- [balance_service.rs](C:/EdwardLee/Project/DataViewer/src-tauri/src/balance_service.rs)

### 5.4 DB / 資料層

沿用現有：

- `images`
- `annotations`
- `categories`
- `source_folders`
- `export_jobs`

若有需要，再針對 `export_jobs` 補 planning metadata。

## 6. Phase 規劃

### Phase 1. Foundation

目標：建立 planning preview 骨架與統一資料模型。

交付：

- 前後端 `ExportPlanning*` 型別
- `get_export_planning_preview`
- mock data
- export candidate pool builder
- preview / export 共用 scope 規則

成功條件：

- 可以從前端拿到一份完整 planning preview
- 同一組 scope 在 preview 與 export 規則一致

### Phase 2. Core Analysis

目標：完成 rule-based 的核心資料分析。

交付：

- class distribution
- source concentration
- split consistency preview
- co-occurrence analysis
- anomaly summary
- exact duplicate summary

成功條件：

- 能穩定產出各類別與主要風險統計
- 使用者調整 split ratio 後 analysis 會同步更新

### Phase 3. Recommendation Layer

目標：把分析結果轉成可執行建議。

交付：

- P0-P3 risk priority
- training-time balancing suggestions
- candidate downsampling suggestions
- collection recommendations

成功條件：

- UI 不只顯示數據，還能明確解釋「為什麼有風險、下一步怎麼做」

### Phase 4. UI Integration

目標：把 planning 體驗完整接進 ExportPage。

交付：

- summary cards
- category analysis table
- recommendations panel
- loading / empty / error state
- refresh behavior

成功條件：

- `ExportPage` 可以完整承接 planning workflow
- UI 不會因資訊變多而失去可讀性

### Phase 5. Export Integration and Outputs

目標：讓 planning 不只是看報告，而是能真正影響交付物。

交付：

- export job metadata
- `class_weights.yaml`
- `suspicious_samples.csv`
- `collection_recommendations.md`
- preview / export consistency guard

成功條件：

- 最終 export 產物與 planning preview 一致
- 使用者能把 action files 交給 downstream pipeline 使用

### Phase 6. QA and Documentation

目標：完成測試、文件與發版準備。

交付：

- Rust unit tests
- frontend integration validation
- manual QA checklist
- workflow / architecture docs 更新
- release notes 草稿

成功條件：

- 至少跑完一輪完整 QA
- 文件可支援內部使用與發版說明

## 7. 主要風險

### 7.1 Preview / Export 邏輯再次分裂

這是目前最需要避免的風險。

對策：

- preview 與 export 必須共用 candidate pool 與 split 規則
- 前端不自行推算核心分析結果

### 7.2 `workspace_service.rs` 再次膨脹

若所有分析邏輯都塞回單一檔案，`0.2.0` 之後會變得很難維護。

對策：

- 這版開始切出 analysis / balance service
- `workspace_service.rs` 只保留 orchestration

### 7.3 UI 資訊過重

`ExportPage` 本來就已有不少設定，如果直接硬塞大量表格與警示，會影響可用性。

對策：

- 使用 summary cards + table + recommendations 的分層設計
- 高風險項目優先顯示，細節放可展開內容

### 7.4 重複檢查成本過高

near duplicate 若做太早，可能拖慢整體交付。

對策：

- `0.2.0` 先把 exact duplicate 做穩
- near duplicate 保留為延伸項

### 7.5 metadata 分析承諾過頭

目前資料模型沒有正式的 `camera_id / batch_id / timestamp / location`。

對策：

- `0.2.0` 專注於 source-level 分析
- 不承諾深度 metadata leakage 能力

## 8. 驗收標準

`0.2.0` 完成至少要滿足：

- `ExportPage` 能顯示 export-time analysis
- planning preview 由後端統一提供
- top risks 與 recommendations 可讀且可解釋
- split ratio 改變後 analysis 與 summary 會同步更新
- export summary 與 planning preview 一致
- 至少輸出 2-3 份 actionable files
- 測試與手動 QA 跑過一輪

## 9. 建議時程切法

### Milestone A: Alpha

重點：

- planning API
- candidate pool
- class / source / split analysis

完成後可驗證：

- 核心資料流是否打通
- preview 能否穩定回傳

### Milestone B: Beta

重點：

- co-occurrence
- anomaly summary
- exact duplicate summary
- risk priority
- recommendations UI

完成後可驗證：

- 使用者是否已能靠分析結果做決策

### Milestone C: RC

重點：

- action files
- export integration
- consistency guard
- docs / QA / polish

完成後可驗證：

- 功能是否達到可發版品質

## 10. 建議搭配文件

建議把這三份一起看：

- [export-time-data-balance-analysis-v0.2.0.md](C:/EdwardLee/Project/DataViewer/docs/export-time-data-balance-analysis-v0.2.0.md)
- [export-time-data-balance-backlog-v0.2.0.md](C:/EdwardLee/Project/DataViewer/docs/export-time-data-balance-backlog-v0.2.0.md)
- [functional-workflow.md](C:/EdwardLee/Project/DataViewer/docs/functional-workflow.md)

## 11. 一句話結論

`0.2.0` 的實作重點，不是做一個全新的資料分析產品，而是把 `Export` 升級成「可分析、可解釋、可決策」的 planning 流程。
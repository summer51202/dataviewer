# DataViewer v0.1.3 版本更新說明

## 摘要
這個版本整理了一輪使用者回饋，重點放在 export 流程更順手、Browser 選取更有效率，以及首頁資訊更完整。

## 更新重點
- Export 預設 split ratio 改為 `train 80 / valid 15 / test 5`。
- Export 現在可直接瀏覽並選擇自訂輸出資料夾，不用只能手動輸入路徑。
- `Conflict Handling` 現在預設開啟 auto rename，同時保留檔名衝突警示提示。
- `Export Scope` 現在會顯示各類別圖片比例，母體為目前 scope 中可匯出的 annotated 圖片。
- `Export Summary` 的 split 數量現在會跟著目前 ratio 設定同步更新，不再固定使用舊比例。
- 首頁 `DataViewer` 標題旁現在會顯示版本號。
- Browser 現在支援 `Shift` 連選，以及目前已載入縮圖範圍的框選。

## 行為變更
- 新的 export 預設 split 為 `80 / 15 / 5`。
- Browser 框選只作用在目前畫面已載入的 thumbnails。
- `Export Scope` 的類別比例以可匯出的 annotated 圖片為分母，不是 scope 內全部圖片。

## 升級說明
- 這次更新不需要重新建立既有 workspace。
- 使用者可直接安裝 `0.1.3` 新版本覆蓋舊版。
- 如果 Windows 提示舊版本仍在使用中，或安裝升級失敗，請先移除舊版後再安裝 `0.1.3`。

## 建議驗證項目
- 開啟 `Export`，確認預設 split 為 `80 / 15 / 5`。
- 修改 split ratio，確認 `Export Summary` 的 split 數量會立即更新。
- 在 `Export` 使用 `Browse Folder`，確認可選擇自訂輸出資料夾。
- 確認有檔名衝突時仍會顯示 warning，且 auto rename 預設為開啟。
- 確認 `Export Scope` 會顯示各類別圖片比例。
- 確認首頁標題旁會顯示 `v0.1.3`。
- 在 Browser 測試一般點選、`Shift` 連選，以及已載入縮圖的框選。

## 驗證
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --tests`
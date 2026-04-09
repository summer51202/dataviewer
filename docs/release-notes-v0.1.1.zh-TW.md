# DataViewer v0.1.1 版本更新說明

## 摘要
這個版本主要聚焦在三件事：讓新 workspace 建立後更快開始整理資料、讓來源辨識更清楚，以及改善 Windows 安裝版啟動時的體驗。

## 更新重點
- 新建立的 workspace 現在會直接進入 `Sources` 頁面，方便使用者建立後立刻加入資料來源。
- 開啟既有 workspace 仍然維持進入 `Browser`，保留目前熟悉的操作流程。
- `Sources` 頁面現在除了 `Source Name` 之外，也會顯示完整 `Source Path`，更容易分辨同名資料夾。
- 過長的來源路徑會以 `...` 省略顯示，滑鼠移上去時可看到完整路徑。
- `Import Review` 現在會顯示每個 category 所屬的來源路徑。
- `Import Review` 的 `Count` 現在改為顯示 `目前類別圖片數 / 該來源總圖片數`，方便判斷這個 category 在來源中的占比。
- 修正 Windows 安裝版啟動時，主程式旁邊會多開一個 `cmd` 視窗的問題。

## 行為變更
- `Create Workspace` 建立完成後，導向頁面由 `Browser` 改為 `Sources`。
- `Open Existing Workspace` 仍然維持導向到 `Browser`。

## 升級說明
- 這次更新不需要重新建立既有 workspace。
- 使用者可直接安裝 `0.1.1` 新版本覆蓋舊版。
- 如果 Windows 提示舊版本仍在使用中，或安裝升級失敗，請先移除舊版後再安裝 `0.1.1`。

## 建議驗證項目
- 建立新的 workspace，確認會直接進入 `Sources`。
- 開啟既有 workspace，確認仍然進入 `Browser`。
- 確認 `Sources` 頁面能顯示來源路徑，且長路徑會省略顯示並支援 hover 看完整內容。
- 確認 `Import Review` 能顯示來源路徑，且 `Count` 為 `目前/總數` 格式。
- 啟動 Windows 安裝版，確認不再出現額外的 console / cmd 視窗。

## 驗證
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`

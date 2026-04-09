# DataViewer v0.1.2 版本更新說明

## 摘要
這個版本主要聚焦在兩件事：提升 COCO 匯入對非標準資料的容錯能力，以及補上更清楚的使用者導覽文件。

## 更新重點
- COCO 匯入現在可接受 `bbox` 內的 numeric string，例如 `"444.00"`，並在解析時自動轉成數值。
- 同樣的 numeric-string 容錯也套用到 COCO 形式的標註同步流程，降低上游匯出格式不一致造成的失敗。
- 新增一份給使用者看的功能導覽文件，整理產品目的、主要工作流程與各頁面用途。

## 行為變更
- 如果 COCO annotation 的 `bbox` 陣列同時混用 JSON number 與可解析的數字字串，現在會視為有效資料。
- 如果字串不是合法數字，仍然會被視為無效資料並略過。

## 升級說明
- 這次更新不需要重新建立既有 workspace。
- 使用者可直接安裝 `0.1.2` 新版本覆蓋舊版。
- 如果 Windows 提示舊版本仍在使用中，或安裝升級失敗，請先移除舊版後再安裝 `0.1.2`。

## 建議驗證項目
- 匯入一份 `bbox` 含有 numeric string 的 COCO 資料集，確認標註仍可正常出現。
- 確認一般標準 COCO 資料集的匯入行為沒有被影響。
- 開啟新的使用者功能導覽文件，確認 Markdown 顯示正常。

## 驗證
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --tests`

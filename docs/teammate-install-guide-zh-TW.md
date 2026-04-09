# DataViewer 安裝與使用說明

## 適用對象
本文件提供給使用 `DataViewer` 的同事，說明 Windows 安裝、首次啟動與基本注意事項。

## 安裝前準備
請先確認：

1. 你使用的是 Windows 電腦。
2. 已取得安裝檔，通常會是以下其中一種：
   - `.msi`
   - `.exe`
3. 你的資料集不要放在程式安裝目錄內。

## 安裝步驟

1. 雙擊安裝檔。
2. 依照畫面提示完成安裝。
3. 若 Windows 跳出 SmartScreen 或安全性提示，請依公司內部流程確認後再繼續。
4. 安裝完成後，從桌面捷徑或開始功能表開啟 `DataViewer`。

## 第一次啟動建議
第一次開啟後，建議先做以下確認：

1. 成功進入主畫面。
2. 可以建立新 workspace，或開啟既有 workspace。
3. 可以新增 source folder。
4. 掃描完成後，Browser 頁面可以看到圖片。
5. 點進單張圖片後，可以看到圖片與 metadata。

## Source Folder 使用提醒
新增資料來源時，請注意資料夾要選對層級：

- `COCO`
  請選擇 dataset root，也就是同時包含圖片與標註資訊的根目錄。
- `YOLO`
  請選擇正確的 dataset root 或 split folder。
  如果上層沒有 `data.yaml`，有些情況需要直接選到 `train` 那層。
- `RAW images`
  請選擇實際存放圖片的資料夾。

如果資料夾選得太深或太淺，可能會造成標註讀不到，或類別資訊不完整。

## 基本使用流程
建議使用順序如下：

1. 建立或開啟 workspace。
2. 新增 source folder。
3. 等待 scan 完成。
4. 到 Browser 頁面檢查圖片與標註。
5. 需要時可進行：
   - Import Review
   - Export Selected
   - Send to CVAT

## Browser 使用提醒
在 Browser 頁面中：

- 可以用 source、category、annotation status、annotation count、box ratio 篩選圖片。
- 點一次圖片會選取，再點一次會取消選取。
- 若要整批操作目前篩選結果，可先按 `Select All Filtered`。
- 若要匯出，請使用 `Export Selected`。

## 常見狀況

### 掃描很久沒有完成
- 大型資料夾可能需要較久時間。
- 若等待很久仍沒有進度，請截圖畫面並回報。

### 匯出時沒有圖片
- 請先確認是否已經選取圖片。
- 若只做了篩選但沒有選取，請先按 `Select All Filtered` 再匯出。

### 看不到標註或類別
- 請先確認 source folder 是否選到正確層級。
- 若是匯入後類別需要合併，請到 `Import Review` 確認 mapping。

## 回報問題時請附上
若遇到問題，請盡量一起提供：

1. 問題發生的頁面名稱。
2. 操作步驟。
3. 錯誤訊息截圖。
4. 使用的資料格式：`COCO`、`YOLO` 或 `RAW`。
5. 使用的資料夾路徑結構說明。

## 版本與安裝包
如果你不確定目前安裝的是哪一版，請向提供安裝檔的人確認版本號與更新內容。

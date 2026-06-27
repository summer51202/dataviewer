# QA Checklist — UMAP Projection

功能範圍：embedding job 完成後以 UMAP 取代 PCA 做 2D 投影，Python 環境不可用時自動 fallback PCA。

---

## 前置條件

```bash
# 安裝 Python 依賴（若尚未安裝）
pip install -r scripts/requirements-umap.txt

# 確認環境正常
python -c "import umap; print('ok')"
```

需要一個含有已跑完 embedding 的 workspace（`fast-preview` 以外的 model）。

---

## 1. Python Script 直接測試（CLI）

### 1-1 基本 happy path

```bash
python scripts/run_umap_projection.py \
  --db-path "<workspace>/.dataviewer/workspace.db" \
  --workspace-id "<id>" \
  --scope object \
  --model-id "<model-id>"
```

- [ ] stdout 輸出合法 JSON，例如 `{"projected": N, "method": "umap-v1"}`
- [ ] `projected` > 0
- [ ] exit code 為 0
- [ ] stderr 印出 `[umap] reading embeddings`、`[umap] running UMAP`、`[umap] writing N projections` 三行進度

### 1-2 投影結果寫入 DB

執行 1-1 後：

```sql
SELECT projection_method, COUNT(*), MIN(x), MAX(x), MIN(y), MAX(y)
FROM embedding_projections
WHERE workspace_id = '<id>' AND scope = 'object' AND model_id = '<model-id>'
GROUP BY projection_method;
```

- [ ] 存在 `projection_method = 'umap-v1'` 的 row
- [ ] `MIN(x) >= -1.0`、`MAX(x) <= 1.0`（容許浮點誤差，可放寬到 `±1.0001`）
- [ ] `MIN(y) >= -1.0`、`MAX(y) <= 1.0`（同上）

### 1-3 重複執行（upsert 行為）

連續執行 1-1 兩次。

- [ ] 第二次 exit code 仍為 0
- [ ] DB 裡 `umap-v1` row 數量不變（upsert，非新增）
- [ ] 座標被更新（`created_at` 變新）

### 1-4 Determinism（固定 seed）

對同一個 DB 執行兩次，比較座標：

```sql
SELECT target_id, x, y FROM embedding_projections
WHERE workspace_id = '<id>' AND scope = 'object'
  AND model_id = '<model-id>' AND projection_method = 'umap-v1'
ORDER BY target_id;
```

- [ ] 兩次結果完全相同

### 1-5 Image scope

將 `--scope` 改為 `image` 重跑。

- [ ] stdout `projected` > 0
- [ ] DB 中 `scope = 'image'` 的 `umap-v1` row 存在

### 1-6 點數少於 n_neighbors（< 16）

準備或找一個 embedding 數量在 2–15 之間的 workspace（或直接建測試 DB）。

- [ ] exit code 為 0（不中止）
- [ ] stderr 印出 `n_neighbors will be clamped to N`
- [ ] `projected` = 實際點數

### 1-7 只有 1 筆 embedding

- [ ] exit code 為 1
- [ ] stderr 印出 `need at least 2`

### 1-8 0 筆 embedding（workspace 或 model 不存在）

傳入一個沒有任何 embedding 的 `--workspace-id`。

- [ ] exit code 為 0
- [ ] stdout `{"projected": 0, "method": "umap-v1"}`
- [ ] stderr 印出 `no embedding vectors found`

### 1-9 DB 路徑不存在

```bash
python scripts/run_umap_projection.py \
  --db-path "/nonexistent/workspace.db" \
  --workspace-id "x" --scope object --model-id "clip-vit-b32"
```

- [ ] exit code 為非 0
- [ ] stderr 有錯誤訊息（目前 `main()` 無 try/except，會是 Python traceback；
  父目錄存在時 sqlite 會自動建空 DB，訊息會變成 `no such table: embeddings`——勿寫死字串比對）

---

## 2. App 測試

### 2-1 正常流程：跑完 embedding job 後使用 UMAP

1. 開啟有圖片的 workspace
2. 進入 Dataset Map，選擇非 Fast Preview 的 model
3. 點 **Run / Refresh Embeddings**，等待完成

- [ ] progress message 顯示 `Generating UMAP projection`（非 `Generating PCA projection`）
- [ ] job 完成後散點圖出現
- [ ] 確認 DB 中 `projection_method = 'umap-v1'` row 存在

### 2-2 UMAP layout 視覺合理性

- [ ] 相同 category 的點在視覺上有聚集傾向（相較於 PCA 應更明顯分群）
- [ ] 座標分佈有覆蓋整個畫布（非全部擠在中央）

### 2-3 DB 優先序：umap-v1 > pca-v1

若 workspace 同時存在 `umap-v1` 與 `pca-v1` projection：

```sql
-- 手動插入 pca-v1 假資料，或在換版前已存在 pca-v1 的 workspace
```

- [ ] UI 顯示的座標對應 `umap-v1`（非 `pca-v1`）

### 2-4 Fast Preview 不受影響

切換回 Fast Preview model，點 Generate Preview Layout。

- [ ] 正常產生散點圖
- [ ] DB 中使用 `projection_method = 'bootstrap-deterministic'`
- [ ] progress message 不出現 UMAP 字樣

### 2-5 LazyLock：Python probe 只執行一次

在 Tauri 日誌中連續觸發兩次 embedding job（不同 scope 或 model）。

> 註：`UMAP_PYTHON` 是 `LazyLock`，只快取「`python -c "import umap"` 這個 probe 子行程」一次；
> fallback / 啟動日誌（`umap-capable Python not found`、`running UMAP via ...`）每個 job 都會印，
> 因此**不要**用日誌行數判斷 probe 是否只跑一次。

- [ ] 第二個 job 沒有 Python probe（`import umap` 子行程）的啟動延遲（probe 已被快取）
- [ ] 每個 job 仍會印出 UMAP 啟動或 fallback 日誌（屬正常，非 probe 重跑）

### 2-6 重跑 embedding job

對同一個 workspace + model 連續執行兩次 embedding job。

- [ ] 第二次正常完成，無錯誤
- [ ] `umap-v1` row 被更新（非重複新增）

---

## 3. Fallback 行為測試

> ⚠️ 重要前提：`UMAP_PYTHON` probe 在「app process 第一個 embedding job」時才初始化，並在整個 process
> 生命週期內快取結果。因此 3-1 / 3-2 必須**在啟動 app 後、跑第一個 job 之前**就把環境弄壞，
> 且每次更改環境都要**重啟 app**，否則會拿到先前快取的結果（假陰性）。

### 3-1 Python 不在 PATH

暫時將 `PATH` 中的 python 移除，或在測試環境模擬（重命名 `python.exe`），重跑 embedding job。

- [ ] Tauri 日誌出現 `umap-capable Python not found; falling back to PCA`
- [ ] job 仍然正常完成
- [ ] DB 中存在 `projection_method = 'pca-v1'`（而非 `umap-v1`）
- [ ] UI 仍能顯示散點圖（使用 PCA）

### 3-2 umap-learn 未安裝

在一個沒有安裝 `umap-learn` 的 Python 環境下觸發 job。

- [ ] Tauri 日誌出現 `falling back to PCA`
- [ ] job 正常完成
- [ ] `pca-v1` 出現於 DB

### 3-3 Script 找不到

將 `scripts/run_umap_projection.py` 暫時改名，重跑 embedding job。

- [ ] 日誌出現 `run_umap_projection.py not found; falling back to PCA`
- [ ] job 正常完成，PCA 結果寫入 DB

---

## 驗收標準

所有 1-x 和 2-1 ~ 2-4 必須通過。2-5 建議確認。3-x 為 fallback 安全網，必須通過。

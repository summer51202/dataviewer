# CLI / Agent 使用說明 — Auto Sample Selection

`scripts/run_sample_selection.py` 是一支可獨立執行的命令列工具,從 workspace 已存的 embedding
產生「覆蓋率最大化、攤平分佈」的樣本子集,並以**具名、非破壞性**的 sample set 寫回 DB,供另外匯出。
GUI 與 agent 都走同一支 CLI。設計細節見 `docs/spec-auto-sample-selection.md`。

> 這支工具是下游步驟,假設 `embeddings` 表已有資料。若你是從一份 COCO 資料集**從零開始**,
> 請看 `docs/cli-pipeline-usage.md`(ingest → 產 embedding → 採樣 → 匯出的完整流程)。

---

## 1. 從零開始:環境與安裝

### 1-1 需求

- **Python 3.9+**(`python --version` 確認;Windows 上若用 `py` 啟動器則為 `py -3`)
- pip
- 一個已經跑完 embedding 的 workspace(`embeddings` 表有資料)

採樣**不需要** `umap-learn`;覆蓋採樣與離群偵測只依賴 `numpy` + `scikit-learn`。

### 1-2 安裝依賴

```bash
pip install -r scripts/requirements-sampling.txt
```

### 1-3 驗證安裝

```bash
# 依賴可載入
python -c "import numpy, sklearn; print('ok', numpy.__version__, sklearn.__version__)"

# 端到端 self-test（建合成資料、跑完整流程、驗冪等與邊界，不碰真實 DB）
python scripts/run_sample_selection.py --self-test
```

`--self-test` 最後應印出 `[sample] self-test: ALL PASSED`。這是上線前最快的健全性檢查。

---

## 2. 找出必要參數

三個必填值都在 workspace 的 `.dataviewer/` 目錄裡。

- `--db-path`:`<workspace-root>/.dataviewer/workspace.db`
- `--workspace-id`:見 `<workspace-root>/.dataviewer/workspace.json` 的 `id` 欄位
- `--model-id`:列出該 DB 內可用的 model / scope:

```bash
sqlite3 "<workspace>/.dataviewer/workspace.db" \
  "SELECT model_id, scope, COUNT(*) FROM embeddings GROUP BY model_id, scope;"
```

> 採樣請用**非 fast-preview** 的 model(fast-preview 是 bootstrap 假投影,不是真 embedding)。

---

## 3. 執行範例

```bash
# 依比例:挑出涵蓋率最大化的 30% 圖片
python scripts/run_sample_selection.py \
  --db-path "<workspace>/.dataviewer/workspace.db" \
  --workspace-id "<id>" --scope object --model-id "<model-id>" \
  --name "coverage-30pct" --target-ratio 0.3

# 指定張數
python scripts/run_sample_selection.py ... --name "cov-500" --target-images 500

# 開啟離群剝除(非破壞性,保留每類至少 5 個物件)
python scripts/run_sample_selection.py ... --name "cov-clean" --target-ratio 0.3 \
  --remove-outliers --outlier-method knn --outlier-pct 0.02 --per-class-floor 5

# 預覽不寫 DB(agent 規劃用)
python scripts/run_sample_selection.py ... --name "preview" --target-ratio 0.3 --dry-run

# 大資料集改用 FPS(便宜、可擴展)
python scripts/run_sample_selection.py ... --name "cov-diverse" --target-ratio 0.3 --mode diverse
```

### 參數摘要

| 參數 | 預設 | 說明 |
| --- | --- | --- |
| `--db-path` `--workspace-id` `--model-id` `--name` | (必填) | 目標 DB / workspace / model / sample set 名稱 |
| `--scope` | `object` | 計算覆蓋的 scope;物件多樣性用 `object` |
| `--target-images` \| `--target-ratio` | (擇一必填) | 目標**圖片**數或比例 (0,1];互斥 |
| `--mode` | `balanced` | `balanced`(代表性、抗離群;底層 facility-location)或 `diverse`(最大鋪散、可擴展;底層 farthest-point) |
| `--remove-outliers` | 關 | 開啟非破壞性離群剝除 |
| `--outlier-method` | `knn` | `knn`(kNN 平均距離)或 `lof` |
| `--outlier-pct` | `0.02` | 標記為離群的比例上限 |
| `--per-class-floor` | `5` | 每個 category 至少保留的物件數(離群剝除的保護下限) |
| `--pca-dim` | `50` | 採樣前先降到的維度(去噪/加速);維度不足自動跳過 |
| `--fl-max-n` | `6000` | balanced 模式的候選物件上限,超過自動退回 diverse |
| `--seed` | `42` | 固定亂數,確保可重現 |
| `--dry-run` | 關 | 只計算與輸出摘要,不寫 DB |
| `--no-overwrite` | 關 | 同名 sample set 已存在時報錯而非覆寫 |

---

## 4. 輸出與 exit code

成功時 **stdout 印一行 JSON**,進度訊息走 **stderr**(前綴 `[sample]`),兩者分流方便 agent 解析。

```json
{
  "sample_set": "coverage-30pct",
  "mode": "balanced",
  "selected_images": 500,
  "selected_objects": 1320,
  "excluded_outliers": 47,
  "saturated": false,
  "seed": 42,
  "total_images": 1666
}
```

- `selected_images`:最終納入匯出的 distinct 圖片數(= sample set 大小)。
- `selected_objects`:覆蓋演算法挑中的物件數(驅動圖片納入)。
- `excluded_outliers`:被標記為離群而排除的物件數。
- `saturated`:目標 ≥ 可用圖片數,已全選。

Exit code:`0` 成功;`1` 參數或資料錯誤(stderr 有訊息)。

---

## 5. Agent 串接要點

- **解析**:只讀 stdout 最後一行 JSON;stderr 當 log,不要拿來判斷結果。
- **冪等**:相同參數(含 `--seed`)重跑,sample set 內容完全一致;同名會 upsert 覆寫,可安全重試。
- **預覽再執行**:先用 `--dry-run` 拿摘要評估張數/離群比例,確認後再正式寫入。
- **可重現稽核**:每個 sample set 在 `sample_sets.params_json` 存了完整參數快照,事後可重跑驗證。
- **不破壞原始資料**:本工具只新增 `sample_sets` / `sample_set_members` 兩張表;原始 `images` /
  `annotations` / `embeddings` 一律不動,sample set 可隨時刪除重建。

### 消費 sample set(匯出前取圖片清單)

```sql
SELECT m.image_id
FROM sample_set_members m
JOIN sample_sets s ON s.id = m.sample_set_id
WHERE s.workspace_id = '<id>' AND s.name = 'coverage-30pct'
  AND m.membership = 'selected';
```

把這批 `image_id` 餵給既有的 COCO / YOLO 匯出流程,即可得到只含選中圖片的資料集。

---

## 6. 疑難排解

| 症狀 | 處理 |
| --- | --- |
| `ModuleNotFoundError: numpy / sklearn` | 跑 §1-2 安裝依賴;確認用的是同一個 python |
| `db path does not exist` | 檢查 `--db-path`,需指到 `.dataviewer/workspace.db` |
| `no embedding vectors found` | 該 model/scope 還沒跑 embedding;見 §2 列出可用 model |
| `selected_images` 比預期少且 `saturated: true` | 目標超過可用圖片數,已全選 |
| 大資料集很慢 | 加大 `--pca-dim` 反而更慢;改 `--mode diverse`,或調低 `--fl-max-n` 觸發 diverse |
| 想完全重現某次結果 | 用該 sample set 的 `params_json` 還原所有參數重跑 |

---

## 7. 與 GUI / Tauri 的關係

本 CLI 是 agent 與後端共用的核心。Tauri 端會以相同模式(快取 Python probe、解析 script 路徑)呼叫它,
並提供建立 / 列出 / 刪除 sample set 的 command。此整合層尚未實作(見 spec §5),目前 CLI 可獨立使用。

> 注意:自動採樣一律在**高維 embedding** 上計算,與 app 上 UMAP 2D 投影**不對應**。2D 投影僅供人工挑選。

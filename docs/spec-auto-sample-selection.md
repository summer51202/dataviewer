# Spec — 自動樣本空間最佳化採樣（Auto Sample Selection）

狀態：Draft
依賴：embedding pipeline（`embeddings` 表已填）、UMAP/PCA projection（僅供視覺化，不參與採樣）

---

## 1. 目的與範圍

在已跑完 embedding 的 workspace 上，自動產生一份「覆蓋率最大化、攤平分佈」的樣本子集，供另外匯出（COCO / YOLO）使用。整套流程同時開放 CLI / JSON 介面，讓 agent 可直接呼叫，不需經過 GUI。

核心特性：

- **非破壞性**：只產生具名 selection（標記），不刪除、不更動原始 `images` / `annotations` / `embeddings`。可隨時還原或重跑。
- **物件多樣性驅動**：覆蓋採樣計算在 **object scope 的高維 embedding** 上；匯出單位為 **image**（圖含任一被選 object 即納入）。
- **與視覺化分離**：UMAP 2D 僅供 app 上人工挑選；自動採樣一律使用高維向量。UI 入口須標示「依高維特徵採樣，與畫面投影不完全對應」。
- **可重現**：固定 seed，相同參數重跑結果一致；冪等。

### Non-goals

- 不改變既有 UMAP / PCA 投影行為（見 `qa-umap-projection.md`）。
- 不做主動學習迴圈（model-in-the-loop）；本版僅基於既有 embedding 的幾何分佈。
- 不處理跨 model 混合採樣；一次 run 綁定單一 `model_id`。

---

## 2. 決策摘要（已定案）

| 議題 | 決定 |
| --- | --- |
| 分佈目標 | 攤平分佈 / 最大化覆蓋（非保留原始比例） |
| 採樣模式（mode） | `balanced` = facility-location 子模最大化（代表性、抗離群、預設）；`diverse` = FPS / k-center greedy（最大鋪散、可擴展） |
| 計算空間 | object scope 高維 embedding（cosine；可選 PCA-50 去噪） |
| 匯出單位 | image（含任一被選 object 即納入） |
| 目標數量 | 停止條件掛在 image 數 / 比例；貪婪驅動力來自 object 覆蓋 |
| 離群處理 | 可選前置步驟，非破壞性（標記排除），保留 class-aware 下限保護 |
| 還原性 | selection 為具名標記，原始資料不動，另外匯出 |
| 介面 | 獨立 CLI script + JSON stdout，沿用 `run_umap_projection.py` 模式 |

---

## 3. 演算法

### 3.1 輸入

- `workspace_id`、`scope=object`、`model_id`
- 目標：`target_images: N` 或 `target_ratio: r`（二擇一，互斥）
- 選項：`remove_outliers`、`outlier_method`、`mode`（`balanced` / `diverse`）、`pca_dim`、`seed`、`per_class_floor`

### 3.2 步驟

1. **讀取向量**：從 `embeddings` 撈出 `(target_id, image_id, annotation_id, vector)`，`scope=object`、指定 `model_id`。向量為 little-endian float32 BLOB（同 `run_umap_projection.py` 解法）。
2. **預處理**：L2 normalize（與 cosine 一致）。若 `pca_dim` 指定（預設 50），先以 PCA 降維去噪；點數不足時自動跳過。
3. **離群標記（可選，預設視方法而定）**：
   - 方法：kNN 平均距離（預設）或 LOF。
   - 門檻：percentile（例如丟最離群的 1–2%），非固定距離。
   - **class-aware 下限**：任一 category 被標記為離群的比例不得使其在最終 sample set 的代表性低於 `per_class_floor`（預設保留每類至少 K 個 object，K 可設）。
   - facility-location 為主時預設 **關閉**（方法本身抗離群）；FPS 時預設 **開啟**。
4. **覆蓋採樣**：
   - **主：facility-location 子模最大化**。最大化 Σ_i max_{j∈S} sim(i, j)，貪婪加入最能提升目標的 object；(1−1/e) 近似保證；對離群穩健。
   - **備援：FPS / k-center greedy**。每次加入離當前集合最遠的 object，最大化最小間距；便宜但易被離群吸引（故配離群剝除）。
   - 兩者皆在高維（或 PCA 後）cosine 空間運作。
5. **聚合到 image 並套用停止條件**：
   - 每選一個 object，其 `image_id` 收進 selection；同圖其他 object 視為順帶覆蓋。
   - 持續貪婪，直到 **distinct image 數** 達到 `target_images`（或 `round(total_images * target_ratio)`）。
   - 邊界：目標 ≥ 可用圖數時，全選並標記 `saturated=true`。
6. **寫出 selection**：以具名 sample set 持久化（見 §4），不動原始資料。
7. **JSON 摘要**輸出至 stdout。

### 3.3 可重現性

- 所有隨機環節（PCA solver、tie-break）固定 `seed`（預設 42，呼應 UMAP `random_state`）。
- 貪婪起點：facility-location 以 medoid 起始；FPS 以離全域中心最遠點起始；皆為確定性。

---

## 4. 資料模型（DB）

非破壞性，新增兩張表，沿用既有 `workspace.db`（schema 見 `db.rs`）。

```sql
-- 一次採樣 run 的中繼資料
CREATE TABLE IF NOT EXISTS sample_sets (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    name TEXT NOT NULL,
    scope TEXT NOT NULL,              -- 'object'
    model_id TEXT NOT NULL,
    coverage_method TEXT NOT NULL,    -- mode 值：'balanced' | 'diverse'（欄名沿用 coverage_method）
    params_json TEXT NOT NULL,        -- 完整參數快照（含 seed），供重現/稽核
    target_images INTEGER,
    target_ratio REAL,
    selected_images INTEGER NOT NULL,
    selected_objects INTEGER NOT NULL,
    excluded_outliers INTEGER NOT NULL,
    saturated INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(workspace_id, name),
    FOREIGN KEY(workspace_id) REFERENCES workspace_meta(id)
);

-- 每個 sample set 的成員（image 為匯出單位；保留觸發的 object 以利稽核）
CREATE TABLE IF NOT EXISTS sample_set_members (
    id TEXT PRIMARY KEY,
    sample_set_id TEXT NOT NULL,
    image_id TEXT NOT NULL,
    membership TEXT NOT NULL,         -- 'selected' | 'outlier-excluded'
    trigger_object_id TEXT,           -- 把這張圖帶進來的 object（selected 時）
    created_at TEXT NOT NULL,
    UNIQUE(sample_set_id, image_id),
    FOREIGN KEY(sample_set_id) REFERENCES sample_sets(id)
);

CREATE INDEX IF NOT EXISTS idx_sample_set_members_set ON sample_set_members(sample_set_id);
```

設計取捨：

- 不重用 `dataset_review_marks`，因為它的語意是「人工 review 狀態」、且 `UNIQUE(workspace_id, scope, target_id)` 每個 target 只能有一筆，無法支援多份並存的具名 selection。sample set 需要可多份、可命名、可 diff。
- `params_json` 存完整參數＋seed＋程式版本，確保任何一份 selection 都能被重跑驗證。

---

## 5. CLI / Agent 介面

新增 `scripts/run_sample_selection.py`，模式與 `run_umap_projection.py` 一致（Python 端算、寫回 DB、stdout 吐 JSON、stderr 印進度）。

```bash
python scripts/run_sample_selection.py \
  --db-path   "<workspace>/.dataviewer/workspace.db" \
  --workspace-id "<id>" \
  --scope     object \
  --model-id  "<model-id>" \
  --name      "coverage-30pct" \
  (--target-images 500 | --target-ratio 0.3) \
  [--mode balanced|diverse] \
  [--remove-outliers] [--outlier-method knn|lof] [--outlier-pct 0.02] \
  [--per-class-floor 5] \
  [--pca-dim 50] \
  [--seed 42] \
  [--dry-run]
```

行為約定：

- `--target-images` 與 `--target-ratio` 互斥，未給其一 → exit 1。
- `--dry-run`：只算並輸出摘要，不寫 DB（agent 預覽用）。
- 同名 `--name` 重跑 → upsert（覆寫該 sample set，冪等）。
- stdout 成功輸出單行 JSON：

```json
{
  "sample_set": "coverage-30pct",
  "mode": "balanced",
  "selected_images": 500,
  "selected_objects": 1320,
  "excluded_outliers": 47,
  "saturated": false,
  "seed": 42
}
```

- exit code：`0` 成功；`1` 參數/資料錯誤（stderr 有訊息）。
- stderr 進度：`[sample] reading embeddings`、`[sample] preprocessing`、`[sample] detecting outliers`（若開）、`[sample] running <method>`、`[sample] writing sample set`。

### Rust / Tauri 整合

- 比照 `generate_umap_embedding_projections`：以 `LazyLock` 快取 umap-capable Python probe（可共用同一支 probe），解析 script 路徑（`resolve_*_script` 模式）。
- 新增 command（`commands/` 下，依領域可放 `browser.rs` 或新 `sampling.rs`）：建立 / 列出 / 刪除 sample set、觸發採樣。於 `lib.rs` 註冊，`api.ts` 加 wrapper + mock fallback（遵循 CLAUDE.md「新增 Tauri command」流程）。
- **無 Python fallback 策略**：與 UMAP 不同，覆蓋採樣沒有純 Rust 等價實作。若 Python/依賴不可用，command 回傳明確錯誤（不靜默退化），UI 顯示「需要 Python 環境」提示。理由：採樣結果直接決定匯出內容，靜默用劣化演算法比直接失敗更危險。

---

## 6. 邊界情況

| 情況 | 行為 |
| --- | --- |
| 0 筆 embedding | exit 0，`selected_images=0`，stderr `no embedding vectors found` |
| 1 筆 embedding | 全選該圖，`saturated=true`，exit 0 |
| target ≥ 可用圖數 | 全選，`saturated=true` |
| target_ratio 算出 0 張（極小比例） | 至少選 1 張，stderr 警告 |
| 某類別過小（< per_class_floor） | 該類全保留，不受離群剝除影響 |
| PCA 點數不足（< pca_dim） | 跳過 PCA，直接用原始向量，stderr 註記 |
| 同名 sample set 已存在 | upsert 覆寫（除非 `--no-overwrite`，則 exit 1） |
| Python / umap-learn 不可用 | command 明確報錯，不退化 |

---

## 7. 驗收標準

CLI

- [ ] happy path：`--target-ratio 0.3` 與 `--target-images N` 皆能產生 sample set，JSON 摘要正確
- [ ] 冪等：相同參數重跑，`sample_set_members` 內容完全一致（固定 seed）
- [ ] `selected_images` 等於實際 distinct image 數，且 ≤ target（saturated 時等於可用圖數）
- [ ] `--remove-outliers` 開啟時，被排除者寫為 `outlier-excluded`，且任一類別代表性不低於 `per_class_floor`
- [ ] `--dry-run` 不寫 DB
- [ ] 互斥旗標檢查、0/1 筆、saturated 等邊界皆符合 §6

方法品質

- [ ] facility-location 結果在高維 cosine 空間的覆蓋（平均最近選中點相似度）優於同數量隨機抽樣
- [ ] 攤平效果：相較原始分佈，稠密類別被相對降採樣、稀疏類別被相對升採樣（以類別佔比變化驗證）
- [ ] 採樣在高維、非 2D UMAP 上進行（程式碼層級確認，無讀 `embedding_projections`）

整合

- [ ] Tauri command 可建立 / 列出 / 刪除 sample set
- [ ] 匯出流程能以 sample set 為輸入產生 COCO / YOLO，且只含選中 image
- [ ] Python 不可用時明確報錯，非靜默退化

---

## 8. 待確認 / 未來延伸

- 是否需要在 UI 上預覽 sample set（把選中點疊在 UMAP 2D 上，僅視覺化）。
- 是否提供「在既有 sample set 上增量擴充到更大 N」（facility-location 貪婪天然支援續算）。
- 多 model / 多 scope 混合採樣（本版排除）。
- 主動學習迴圈（model uncertainty 加權）——後續版本。

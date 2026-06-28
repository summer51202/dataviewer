# CLI / Agent 完整 Pipeline — 從一份 COCO 資料集到採樣匯出

這份說明涵蓋**全程 CLI、不需 GUI** 的流程,適合 agent 自動化:把一份 COCO 資料集
ingest 進 workspace、產生 embeddings、做覆蓋率最大化採樣,最後取出選中的圖片清單匯出。

```
COCO 資料集
  │  ① run_coco_ingest.py          → 建立 .dataviewer workspace（images/annotations/categories）
  ▼
workspace.db
  │  ② run_generate_embeddings.py  → 寫入 embeddings 表（CLI 自有 model_id 命名空間）
  ▼
embeddings
  │  ③ run_sample_selection.py     → 覆蓋率最大化採樣，寫入 sample_sets / sample_set_members
  ▼
sample set
  │  ④ SQL 取出 selected image_id → 餵給 COCO / YOLO 匯出
  ▼
子資料集
```

> 採樣參數的完整細節見 `docs/cli-sample-selection-usage.md`。本文聚焦端到端串接。

---

## 0. 環境

- Python 3.9+(Windows 用 `py -3`)
- 三組依賴,按需安裝:

```bash
pip install -r scripts/requirements-sampling.txt    # 採樣（numpy + scikit-learn）
pip install -r scripts/requirements-embeddings.txt   # 產 embedding（torch + open_clip，較大）
# ingest 只用 Python 標準庫，無需額外安裝
```

每支工具都有離線 self-test,安裝後先各跑一次確認:

```bash
python scripts/run_coco_ingest.py --self-test
python scripts/run_generate_embeddings.py --self-test   # 用 --mock，不需 torch
python scripts/run_sample_selection.py --self-test
python scripts/run_pipeline.py --self-test              # 端到端整合測試（需 numpy+scikit-learn）
```

都印出 `ALL PASSED` 即可。`run_pipeline.py --self-test` 會用合成資料與 mock 向量把四步整條跑一遍。

> **不想裝 torch 也能跑完整鏈**:②可加 `--mock` 寫入確定性假向量(只需 numpy),
> 用來驗證 ingest → embed → sample → 匯出整條流程是否打通,之後再換成真模型重跑②即可。

---

## 1. 一鍵執行 run_pipeline.py（推薦）

一支指令把四步跑完並輸出採樣後的 COCO 子集。底層仍是分別呼叫四支工具(解析各自的 stdout JSON),
所以行為與分步執行完全一致。

真模型:

```bash
python scripts/run_pipeline.py \
  --coco       path/to/instances.json \
  --images-dir path/to/images \
  --workspace-root path/to/my-workspace \
  --name "My Dataset" \
  --scope object --target-ratio 0.3 --remove-outliers
```

先用 mock 打通整條鏈(不需 torch,不需真圖檔):

```bash
python scripts/run_pipeline.py --coco ... --images-dir ... --workspace-root ... \
  --target-ratio 0.3 --mock
```

**Roboflow 多 split 一鍵**:用 `--dataset-root` 取代 `--coco`/`--images-dir`,會把
`train/valid/test` 全部匯入同一 workspace、跨所有 split 產 embeddings 與採樣:

```bash
python scripts/run_pipeline.py \
  --dataset-root "path/to/roboflow-export" \
  --workspace-root path/to/my-workspace --name "My Dataset" \
  --scope object --target-ratio 0.3 --mode balanced --mock
```

輸出單行 JSON,匯總四步結果並附上 COCO 子集路徑:

```json
{"workspace_id": "my-dataset", "model_id": "clip-vit-b-32", "sample_set": "pipeline",
 "ingest": {"images": 120, "annotations": 530, "categories": 4},
 "embed":  {"embedded": 530, "dim": 512, "backend": "cuda"},
 "sample": {"selected_images": 36, "excluded_outliers": 10, "saturated": false},
 "export": {"path": ".../.dataviewer/exports/pipeline.coco.json", "images": 36, "annotations": 152}}
```

常用參數:`--target-images N` / `--target-ratio r`(擇一,預設 ratio 0.3)、`--sample-name`、
`--mode`、`--remove-outliers`、`--mock` / `--mock-dim`、`--export-coco PATH`(自訂輸出)、
`--no-export`、`--python`(指定子步驟用的直譯器)。完整旗標見 `run_pipeline.py -h`。

第 4 步會依選中的 `file_name` 過濾出子集,預設寫到
`<workspace>/.dataviewer/exports/<sample-name>.coco.json`,原始資料不動。多 split(`--dataset-root`)
時會把各 split 的 image/annotation **合併並重新編號**、類別依名稱統一,輸出成一份合法的單一 COCO。

若要分步執行或除錯,見以下各步驟。

---

## 2. 分步:① Ingest — COCO → workspace

```bash
python scripts/run_coco_ingest.py \
  --coco        path/to/instances.json \
  --images-dir  path/to/images \
  --workspace-root  path/to/my-workspace \
  --name "My Dataset"
```

- 會在 `path/to/my-workspace/.dataviewer/` 建立 `workspace.json` 與 `workspace.db`。
- 只建立本工具會寫的表;之後用 GUI 開啟此 workspace 時,DataViewer 會自動補齊其餘表,保持相容。
- 輸出 JSON 含 **`workspace_id`**,後續每一步都要用它:

```json
{"workspace_id": "my-dataset", "workspace_db": ".../workspace.db",
 "images": 120, "annotations": 530, "categories": 4}
```

> agent 請從 stdout JSON 取 `workspace_id` 與 `workspace_db`,別自行猜測。

**Roboflow 多 split 一次匯入**:Roboflow 的 COCO 匯出是 `train/ valid/ test/` 三個夾,各自有
`_annotations.coco.json` + 圖片在同層。用 `--dataset-root` 指到匯出根目錄,會把所有 split 匯入
**同一個 workspace**(類別依名稱合併、每個 split 一個 source folder):

```bash
python scripts/run_coco_ingest.py \
  --dataset-root "path/to/roboflow-export" \
  --workspace-root path/to/my-workspace --name "My Dataset"
# 輸出含 splits 明細：{"images": 230, "annotations": 245, "categories": 2,
#   "splits": [{"split": "train", ...}, {"split": "valid", ...}, {"split": "test", ...}]}
```

`run_pipeline.py` 也支援 `--dataset-root`(取代 `--coco`/`--images-dir`),會跨所有 split
產生 embeddings、採樣,並輸出一份**合併後的** COCO 子集。

---

## 3. 分步:② 產生 embeddings

真模型(open_clip,物件級):

```bash
python scripts/run_generate_embeddings.py \
  --db-path "<workspace_db>" --workspace-id "<workspace_id>" \
  --scope object --model ViT-B-32 --pretrained laion2b_s34b_b79k \
  --batch-size 64 --device auto
```

先打通流程的免 torch 版本:

```bash
python scripts/run_generate_embeddings.py \
  --db-path "<workspace_db>" --workspace-id "<workspace_id>" \
  --scope object --mock --mock-dim 64
```

輸出:

```json
{"embedded": 530, "scope": "object", "model_id": "clip-vit-b-32",
 "dim": 512, "skipped": 4, "backend": "cuda"}
```

重點:

- `--scope object` 編碼每個標註的 **bbox 裁切**(物件多樣性);`--scope image` 編碼整張圖。
- 記下輸出的 **`model_id`**(真模型預設 `clip-<model>`,mock 為 `mock-<dim>`),③要用同一個。
- 這條 CLI 軌道與 GUI 的 ONNX 產生器**各自獨立**;同一個 `model_id` 下只放同一個產生器的向量,別混用。
- 重跑同一 `model_id` 為 upsert(覆寫),可安全重試。
- GPU:`--device auto` 偵測 CUDA;CUDA 版 torch 請先依 https://pytorch.org 安裝對應 wheel。

---

## 4. 分步:③ 採樣

```bash
python scripts/run_sample_selection.py \
  --db-path "<workspace_db>" --workspace-id "<workspace_id>" \
  --scope object --model-id "<model_id from step ②>" \
  --name "coverage-30pct" --target-ratio 0.3 \
  --remove-outliers
```

輸出摘要(`selected_images` 即子資料集大小):

```json
{"sample_set": "coverage-30pct", "mode": "balanced",
 "selected_images": 36, "selected_objects": 95, "excluded_outliers": 10,
 "saturated": false, "seed": 42, "total_images": 120}
```

參數細節(mode、outlier 選項、pca-dim、target-images vs ratio 等)見
`docs/cli-sample-selection-usage.md` §3。

---

## 5. 分步:④ 取出選中圖片並匯出

```sql
SELECT i.original_path, i.file_name
FROM sample_set_members m
JOIN sample_sets s ON s.id = m.sample_set_id
JOIN images i ON i.id = m.image_id
WHERE s.workspace_id = '<workspace_id>' AND s.name = 'coverage-30pct'
  AND m.membership = 'selected';
```

把這批圖片(及其在 `annotations` 表中的標註)輸出成新的 COCO / YOLO。原始資料完全未動,
sample set 可隨時刪除重建,也可建立多份不同比例/方法的 sample set 並存比較。

---

## 6. Agent 串接備忘

1. **串接靠 stdout JSON**:每步只讀最後一行 JSON,stderr 當 log。
2. **id 一路傳遞**:①給 `workspace_id`/`workspace_db` → ②給 `model_id` → ③用同一組。
3. **冪等**:②③同參數重跑都是 upsert,可安全重試;③固定 `--seed` 結果可重現。
4. **先 mock 後真跑**:②先 `--mock` 打通全鏈,再換真模型重跑②、接著③。
5. **非破壞**:全程不改原始 images/annotations,只新增 embeddings / sample_sets / sample_set_members。

---

## 7. 已知限制

- ② 的 open_clip 路徑需要 torch(體積大);CPU 可跑但慢,大資料集建議 GPU。
- ③ balanced 模式候選物件超過 `--fl-max-n`(預設 6000)會自動退回 diverse;超大資料集請直接用
  `--mode diverse`。
- 本機若無法執行(例如環境受限),三支工具的 `--self-test` 是最快的離線驗證手段。
- Tauri/GUI 整合層(把這些工具接進 app 與 agent command)尚未實作,目前以 CLI 形式提供。

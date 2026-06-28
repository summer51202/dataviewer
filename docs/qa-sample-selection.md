# 人工驗證 — Auto Sample Selection（In-App + CLI）

功能範圍：在已有 embeddings 的 workspace 上,於 DataViewer app 內以覆蓋率最大化採樣產生具名
sample set,並用該 sample set 篩選匯出 COCO/YOLO 子集;同時涵蓋底層 CLI 的端到端驗證。

採樣有兩種 **mode**:
- **Balanced**(預設,底層 facility-location)— 代表性、抗離群。
- **Diverse**(底層 farthest-point)— 最大鋪散、可擴展。

設計與參數細節:`docs/cli-pipeline-usage.md`、`docs/spec-auto-sample-selection.md`。

---

## 0. 前置條件

```bash
pip install -r scripts/requirements-sampling.txt     # numpy + scikit-learn（採樣必需）

# 離線自測（最快的健全性檢查）
python scripts/run_coco_ingest.py --self-test
python scripts/run_generate_embeddings.py --self-test
python scripts/run_sample_selection.py --self-test
python scripts/run_pipeline.py --self-test
```

- [ ] 四個 self-test 都印出 `ALL PASSED`

關鍵前提（會影響 app 內能不能採樣）：

1. **採樣在 app 內由後端呼叫系統 `python`**,所以 PATH 上的那支 `python` 必須裝得到 numpy + scikit-learn。
2. **app 採樣面板用「編碼器下拉選中的 model_id」**,而下拉只列內建編碼器(`clip-vit-b32`、`dinov2-small`、
   `fast-preview`)。因此 workspace 必須在**某個內建 model_id** 底下有 embeddings,面板才會啟用(選
   `fast-preview` 會顯示「需要真 embeddings」提示)。
3. 要在 app 內**用真模型產生 embeddings**,需以 `npm run tauri:dev:onnx` 啟動(帶 `onnx-runtime` feature),
   且 `onnxruntime.dll`(≥1.20)放在 `<WS>/.dataviewer/runtime/`。

以下以 `<WS>` 代表 workspace 根目錄,`<DB>` 代表 `<WS>/.dataviewer/workspace.db`。

---

## 1. 準備一個可採樣的 workspace

選一條路把 embeddings 準備好(掛在內建 model_id 下):

**路線 A — 真模型(在 app 內跑)**
1. 以 `npm run tauri:dev:onnx` 啟動,Open Workspace 開 `<WS>`。
2. Dataset Map → Encoder 選 **CLIP ViT-B/32** 或 **DINOv2 Small** → **Run / Refresh Embeddings**,等完成。

**路線 B — 免 torch/onnx(CLI mock,適合先驗流程)**
```bash
# 單一 COCO 檔
python scripts/run_coco_ingest.py --coco <instances.json> --images-dir <images> \
  --workspace-root <WS> --name "QA Dataset"        # 記下輸出的 workspace_id
# 或 Roboflow 匯出(train/valid/test 一次匯入同一 workspace)
python scripts/run_coco_ingest.py --dataset-root <roboflow-export-root> \
  --workspace-root <WS> --name "QA Dataset"

python scripts/run_generate_embeddings.py --db-path <DB> --workspace-id <id> \
  --scope object --mock --mock-dim 64 --model-id clip-vit-b32   # 掛在內建 id 下
# 選用：產投影讓散點圖有點（疊圖預覽才看得到）
python scripts/run_umap_projection.py --db-path <DB> --workspace-id <id> \
  --scope object --model-id clip-vit-b32
```

- [ ] `embeddings` 表在 `clip-vit-b32`(或 `dinov2-small`)底下有資料

---

## 2. App 採樣面板（核心）

啟動 app、Open Workspace 開 `<WS>`、進入 **Dataset Map**,Encoder 選步驟 1 用的那個(非 Fast Preview)。
在下方 **Auto Sample Selection** 面板:

1. Sample set name 填 `qa-balanced`
2. Target 選 **Ratio**,填 `0.3`(或選 Image count 填張數)
3. **Mode** 下拉 → **Balanced (representative)**
4. Outliers 下拉 → 視需要選 Exclude outliers
5. 按 **Run sampling**

- [ ] 執行中出現不確定型(跑動)進度條,標示 `balanced`
- [ ] 完成後出現摘要:`Selected N images (M objects)…`,N 約為總圖數 30%
- [ ] 下方列表多一筆 `qa-balanced`,顯示 `N images · balanced · clip-vit-b32`
- [ ] **Fast Preview 編碼器下**面板顯示「需要真 embeddings」提示、不可採樣

再做一筆 Diverse 對照:Mode 改 **Diverse (max spread)**、name 改 `qa-diverse`、Run。

- [ ] 進度條標示 `diverse`,完成後 `qa-diverse` 與 `qa-balanced` **並存**於列表(可多份)

---

## 3. App 疊圖預覽

在 sample set 列表點某筆的 **Preview on map**(需步驟 1 有產投影,散點圖才有點):

- [ ] 按鈕變為 **Previewing**;散點圖中屬於該 set 的點以**綠色描邊高亮**、其餘**淡化**
- [ ] 散點圖上方標題顯示 `sample "<name>" highlighted (K images)`,K = 該 set 圖數
- [ ] 再按一次取消預覽,高亮消失
- [ ] 切換到另一筆的 Preview,高亮點隨之改變

> 注意:散點是 UMAP/PCA 的 2D 投影,**與採樣所用的高維空間不對應**;疊圖只是把「被選中的圖片」
> 標出來,不代表採樣是在這張 2D 圖上做的。

---

## 4. App 以 sample set 篩選匯出（end-to-end 閉環）

進入 **Export** 頁:

1. **Sample Set** 下拉 → 選 `qa-balanced`

- [ ] 出現「Sample set scope active: qa-balanced」橫幅
- [ ] Export Summary 的 **Images Included** = 該 set 的選中張數(覆寫 Browser scope)
- [ ] 設輸出資料夾 → **Start Export** → 完成横幅出現
- [ ] 開啟輸出資料夾,匯出的 COCO/YOLO **只含被選中的圖片**,標註完整、categories 不缺

2. Sample Set 下拉切回 **None (use Browser scope)**

- [ ] 橫幅消失,範圍回到原本的 Browser scope

---

## 5. 刪除與非破壞性

回 Dataset Map,點 `qa-diverse` 的 **Delete**:

- [ ] 列表該筆消失;若正在 Preview 它,高亮一併清除
- [ ] Export 頁的 Sample Set 下拉也不再有 `qa-diverse`
- [ ] 原始 `images` / `annotations` / `embeddings` 不受影響(Browser/Dataset Map 內容不變)

DB 層快速確認(非破壞、具名、可多份):

```sql
SELECT name, coverage_method, selected_images, selected_objects, excluded_outliers, saturated
FROM sample_sets;                                  -- coverage_method 欄位存的就是 mode 值（balanced/diverse）
SELECT membership, COUNT(*) FROM sample_set_members GROUP BY membership;
```

- [ ] 相同名稱、相同參數重跑為 upsert(覆寫,非新增)
- [ ] `sample_set_members` 的 `selected` 數量等於面板摘要的 selected images

---

## 6. CLI 端到端（後端契約,獨立於 GUI）

一鍵驗證底層四步串接(可用 `--mock` 免 torch):

```bash
python scripts/run_pipeline.py \
  --coco <instances.json> --images-dir <images> \
  --workspace-root <WS2> --name "QA CLI" \
  --scope object --target-ratio 0.3 --mode balanced --remove-outliers --mock
```

- [ ] exit code 0;stdout 單行 JSON 含 `ingest`/`embed`/`sample`/`export` 四段
- [ ] `sample.mode` 為 `balanced`,`sample.selected_images` 約總圖數 30%
- [ ] `export.path` 的 `*.coco.json` 存在,且圖數 = `sample.selected_images`

把匯出子集當新 workspace 再開,肉眼複核(選用):

```bash
python scripts/run_coco_ingest.py --coco "<export.path>" --images-dir <images> \
  --workspace-root <WS2>-sampled --name "QA Sampled"
```

- [ ] GUI 開 `<WS2>-sampled`,Browser 圖數 = 選中張數,且都是原集子集、標註與類別完整

---

## 7. 邊界與 fallback

- [ ] `--target-ratio 1.0`(或面板填滿):`saturated` 為 true,選中圖數 = 可用圖數
- [ ] 很小的目標(target images = 1):正常產生,至少 1 張
- [ ] 開啟 outlier 剝除:`excluded_outliers > 0`,且每個 category 仍保留(class-aware 下限)
- [ ] 大資料集 / **Diverse**:正常完成(Balanced 候選超過 `--fl-max-n` 會自動退回 diverse,log 有提示)
- [ ] 系統 `python` 缺 numpy/scikit-learn 時:app 採樣顯示**明確錯誤訊息**(含 Python stderr 末行),非靜默劣化

---

## 驗收標準

- §2(app 採樣面板)、§4(以 sample set 匯出)、§5(刪除/非破壞)為本次功能核心,必須通過。
- §3(疊圖預覽)建議確認;需有投影資料才看得到高亮(mock 分群無語意,以「能標示選中點」為準)。
- §6(CLI 端到端)必須通過——這是 GUI 與 agent 共用的後端契約。
- §7 為邊界安全網,建議通過。

# DataViewer Sampling CLI — Agent Handoff (v0.2.0)

A self-contained CLI for turning a COCO (or Roboflow) detection dataset into a
coverage-maximising sampled subset. **No app build or GUI is required** — these
are plain Python scripts. Designed to be driven by an automation agent.

## What's in this bundle

```
scripts/
  run_coco_ingest.py          # COCO / Roboflow export  -> .dataviewer workspace
  run_generate_embeddings.py  # images/objects          -> embeddings table
  run_sample_selection.py     # coverage-max sampling    -> named sample set
  run_pipeline.py             # one-shot: all of the above + export a COCO subset
  run_umap_projection.py      # (optional) 2D projection for the desktop app
  requirements-sampling.txt   # numpy + scikit-learn   (sampling — required)
  requirements-embeddings.txt # torch + open_clip      (real embeddings only)
docs/                         # this file + full usage / spec / QA
```

## Setup

- Python 3.9+
- `pip install -r scripts/requirements-sampling.txt`
- For real (non-mock) embeddings: `pip install -r scripts/requirements-embeddings.txt`
  (large: torch + open_clip; first real run downloads ~hundreds of MB of model
  weights from the internet)

Verify everything offline (no real data needed):

```bash
python scripts/run_coco_ingest.py --self-test
python scripts/run_generate_embeddings.py --self-test
python scripts/run_sample_selection.py --self-test
python scripts/run_pipeline.py --self-test
```

All four must print `ALL PASSED`.

## Run it (one shot)

Single COCO file:

```bash
python scripts/run_pipeline.py \
  --coco path/to/instances.json --images-dir path/to/images \
  --workspace-root path/to/out-workspace --name "My Dataset" \
  --scope object --target-ratio 0.3 --mode balanced --mock
```

Roboflow export (a folder with train/valid/test, each containing
`_annotations.coco.json` + images) — ingests all splits into one workspace:

```bash
python scripts/run_pipeline.py \
  --dataset-root path/to/roboflow-export \
  --workspace-root path/to/out-workspace --name "My Dataset" \
  --scope object --target-ratio 0.3 --mode balanced --mock
```

Drop `--mock` and add `--model ViT-B-32 --pretrained laion2b_s34b_b79k` to use a
real CLIP encoder instead of deterministic mock vectors.

## Modes

- `--mode balanced` (default) — representative coverage, robust to outliers.
- `--mode diverse` — maximum spread / edge cases; scales to large sets.
- `--target-ratio R` (0–1) or `--target-images N` — how big the subset is (by images).
- `--remove-outliers` — optional non-destructive outlier exclusion.

## Output / machine contract

- **stdout**: a single JSON line (parse the LAST non-empty line). stderr is logs only.
- **exit code**: `0` success, non-zero on error (message on stderr).
- Pipeline JSON summarises every stage and the written subset path, e.g.:

```json
{"workspace_id": "my-dataset", "model_id": "mock-64", "sample_set": "pipeline",
 "ingest": {"images": 205, "annotations": 220, "categories": 2},
 "embed":  {"embedded": 220, "dim": 64, "backend": "mock"},
 "sample": {"selected_images": 61, "mode": "balanced", "saturated": false, "total_images": 203},
 "export": {"path": ".../.dataviewer/exports/pipeline.coco.json", "images": 61, "annotations": 71}}
```

- The **sampled subset** is written to
  `<workspace-root>/.dataviewer/exports/<sample-name>.coco.json` (override with
  `--export-coco PATH`). Original data is never modified.

## Notes / gotchas

- Install deps into the **same** `python` the agent invokes.
- Idempotent: re-running with the same args + `--seed` upserts identical results.
- `total_images` counts images that have objects; images with no annotations are
  not part of object-scope sampling (so it can be < ingest's image count).
- Multi-split export merges + renumbers ids and unifies categories by name into
  one valid COCO.
- Quote paths that contain spaces.
- Full reference: `docs/cli-pipeline-usage.md`, `docs/cli-sample-selection-usage.md`,
  `docs/spec-auto-sample-selection.md`.

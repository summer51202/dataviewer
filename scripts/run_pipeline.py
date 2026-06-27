"""
One-shot CLI pipeline: COCO dataset -> sampled sub-dataset.

Runs the four steps end to end:
    1. run_coco_ingest.py          COCO -> .dataviewer workspace
    2. run_generate_embeddings.py  images/objects -> embeddings table
    3. run_sample_selection.py     coverage-max sampling -> named sample set
    4. export                       selected image_ids -> filtered COCO subset

Each step is invoked as a subprocess and its stdout JSON is parsed, so the same
stable CLI contract agents use is exercised here. Original data is never modified.

Usage (real model):
    python run_pipeline.py \
        --coco path/to/instances.json --images-dir path/to/images \
        --workspace-root path/to/ws --name "My Dataset" \
        --target-ratio 0.3 --remove-outliers

Usage (no torch, smoke-test the whole chain with mock vectors):
    python run_pipeline.py --coco ... --images-dir ... --workspace-root ... \
        --target-ratio 0.3 --mock

Offline integration check (synthetic data, no real images, mock vectors):
    python run_pipeline.py --self-test

Exit codes: 0 success; non-zero if any step fails.
Stdout on success: single-line JSON summarising every stage.
"""
from __future__ import annotations

import argparse
import json
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent


def log(msg: str) -> None:
    print(f"[pipeline] {msg}", file=sys.stderr, flush=True)


def fail(msg: str, code: int = 1):
    print(f"[pipeline] ERROR: {msg}", file=sys.stderr, flush=True)
    sys.exit(code)


def call(python: str, script: str, args: "list[str]") -> dict:
    """Run a sibling script, stream its stderr, return parsed stdout JSON."""
    cmd = [python, str(SCRIPTS / script), *args]
    log("$ " + " ".join(cmd))
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.stderr:
        sys.stderr.write(proc.stderr)
        sys.stderr.flush()
    if proc.returncode != 0:
        fail(f"{script} failed (exit {proc.returncode})", proc.returncode or 1)
    lines = [ln for ln in proc.stdout.splitlines() if ln.strip()]
    if not lines:
        fail(f"{script} produced no JSON output")
    try:
        return json.loads(lines[-1])
    except json.JSONDecodeError as e:  # noqa: BLE001
        fail(f"could not parse {script} output as JSON: {e}\n{lines[-1]}")


def discover_coco_paths(args) -> "list[Path]":
    """COCO json file(s) to filter for the subset export: the single --coco, or
    every `_annotations.coco.json` under a Roboflow --dataset-root."""
    if args.dataset_root is None:
        return [args.coco]
    root = args.dataset_root
    paths = []
    if (root / "_annotations.coco.json").exists():
        paths.append(root / "_annotations.coco.json")
    for child in sorted(p for p in root.iterdir() if p.is_dir()):
        ann = child / "_annotations.coco.json"
        if ann.exists():
            paths.append(ann)
    return paths


def export_subset(coco_paths, db_path: Path, workspace_id: str,
                  sample_name: str, out_path: Path) -> dict:
    """Build a merged COCO subset (across one or more split files) containing only
    the sample set's selected images. Image/annotation ids are renumbered and
    categories unified by name, so multi-split output is a valid single COCO."""
    con = sqlite3.connect(str(db_path))
    try:
        rows = con.execute(
            """
            SELECT i.file_name
            FROM sample_set_members m
            JOIN sample_sets s ON s.id = m.sample_set_id
            JOIN images i ON i.id = m.image_id
            WHERE s.workspace_id = ? AND s.name = ? AND m.membership = 'selected'
            """,
            (workspace_id, sample_name),
        ).fetchall()
    finally:
        con.close()
    selected_files = {r[0] for r in rows}

    images_out, anns_out, cats_out = [], [], []
    cat_id_by_name: dict = {}
    next_cat = next_img = next_ann = 1
    for path in coco_paths:
        coco = json.loads(Path(path).read_text(encoding="utf-8"))
        local_cat = {}
        for c in coco.get("categories", []):
            key = str(c.get("name", c.get("id")))
            if key not in cat_id_by_name:
                cat_id_by_name[key] = next_cat
                cats_out.append({"id": next_cat, "name": key,
                                 "supercategory": c.get("supercategory", "")})
                next_cat += 1
            local_cat[c["id"]] = cat_id_by_name[key]
        local_img = {}
        for im in coco.get("images", []):
            if im.get("file_name") in selected_files:
                local_img[im["id"]] = next_img
                images_out.append({**im, "id": next_img})
                next_img += 1
        for a in coco.get("annotations", []):
            if a.get("image_id") in local_img:
                anns_out.append({**a, "id": next_ann,
                                 "image_id": local_img[a["image_id"]],
                                 "category_id": local_cat.get(a.get("category_id"))})
                next_ann += 1

    subset = {"images": images_out, "annotations": anns_out, "categories": cats_out}
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(subset), encoding="utf-8")
    return {"path": str(out_path), "images": len(images_out), "annotations": len(anns_out)}


def run_pipeline(args) -> dict:
    python = args.python or sys.executable

    # ----- 1. ingest -----
    if args.dataset_root is not None:
        ingest_args = [
            "--dataset-root", str(args.dataset_root),
            "--workspace-root", str(args.workspace_root),
        ]
    else:
        ingest_args = [
            "--coco", str(args.coco),
            "--images-dir", str(args.images_dir),
            "--workspace-root", str(args.workspace_root),
        ]
    if args.name:
        ingest_args += ["--name", args.name]
    if args.workspace_id:
        ingest_args += ["--workspace-id", args.workspace_id]
    s_ingest = call(python, "run_coco_ingest.py", ingest_args)
    workspace_id = s_ingest["workspace_id"]
    workspace_db = s_ingest["workspace_db"]

    # ----- 2. embeddings -----
    embed_args = [
        "--db-path", workspace_db, "--workspace-id", workspace_id,
        "--scope", args.scope,
    ]
    if args.mock:
        embed_args += ["--mock", "--mock-dim", str(args.mock_dim)]
    else:
        embed_args += [
            "--model", args.model, "--pretrained", args.pretrained,
            "--device", args.device, "--batch-size", str(args.batch_size),
        ]
    if args.limit:
        embed_args += ["--limit", str(args.limit)]
    s_embed = call(python, "run_generate_embeddings.py", embed_args)
    model_id = s_embed["model_id"]

    # ----- 3. sampling -----
    sample_args = [
        "--db-path", workspace_db, "--workspace-id", workspace_id,
        "--scope", args.scope, "--model-id", model_id,
        "--name", args.sample_name,
        "--mode", args.mode,
        "--per-class-floor", str(args.per_class_floor),
        "--pca-dim", str(args.pca_dim),
        "--seed", str(args.seed),
    ]
    if args.target_images is not None:
        sample_args += ["--target-images", str(args.target_images)]
    else:
        sample_args += ["--target-ratio", str(args.target_ratio)]
    if args.remove_outliers:
        sample_args += ["--remove-outliers", "--outlier-method", args.outlier_method,
                        "--outlier-pct", str(args.outlier_pct)]
    s_sample = call(python, "run_sample_selection.py", sample_args)

    summary = {
        "workspace_id": workspace_id,
        "workspace_db": workspace_db,
        "model_id": model_id,
        "sample_set": args.sample_name,
        "ingest": s_ingest,
        "embed": s_embed,
        "sample": s_sample,
    }

    # ----- 4. export subset -----
    if not args.no_export:
        out_path = args.export_coco or (Path(workspace_db).parent / "exports" / f"{args.sample_name}.coco.json")
        s_export = export_subset(discover_coco_paths(args), Path(workspace_db), workspace_id, args.sample_name, Path(out_path))
        log(f"wrote COCO subset: {s_export['path']} ({s_export['images']} images)")
        summary["export"] = s_export

    return summary


def parse_args(argv=None):
    p = argparse.ArgumentParser(description="One-shot COCO -> sampled sub-dataset pipeline.")
    p.add_argument("--self-test", action="store_true")
    # ingest
    p.add_argument("--coco", type=Path)
    p.add_argument("--images-dir", type=Path)
    p.add_argument("--dataset-root", type=Path,
                   help="Roboflow-style export root (alternative to --coco/--images-dir; ingests all splits)")
    p.add_argument("--workspace-root", type=Path)
    p.add_argument("--name")
    p.add_argument("--workspace-id")
    # embeddings
    p.add_argument("--scope", default="object", choices=["object", "image"])
    p.add_argument("--model", default="ViT-B-32")
    p.add_argument("--pretrained", default="laion2b_s34b_b79k")
    p.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"])
    p.add_argument("--batch-size", type=int, default=64)
    p.add_argument("--limit", type=int)
    p.add_argument("--mock", action="store_true")
    p.add_argument("--mock-dim", type=int, default=64)
    # sampling
    p.add_argument("--sample-name", default="pipeline")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--target-images", type=int)
    g.add_argument("--target-ratio", type=float)
    p.add_argument("--mode", default="balanced", choices=["balanced", "diverse"],
                   help="balanced = representative coverage; diverse = max spread")
    p.add_argument("--remove-outliers", action="store_true")
    p.add_argument("--outlier-method", default="knn", choices=["knn", "lof"])
    p.add_argument("--outlier-pct", type=float, default=0.02)
    p.add_argument("--per-class-floor", type=int, default=5)
    p.add_argument("--pca-dim", type=int, default=50)
    p.add_argument("--seed", type=int, default=42)
    # export
    p.add_argument("--export-coco", type=Path, help="output path for the sampled COCO subset")
    p.add_argument("--no-export", action="store_true")
    # misc
    p.add_argument("--python", help="python interpreter for sub-steps (default: this one)")
    return p.parse_args(argv)


def validate(args):
    if args.workspace_root is None:
        fail("--workspace-root is required")
    if args.dataset_root is not None:
        if not args.dataset_root.exists():
            fail(f"dataset root not found: {args.dataset_root}")
    else:
        if args.coco is None or args.images_dir is None:
            fail("provide --dataset-root, or both --coco and --images-dir")
        if not args.coco.exists():
            fail(f"coco json not found: {args.coco}")
    if args.target_images is None and args.target_ratio is None:
        args.target_ratio = 0.3
        log("no target given; defaulting to --target-ratio 0.3")


def self_test():
    log("self-test: building synthetic COCO + running full pipeline with --mock")
    tmp = Path(tempfile.mkdtemp())
    coco = {
        "images": [{"id": i, "file_name": f"{i}.jpg", "width": 640, "height": 480} for i in range(10)],
        "categories": [{"id": 1, "name": "cat"}, {"id": 2, "name": "dog"}],
        "annotations": [
            {"id": 100 + k, "image_id": k % 10, "category_id": (k % 2) + 1, "bbox": [10, 10, 50, 50]}
            for k in range(40)
        ],
    }
    coco_path = tmp / "instances.json"
    coco_path.write_text(json.dumps(coco))
    (tmp / "imgs").mkdir()

    base = dict(
        coco=coco_path, images_dir=tmp / "imgs", dataset_root=None, workspace_root=tmp / "ws",
        name="Self Test", workspace_id=None,
        scope="object", model="ViT-B-32", pretrained="x", device="auto", batch_size=64,
        limit=None, mock=True, mock_dim=32,
        sample_name="pipeline", target_images=4, target_ratio=None,
        mode="balanced", remove_outliers=True, outlier_method="knn",
        outlier_pct=0.05, per_class_floor=3, pca_dim=50, seed=42,
        export_coco=None, no_export=False, python=None,
    )
    summary = run_pipeline(argparse.Namespace(**base))
    log("self-test summary: " + json.dumps(summary["sample"]))
    assert summary["ingest"]["images"] == 10, summary["ingest"]
    assert summary["embed"]["embedded"] == 40, summary["embed"]
    assert summary["sample"]["selected_images"] == 4, summary["sample"]
    assert summary["export"]["images"] == 4, summary["export"]
    assert summary["export"]["annotations"] > 0, summary["export"]
    assert Path(summary["export"]["path"]).exists()

    # Roboflow-style multi-split root, end to end
    log("self-test: building synthetic Roboflow root + running pipeline on --dataset-root")
    root = Path(tempfile.mkdtemp())
    for split in ("train", "valid"):
        d = root / split
        d.mkdir(parents=True)
        scoco = {
            "images": [{"id": i, "file_name": f"{split}_{i}.jpg", "width": 100, "height": 100} for i in range(5)],
            "categories": [{"id": 1, "name": "ball"}],
            "annotations": [{"id": i, "image_id": i, "category_id": 1, "bbox": [1, 1, 9, 9]} for i in range(5)],
        }
        (d / "_annotations.coco.json").write_text(json.dumps(scoco))
    ms = run_pipeline(argparse.Namespace(**{
        **base, "coco": None, "images_dir": None, "dataset_root": root,
        "workspace_root": root / "ws", "name": "Multi", "target_images": 4, "target_ratio": None,
    }))
    log("multi-split summary: " + json.dumps({"ingest": ms["ingest"], "export": ms["export"]}))
    assert ms["ingest"]["images"] == 10, ms["ingest"]                 # 5 + 5 across splits
    assert ms["embed"]["embedded"] == 10, ms["embed"]
    assert ms["sample"]["selected_images"] == 4, ms["sample"]
    assert ms["export"]["images"] == 4 and ms["export"]["annotations"] > 0, ms["export"]
    assert Path(ms["export"]["path"]).exists()
    log("self-test: ALL PASSED")


def main():
    args = parse_args()
    if args.self_test:
        self_test()
        return
    validate(args)
    summary = run_pipeline(args)
    print(json.dumps(summary))


if __name__ == "__main__":
    main()

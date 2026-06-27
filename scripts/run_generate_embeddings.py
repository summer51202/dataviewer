"""
Generate embedding vectors for a DataViewer workspace and write them into the
`embeddings` table, so downstream tools (run_umap_projection.py,
run_sample_selection.py) and the export pipeline can use them.

This is the CLI/agent embedding producer. It is independent from the GUI's
Rust/ONNX path: vectors produced here form their own self-consistent track and
should be kept under their own --model-id (do not mix producers under one id).

Encoders:
    - Real:  open_clip image encoder (default ViT-B-32 / laion2b). Object scope
             encodes the bbox crop; image scope encodes the whole image.
    - Mock:  --mock writes deterministic pseudo-random vectors derived from the
             target id. Needs only numpy (no torch, no image files) and is meant
             for smoke-testing the full ingest -> embed -> sample -> export chain.

Usage:
    python run_generate_embeddings.py \
        --db-path <workspace>/.dataviewer/workspace.db \
        --workspace-id <id> --scope object \
        [--model ViT-B-32] [--pretrained laion2b_s34b_b79k] \
        [--batch-size 64] [--device auto] [--limit N]

    python run_generate_embeddings.py ... --mock [--mock-dim 64]
    python run_generate_embeddings.py --self-test

Exit codes: 0 success; 1 error (stderr).
Stdout on success: single-line JSON, e.g.
    {"embedded": 530, "scope": "object", "model_id": "clip-vit-b-32", "dim": 512,
     "skipped": 4, "backend": "cpu"}
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sqlite3
import struct
import sys
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path

EMBEDDINGS_DDL = """
CREATE TABLE IF NOT EXISTS embeddings (
    id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, scope TEXT NOT NULL,
    target_id TEXT NOT NULL, image_id TEXT NOT NULL, annotation_id TEXT,
    model_id TEXT NOT NULL, runtime_backend TEXT NOT NULL, vector BLOB NOT NULL,
    vector_norm REAL, created_at TEXT NOT NULL,
    UNIQUE(workspace_id, scope, target_id, model_id)
);
CREATE INDEX IF NOT EXISTS idx_embeddings_workspace_scope_model
    ON embeddings(workspace_id, scope, model_id);
"""


def log(msg: str) -> None:
    print(f"[embed] {msg}", file=sys.stderr, flush=True)


def fail(msg: str, code: int = 1):
    print(f"[embed] ERROR: {msg}", file=sys.stderr, flush=True)
    sys.exit(code)


# --------------------------------------------------------------------------- #
# Work items
# --------------------------------------------------------------------------- #
def read_items(db_path: Path, workspace_id: str, scope: str):
    """Return list of work items dicts: target_id, image_id, annotation_id, path, bbox."""
    con = sqlite3.connect(str(db_path))
    try:
        if scope == "image":
            rows = con.execute(
                "SELECT id, original_path FROM images WHERE workspace_id = ? ORDER BY id",
                (workspace_id,),
            ).fetchall()
            return [
                {"target_id": iid, "image_id": iid, "annotation_id": None, "path": path, "bbox": None}
                for iid, path in rows
            ]
        # object scope: one item per annotation with a usable bbox
        rows = con.execute(
            """
            SELECT a.id, a.image_id, i.original_path,
                   a.bbox_x, a.bbox_y, a.bbox_width, a.bbox_height
            FROM annotations a
            JOIN images i ON i.id = a.image_id
            WHERE a.workspace_id = ?
            ORDER BY a.id
            """,
            (workspace_id,),
        ).fetchall()
        items = []
        for aid, image_id, path, bx, by, bw, bh in rows:
            items.append({
                "target_id": aid, "image_id": image_id, "annotation_id": aid,
                "path": path, "bbox": (bx, by, bw, bh),
            })
        return items
    finally:
        con.close()


# --------------------------------------------------------------------------- #
# Encoders
# --------------------------------------------------------------------------- #
def mock_vectors(items, dim: int):
    import numpy as np

    out = []
    for it in items:
        seed = int.from_bytes(hashlib.sha256(str(it["target_id"]).encode()).digest()[:8], "little")
        rng = np.random.default_rng(seed)
        out.append((it, rng.standard_normal(dim).astype(np.float32)))
    return out, dim, "mock"


def real_vectors(items, model_name, pretrained, batch_size, device, skip_counter):
    import numpy as np
    import open_clip
    import torch
    from PIL import Image

    if device == "auto":
        device = "cuda" if torch.cuda.is_available() else "cpu"
    log(f"loading open_clip {model_name} / {pretrained} on {device}")
    model, _, preprocess = open_clip.create_model_and_transforms(model_name, pretrained=pretrained)
    model = model.to(device).eval()

    def crop(it):
        img = Image.open(it["path"]).convert("RGB")
        if it["bbox"] is not None and all(v is not None for v in it["bbox"]):
            x, y, w, h = it["bbox"]
            if w and h and w > 0 and h > 0:
                left, top = max(0, int(x)), max(0, int(y))
                right, bottom = min(img.width, int(x + w)), min(img.height, int(y + h))
                if right > left and bottom > top:
                    img = img.crop((left, top, right, bottom))
        return preprocess(img)

    results = []
    dim = None
    batch, batch_items = [], []

    def flush():
        nonlocal dim
        if not batch:
            return
        with torch.no_grad():
            t = torch.stack(batch).to(device)
            feats = model.encode_image(t).float().cpu().numpy()
        if dim is None:
            dim = feats.shape[1]
        for bi, vec in zip(batch_items, feats):
            results.append((bi, vec.astype(np.float32)))
        batch.clear()
        batch_items.clear()

    for it in items:
        try:
            batch.append(crop(it))
            batch_items.append(it)
        except Exception as e:  # noqa: BLE001
            skip_counter[0] += 1
            log(f"skip {it['target_id']}: {e}")
            continue
        if len(batch) >= batch_size:
            flush()
    flush()
    return results, (dim or 0), device


# --------------------------------------------------------------------------- #
# DB write
# --------------------------------------------------------------------------- #
def write_embeddings(db_path: Path, workspace_id: str, scope: str, model_id: str,
                     backend: str, encoded) -> int:
    import numpy as np

    now = datetime.now(timezone.utc).isoformat()
    rows = []
    for it, vec in encoded:
        vec = np.asarray(vec, dtype=np.float32)
        blob = struct.pack(f"<{vec.shape[0]}f", *vec.tolist())
        norm = float(np.linalg.norm(vec))
        rows.append((
            str(uuid.uuid4()), workspace_id, scope, it["target_id"], it["image_id"],
            it["annotation_id"], model_id, backend, blob, norm, now,
        ))
    con = sqlite3.connect(str(db_path))
    try:
        con.executescript(EMBEDDINGS_DDL)
        con.executemany(
            """
            INSERT INTO embeddings
                (id, workspace_id, scope, target_id, image_id, annotation_id,
                 model_id, runtime_backend, vector, vector_norm, created_at)
            VALUES (?,?,?,?,?,?,?,?,?,?,?)
            ON CONFLICT(workspace_id, scope, target_id, model_id) DO UPDATE SET
                vector = excluded.vector, vector_norm = excluded.vector_norm,
                runtime_backend = excluded.runtime_backend, image_id = excluded.image_id,
                annotation_id = excluded.annotation_id, created_at = excluded.created_at
            """,
            rows,
        )
        con.commit()
    finally:
        con.close()
    return len(rows)


# --------------------------------------------------------------------------- #
# Orchestration
# --------------------------------------------------------------------------- #
def run(args) -> dict:
    log(f"reading work items: workspace={args.workspace_id} scope={args.scope}")
    items = read_items(args.db_path, args.workspace_id, args.scope)
    if args.limit:
        items = items[: args.limit]
    if not items:
        log("no work items found")
        return {"embedded": 0, "scope": args.scope, "model_id": args.model_id or "", "dim": 0,
                "skipped": 0, "backend": "none"}

    skip_counter = [0]
    if args.mock:
        encoded, dim, backend = mock_vectors(items, args.mock_dim)
        model_id = args.model_id or f"mock-{dim}"
    else:
        encoded, dim, backend = real_vectors(
            items, args.model, args.pretrained, args.batch_size, args.device, skip_counter
        )
        model_id = args.model_id or ("clip-" + args.model.lower().replace("/", "-"))

    log(f"encoded {len(encoded)} item(s), dim={dim}, backend={backend}")
    if args.dry_run:
        log("dry-run: not writing to DB")
    else:
        write_embeddings(args.db_path, args.workspace_id, args.scope, model_id, backend, encoded)

    return {
        "embedded": len(encoded), "scope": args.scope, "model_id": model_id,
        "dim": dim, "skipped": skip_counter[0], "backend": backend,
    }


def parse_args(argv=None):
    p = argparse.ArgumentParser(description="Generate embeddings for a DataViewer workspace.")
    p.add_argument("--self-test", action="store_true")
    p.add_argument("--db-path", type=Path)
    p.add_argument("--workspace-id")
    p.add_argument("--scope", default="object", choices=["object", "image"])
    p.add_argument("--model-id", help="override the stored model_id (default derived from --model / --mock)")
    p.add_argument("--model", default="ViT-B-32")
    p.add_argument("--pretrained", default="laion2b_s34b_b79k")
    p.add_argument("--batch-size", type=int, default=64)
    p.add_argument("--device", default="auto", choices=["auto", "cpu", "cuda"])
    p.add_argument("--limit", type=int)
    p.add_argument("--mock", action="store_true", help="write deterministic pseudo-vectors (no torch/images)")
    p.add_argument("--mock-dim", type=int, default=64)
    p.add_argument("--dry-run", action="store_true")
    return p.parse_args(argv)


def validate(args):
    if args.db_path is None or args.workspace_id is None:
        fail("--db-path and --workspace-id are required")
    if not args.db_path.exists():
        fail(f"db path does not exist: {args.db_path}")


def self_test():
    import numpy as np

    log("self-test: building synthetic workspace DB")
    tmp = Path(tempfile.mkdtemp()) / "ws.db"
    con = sqlite3.connect(str(tmp))
    con.executescript(
        """
        CREATE TABLE images (id TEXT PRIMARY KEY, workspace_id TEXT, original_path TEXT);
        CREATE TABLE annotations (id TEXT PRIMARY KEY, workspace_id TEXT, image_id TEXT,
            bbox_x REAL, bbox_y REAL, bbox_width REAL, bbox_height REAL);
        """
    )
    imgs = [(f"img-{i}", "ws", f"/nonexistent/{i}.jpg") for i in range(6)]
    anns = [(f"ann-{k}", "ws", f"img-{k % 6}", 0, 0, 10, 10) for k in range(20)]
    con.executemany("INSERT INTO images VALUES (?,?,?)", imgs)
    con.executemany("INSERT INTO annotations VALUES (?,?,?,?,?,?,?)", anns)
    con.commit()
    con.close()

    base = dict(db_path=tmp, workspace_id="ws", model=None, pretrained=None, batch_size=64,
                device="auto", limit=None, mock=True, mock_dim=32, dry_run=False, model_id=None)
    a_obj = argparse.Namespace(scope="object", **base)
    s1 = run(a_obj)
    log("object: " + json.dumps(s1))
    assert s1["embedded"] == 20 and s1["dim"] == 32, s1

    # idempotency: re-run, count of rows unchanged (upsert)
    run(argparse.Namespace(scope="object", **base))
    con = sqlite3.connect(str(tmp))
    try:
        n = con.execute("SELECT COUNT(*) FROM embeddings WHERE scope='object'").fetchone()[0]
        # determinism: vectors identical across runs
        v = con.execute("SELECT vector FROM embeddings WHERE target_id='ann-0' AND scope='object'").fetchone()[0]
    finally:
        con.close()
    assert n == 20, n
    # recompute expected mock vector for ann-0
    seed = int.from_bytes(hashlib.sha256(b"ann-0").digest()[:8], "little")
    exp = np.random.default_rng(seed).standard_normal(32).astype(np.float32)
    got = np.array(struct.unpack_from("<32f", v), dtype=np.float32)
    assert np.allclose(exp, got), "mock vectors not deterministic"
    log("self-test: idempotency + determinism OK")

    a_img = argparse.Namespace(scope="image", **base)
    s2 = run(a_img)
    log("image: " + json.dumps(s2))
    assert s2["embedded"] == 6, s2
    log("self-test: ALL PASSED")


def main():
    args = parse_args()
    if args.self_test:
        self_test()
        return
    validate(args)
    summary = run(args)
    print(json.dumps(summary))


if __name__ == "__main__":
    main()

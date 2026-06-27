"""
Auto sample-space selection from stored embedding vectors.

Selects a coverage-maximising, distribution-flattening subset of objects and
records the resulting image set as a named, non-destructive "sample set" in the
workspace database. Intended for both GUI-triggered runs and direct CLI/agent
use. Original images/annotations/embeddings are never modified.

Design summary (see docs/spec-auto-sample-selection.md):
    - Coverage is computed on the high-dimensional OBJECT embeddings (cosine),
      NOT on the 2D UMAP/PCA projection.
    - Export unit is the IMAGE: an image is included if any of its objects is
      selected. The greedy driver maximises object-space coverage; the stop
      condition is a target image count / ratio.
    - facility-location (CELF lazy greedy) is the primary method; farthest-point
      sampling (FPS) is the lightweight / large-N fallback.
    - Optional, non-destructive outlier exclusion with a per-class floor.

Usage:
    python run_sample_selection.py \
        --db-path <path/to/workspace.db> \
        --workspace-id <id> --scope object --model-id <model-id> \
        --name "coverage-30pct" \
        (--target-images 500 | --target-ratio 0.3) \
        [--mode balanced|diverse] \
        [--remove-outliers] [--outlier-method knn|lof] [--outlier-pct 0.02] \
        [--per-class-floor 5] [--pca-dim 50] [--seed 42] [--dry-run]

    python run_sample_selection.py --self-test   # offline sanity check, no DB

Exit codes:
    0  success
    1  error (details on stderr)

Stdout on success: single-line JSON summary, e.g.
    {"sample_set": "coverage-30pct", "mode": "balanced",
     "selected_images": 500, "selected_objects": 1320,
     "excluded_outliers": 47, "saturated": false, "seed": 42,
     "total_images": 1666}
"""
from __future__ import annotations

import argparse
import json
import sqlite3
import struct
import sys
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path

SCRIPT_VERSION = "sample-v1"


# --------------------------------------------------------------------------- #
# Logging helpers
# --------------------------------------------------------------------------- #
def log(msg: str) -> None:
    print(f"[sample] {msg}", file=sys.stderr, flush=True)


def fail(msg: str, code: int = 1) -> "None":
    print(f"[sample] ERROR: {msg}", file=sys.stderr, flush=True)
    sys.exit(code)


# --------------------------------------------------------------------------- #
# DB I/O
# --------------------------------------------------------------------------- #
def read_objects(db_path: Path, workspace_id: str, scope: str, model_id: str):
    """Return parallel lists: target_ids, image_ids, annotation_ids, categories, vectors."""
    con = sqlite3.connect(str(db_path))
    try:
        cur = con.execute(
            """
            SELECT e.target_id, e.image_id, e.annotation_id, a.category_id, e.vector
            FROM embeddings e
            LEFT JOIN annotations a ON a.id = e.annotation_id
            WHERE e.workspace_id = ? AND e.scope = ? AND e.model_id = ?
            ORDER BY e.target_id
            """,
            (workspace_id, scope, model_id),
        )
        rows = cur.fetchall()
    finally:
        con.close()

    target_ids, image_ids, annotation_ids, categories, vectors = [], [], [], [], []
    for target_id, image_id, annotation_id, category_id, blob in rows:
        n = len(blob) // 4
        vectors.append(struct.unpack_from(f"<{n}f", blob))
        target_ids.append(target_id)
        image_ids.append(image_id)
        annotation_ids.append(annotation_id)
        categories.append(category_id)
    return target_ids, image_ids, annotation_ids, categories, vectors


def ensure_tables(con: sqlite3.Connection) -> None:
    con.executescript(
        """
        CREATE TABLE IF NOT EXISTS sample_sets (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            name TEXT NOT NULL,
            scope TEXT NOT NULL,
            model_id TEXT NOT NULL,
            coverage_method TEXT NOT NULL,
            params_json TEXT NOT NULL,
            target_images INTEGER,
            target_ratio REAL,
            selected_images INTEGER NOT NULL,
            selected_objects INTEGER NOT NULL,
            excluded_outliers INTEGER NOT NULL,
            saturated INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            UNIQUE(workspace_id, name)
        );
        CREATE TABLE IF NOT EXISTS sample_set_members (
            id TEXT PRIMARY KEY,
            sample_set_id TEXT NOT NULL,
            image_id TEXT NOT NULL,
            membership TEXT NOT NULL,
            trigger_object_id TEXT,
            created_at TEXT NOT NULL,
            UNIQUE(sample_set_id, image_id)
        );
        CREATE INDEX IF NOT EXISTS idx_sample_set_members_set
            ON sample_set_members(sample_set_id);
        """
    )


def write_sample_set(
    db_path: Path,
    workspace_id: str,
    name: str,
    scope: str,
    model_id: str,
    coverage_method: str,
    params: dict,
    target_images,
    target_ratio,
    selected_members: "list[tuple[str, str]]",  # (image_id, trigger_object_id)
    outlier_excluded_images: "list[str]",
    selected_objects: int,
    excluded_outliers: int,
    saturated: bool,
    no_overwrite: bool,
) -> None:
    now = datetime.now(timezone.utc).isoformat()
    con = sqlite3.connect(str(db_path))
    try:
        ensure_tables(con)
        existing = con.execute(
            "SELECT id FROM sample_sets WHERE workspace_id = ? AND name = ?",
            (workspace_id, name),
        ).fetchone()
        if existing and no_overwrite:
            fail(f"sample set '{name}' already exists (use a different --name or drop --no-overwrite)")
        if existing:
            old_id = existing[0]
            con.execute("DELETE FROM sample_set_members WHERE sample_set_id = ?", (old_id,))
            con.execute("DELETE FROM sample_sets WHERE id = ?", (old_id,))

        set_id = str(uuid.uuid4())
        con.execute(
            """
            INSERT INTO sample_sets
                (id, workspace_id, name, scope, model_id, coverage_method, params_json,
                 target_images, target_ratio, selected_images, selected_objects,
                 excluded_outliers, saturated, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                set_id, workspace_id, name, scope, model_id, coverage_method,
                json.dumps(params, sort_keys=True),
                target_images, target_ratio,
                len(selected_members), selected_objects, excluded_outliers,
                1 if saturated else 0, now,
            ),
        )
        member_rows = [
            (str(uuid.uuid4()), set_id, img, "selected", trig, now)
            for img, trig in selected_members
        ]
        member_rows += [
            (str(uuid.uuid4()), set_id, img, "outlier-excluded", None, now)
            for img in outlier_excluded_images
        ]
        con.executemany(
            """
            INSERT INTO sample_set_members
                (id, sample_set_id, image_id, membership, trigger_object_id, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            """,
            member_rows,
        )
        con.commit()
    finally:
        con.close()


# --------------------------------------------------------------------------- #
# Numeric pipeline
# --------------------------------------------------------------------------- #
def preprocess(vectors, pca_dim: int, seed: int):
    import numpy as np

    X = np.asarray(vectors, dtype=np.float32)
    # L2 normalise so dot product == cosine similarity
    norms = np.linalg.norm(X, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    X = X / norms

    n, d = X.shape
    if pca_dim and pca_dim > 0 and d > pca_dim and n > pca_dim:
        from sklearn.decomposition import PCA

        X = PCA(n_components=pca_dim, random_state=seed).fit_transform(X).astype(np.float32)
        # renormalise after PCA so cosine geometry is preserved downstream
        norms = np.linalg.norm(X, axis=1, keepdims=True)
        norms[norms == 0] = 1.0
        X = X / norms
        log(f"PCA reduced to {pca_dim} dims")
    else:
        if pca_dim and d <= pca_dim:
            log(f"skipping PCA: dim {d} <= pca_dim {pca_dim}")
    return X


def detect_outliers(X, categories, method: str, pct: float, per_class_floor: int, seed: int):
    import numpy as np

    n = len(X)
    is_out = np.zeros(n, dtype=bool)
    if pct <= 0 or n < 3:
        return is_out, 0

    k = min(15, n - 1)
    if method == "lof":
        from sklearn.neighbors import LocalOutlierFactor

        lof = LocalOutlierFactor(n_neighbors=k, metric="cosine")
        lof.fit_predict(X)
        score = -lof.negative_outlier_factor_  # higher == more outlying
    else:  # knn mean distance
        from sklearn.neighbors import NearestNeighbors

        nn = NearestNeighbors(n_neighbors=k + 1, metric="cosine").fit(X)
        dist, _ = nn.kneighbors(X)
        score = dist[:, 1:].mean(axis=1)  # exclude self (col 0)

    n_out = int(round(n * pct))
    if n_out <= 0:
        return is_out, 0
    worst = np.argsort(score)[::-1][:n_out]
    is_out[worst] = True

    # class-aware floor: every category keeps >= per_class_floor non-outlier objects
    cats = np.array([c if c is not None else "__none__" for c in categories], dtype=object)
    for c in np.unique(cats):
        idx = np.where(cats == c)[0]
        kept = idx[~is_out[idx]]
        if len(kept) >= per_class_floor:
            continue
        need = per_class_floor - len(kept)
        out_idx = idx[is_out[idx]]
        # un-mark the least-outlying first (lowest score)
        restore = out_idx[np.argsort(score[out_idx])][:need]
        is_out[restore] = False

    return is_out, int(is_out.sum())


def fps_order(X, image_ids, target_images):
    """Farthest-point sampling; yields object indices in selection order."""
    import numpy as np

    n = len(X)
    centroid = X.mean(axis=0)
    # deterministic start: farthest from centroid (largest cosine distance)
    first = int(np.argmax(1.0 - (X @ centroid)))

    selected_members = []
    selected_objs = []
    seen_images = set()

    def take(i):
        selected_objs.append(i)
        img = image_ids[i]
        if img not in seen_images:
            seen_images.add(img)
            selected_members.append((img, i))

    take(first)
    min_dist = 1.0 - (X @ X[first])
    min_dist[first] = -np.inf
    while len(seen_images) < target_images and len(selected_objs) < n:
        nxt = int(np.argmax(min_dist))
        if not np.isfinite(min_dist[nxt]):
            break  # everything already selected
        take(nxt)
        min_dist = np.minimum(min_dist, 1.0 - (X @ X[nxt]))
        min_dist[nxt] = -np.inf
    return selected_members, selected_objs


def facility_location_order(X, image_ids, target_images, fl_max_n):
    """CELF lazy-greedy facility-location. Returns None to signal FPS fallback."""
    import heapq

    import numpy as np

    n = len(X)
    if n > fl_max_n:
        return None

    S = (X @ X.T).astype(np.float32)  # cosine similarity matrix
    rowsum = S.sum(axis=1)
    first = int(np.argmax(rowsum))  # medoid seed (deterministic)

    best = S[first].copy()
    selected_members = []
    selected_objs = []
    seen_images = set()
    chosen = {first}

    def take(i):
        selected_objs.append(i)
        img = image_ids[i]
        if img not in seen_images:
            seen_images.add(img)
            selected_members.append((img, i))

    take(first)

    # CELF priority queue, tagged by selection count when gain was computed
    it = 1
    gains = np.maximum(0.0, S - best).sum(axis=1)
    heap = [(-gains[j], j, it) for j in range(n) if j != first]
    heapq.heapify(heap)

    while heap and len(seen_images) < target_images and len(chosen) < n:
        neg_g, j, tag = heapq.heappop(heap)
        if j in chosen:
            continue
        if tag == it:
            chosen.add(j)
            take(j)
            best = np.maximum(best, S[j])
            it += 1
        else:
            g = float(np.maximum(0.0, S[j] - best).sum())
            heapq.heappush(heap, (-g, j, it))
    return selected_members, selected_objs


# --------------------------------------------------------------------------- #
# Orchestration
# --------------------------------------------------------------------------- #
def run(args) -> dict:
    log(f"reading embeddings: workspace={args.workspace_id} scope={args.scope} model={args.model_id}")
    target_ids, image_ids, annotation_ids, categories, vectors = read_objects(
        args.db_path, args.workspace_id, args.scope, args.model_id
    )

    total_images = len({img for img in image_ids})
    if not vectors:
        log("no embedding vectors found; nothing to select")
        summary = {
            "sample_set": args.name, "mode": args.mode,
            "selected_images": 0, "selected_objects": 0, "excluded_outliers": 0,
            "saturated": False, "seed": args.seed, "total_images": 0,
        }
        if not args.dry_run:
            write_sample_set(
                args.db_path, args.workspace_id, args.name, args.scope, args.model_id,
                args.mode, _params(args), args.target_images, args.target_ratio,
                [], [], 0, 0, False, args.no_overwrite,
            )
        return summary

    # resolve target image count
    if args.target_images is not None:
        target_images = args.target_images
    else:
        target_images = max(1, round(total_images * args.target_ratio))
        log(f"target_ratio {args.target_ratio} -> {target_images} of {total_images} images")

    log("preprocessing")
    X = preprocess(vectors, args.pca_dim, args.seed)

    # outlier exclusion (non-destructive)
    excluded_outliers = 0
    keep_mask = None
    if args.remove_outliers:
        log(f"detecting outliers (method={args.outlier_method}, pct={args.outlier_pct})")
        import numpy as np

        is_out, excluded_outliers = detect_outliers(
            X, categories, args.outlier_method, args.outlier_pct, args.per_class_floor, args.seed
        )
        keep_mask = ~is_out
        if excluded_outliers:
            log(f"excluded {excluded_outliers} outlier object(s)")

    import numpy as np

    if keep_mask is not None:
        cand_idx = np.where(keep_mask)[0]
    else:
        cand_idx = np.arange(len(X))
    X_cand = X[cand_idx]
    img_cand = [image_ids[i] for i in cand_idx]

    cand_distinct_images = len(set(img_cand))
    saturated = target_images >= cand_distinct_images
    effective_target = min(target_images, cand_distinct_images)

    mode = args.mode
    log(f"running {mode} on {len(X_cand)} candidate objects (target {effective_target} images)")
    result = None
    if mode == "balanced":
        result = facility_location_order(X_cand, img_cand, effective_target, args.fl_max_n)
        if result is None:
            log(f"candidate count {len(X_cand)} > fl-max-n {args.fl_max_n}; falling back to diverse")
            mode = "diverse"
    if result is None:
        result = fps_order(X_cand, img_cand, effective_target)

    local_members, local_objs = result
    # map candidate-local indices back to original index space
    selected_members = [(img, target_ids[cand_idx[i]]) for img, i in local_members]
    selected_objects = len(local_objs)

    # audit: images whose objects are ALL outliers and which were not selected
    outlier_excluded_images = []
    if keep_mask is not None:
        selected_image_set = {img for img, _ in selected_members}
        from collections import defaultdict

        per_image_keep = defaultdict(lambda: [0, 0])  # image -> [kept, total]
        for i, img in enumerate(image_ids):
            per_image_keep[img][1] += 1
            if keep_mask[i]:
                per_image_keep[img][0] += 1
        outlier_excluded_images = [
            img for img, (kept, _tot) in per_image_keep.items()
            if kept == 0 and img not in selected_image_set
        ]

    summary = {
        "sample_set": args.name,
        "mode": mode,
        "selected_images": len(selected_members),
        "selected_objects": selected_objects,
        "excluded_outliers": excluded_outliers,
        "saturated": bool(saturated),
        "seed": args.seed,
        "total_images": total_images,
    }

    if args.dry_run:
        log("dry-run: not writing to DB")
        return summary

    log(f"writing sample set '{args.name}' ({len(selected_members)} images)")
    write_sample_set(
        args.db_path, args.workspace_id, args.name, args.scope, args.model_id,
        mode, _params(args), args.target_images, args.target_ratio,
        selected_members, outlier_excluded_images,
        selected_objects, excluded_outliers, saturated, args.no_overwrite,
    )
    return summary


def _params(args) -> dict:
    return {
        "version": SCRIPT_VERSION,
        "scope": args.scope,
        "model_id": args.model_id,
        "mode": args.mode,
        "target_images": args.target_images,
        "target_ratio": args.target_ratio,
        "remove_outliers": args.remove_outliers,
        "outlier_method": args.outlier_method,
        "outlier_pct": args.outlier_pct,
        "per_class_floor": args.per_class_floor,
        "pca_dim": args.pca_dim,
        "fl_max_n": args.fl_max_n,
        "seed": args.seed,
    }


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #
def parse_args(argv=None) -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Auto sample-space selection from embeddings.")
    p.add_argument("--self-test", action="store_true", help="run an offline synthetic sanity check and exit")
    p.add_argument("--db-path", type=Path)
    p.add_argument("--workspace-id")
    p.add_argument("--scope", default="object", choices=["object", "image"])
    p.add_argument("--model-id")
    p.add_argument("--name")
    g = p.add_mutually_exclusive_group()
    g.add_argument("--target-images", type=int)
    g.add_argument("--target-ratio", type=float)
    p.add_argument("--mode", default="balanced", choices=["balanced", "diverse"],
                   help="balanced = representative coverage (facility-location); diverse = max spread (farthest-point)")
    p.add_argument("--remove-outliers", action="store_true")
    p.add_argument("--outlier-method", default="knn", choices=["knn", "lof"])
    p.add_argument("--outlier-pct", type=float, default=0.02)
    p.add_argument("--per-class-floor", type=int, default=5)
    p.add_argument("--pca-dim", type=int, default=50)
    p.add_argument("--fl-max-n", type=int, default=6000, help="max candidate objects for balanced mode before falling back to diverse")
    p.add_argument("--seed", type=int, default=42)
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--no-overwrite", action="store_true")
    return p.parse_args(argv)


def validate(args) -> None:
    missing = [n for n in ("db_path", "workspace_id", "model_id", "name") if getattr(args, n) is None]
    if missing:
        fail("missing required args: " + ", ".join("--" + m.replace("_", "-") for m in missing))
    if args.target_images is None and args.target_ratio is None:
        fail("one of --target-images or --target-ratio is required")
    if args.target_ratio is not None and not (0.0 < args.target_ratio <= 1.0):
        fail("--target-ratio must be in (0, 1]")
    if args.target_images is not None and args.target_images < 1:
        fail("--target-images must be >= 1")
    if not args.db_path.exists():
        fail(f"db path does not exist: {args.db_path}")


def self_test() -> None:
    """Build a synthetic clustered DB and exercise the full pipeline (no real data)."""
    import numpy as np

    log("self-test: building synthetic workspace")
    rng = np.random.default_rng(0)
    tmp = Path(tempfile.mkdtemp()) / "selftest.db"
    con = sqlite3.connect(str(tmp))
    con.executescript(
        """
        CREATE TABLE embeddings (id TEXT, workspace_id TEXT, scope TEXT, target_id TEXT,
            image_id TEXT, annotation_id TEXT, model_id TEXT, vector BLOB);
        CREATE TABLE annotations (id TEXT PRIMARY KEY, category_id TEXT);
        """
    )
    dim = 32
    centers = rng.normal(size=(4, dim)).astype(np.float32)
    n_per = 60
    oid = 0
    rows_e, rows_a = [], []
    for c in range(4):
        for _ in range(n_per):
            v = centers[c] + 0.15 * rng.normal(size=dim).astype(np.float32)
            blob = struct.pack(f"<{dim}f", *v.tolist())
            tid = f"obj-{oid}"
            img = f"img-{oid // 2}"  # 2 objects per image
            ann = f"ann-{oid}"
            rows_e.append((f"e{oid}", "ws", "object", tid, img, ann, "clip", blob))
            rows_a.append((ann, f"cat-{c}"))
            oid += 1
    con.executemany("INSERT INTO embeddings VALUES (?,?,?,?,?,?,?,?)", rows_e)
    con.executemany("INSERT INTO annotations VALUES (?,?)", rows_a)
    con.commit()
    con.close()

    base = dict(
        db_path=tmp, workspace_id="ws", scope="object", model_id="clip",
        mode="balanced", target_images=40, target_ratio=None,
        remove_outliers=True, outlier_method="knn", outlier_pct=0.02,
        per_class_floor=5, pca_dim=16, fl_max_n=6000, seed=42,
        dry_run=False, no_overwrite=False, name="selftest",
    )
    a1 = argparse.Namespace(**base)
    s1 = run(a1)
    log("self-test run #1: " + json.dumps(s1))
    assert s1["selected_images"] == 40, s1
    assert 0 < s1["selected_objects"], s1

    # idempotency: re-run with same params -> identical membership
    def members(name):
        con = sqlite3.connect(str(tmp))
        try:
            sid = con.execute("SELECT id FROM sample_sets WHERE name=?", (name,)).fetchone()[0]
            rows = con.execute(
                "SELECT image_id, membership FROM sample_set_members WHERE sample_set_id=? ORDER BY image_id",
                (sid,),
            ).fetchall()
        finally:
            con.close()
        return rows

    m1 = members("selftest")
    run(argparse.Namespace(**base))
    m2 = members("selftest")
    assert m1 == m2, "non-deterministic membership across identical runs"
    log("self-test: idempotency OK")

    # diverse (farthest-point) path
    a3 = argparse.Namespace(**{**base, "mode": "diverse", "name": "selftest-fps"})
    s3 = run(a3)
    assert s3["selected_images"] == 40, s3
    log("self-test fps: " + json.dumps(s3))

    # ratio + saturated
    a4 = argparse.Namespace(**{**base, "target_images": None, "target_ratio": 1.0, "name": "selftest-sat"})
    s4 = run(a4)
    assert s4["saturated"] is True, s4
    log("self-test saturated: " + json.dumps(s4))

    log("self-test: ALL PASSED")


def main() -> None:
    args = parse_args()
    if args.self_test:
        self_test()
        return
    validate(args)
    summary = run(args)
    print(json.dumps(summary))


if __name__ == "__main__":
    main()

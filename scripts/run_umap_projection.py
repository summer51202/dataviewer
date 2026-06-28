"""
Generate UMAP 2D projections from stored embedding vectors and write them back
to the workspace database.

Usage:
    python run_umap_projection.py \
        --db-path <path/to/workspace.db> \
        --workspace-id <id> \
        --scope <object|image> \
        --model-id <model-id>

Exit codes:
    0  success
    1  error (details on stderr)

Stdout on success: JSON summary, e.g.
    {"projected": 1234, "method": "umap-v1"}
"""
from __future__ import annotations

import argparse
import json
import sqlite3
import struct
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path


PROJECTION_METHOD = "umap-v1"
UMAP_N_NEIGHBORS = 15
UMAP_MIN_DIST = 0.1
UMAP_RANDOM_STATE = 42
UMAP_METRIC = "cosine"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate UMAP projections from workspace embeddings."
    )
    parser.add_argument("--db-path", required=True, type=Path)
    parser.add_argument("--workspace-id", required=True)
    parser.add_argument("--scope", required=True, choices=["object", "image"])
    parser.add_argument("--model-id", required=True)
    return parser.parse_args()


def read_embedding_vectors(
    db_path: Path, workspace_id: str, scope: str, model_id: str
) -> list[tuple[str, list[float]]]:
    """Return list of (target_id, vector) from the embeddings table."""
    con = sqlite3.connect(str(db_path))
    try:
        cur = con.execute(
            """
            SELECT target_id, vector
            FROM embeddings
            WHERE workspace_id = ? AND scope = ? AND model_id = ?
            ORDER BY target_id
            """,
            (workspace_id, scope, model_id),
        )
        rows = cur.fetchall()
    finally:
        con.close()

    result = []
    for target_id, blob in rows:
        n = len(blob) // 4
        vector = list(struct.unpack_from(f"<{n}f", blob))
        result.append((target_id, vector))
    return result


def run_umap(vectors: list[list[float]]) -> list[tuple[float, float]]:
    import numpy as np
    import umap  # type: ignore[import]

    # n_neighbors must be < number of samples; clamp so small datasets don't crash
    n_neighbors = min(UMAP_N_NEIGHBORS, len(vectors) - 1)

    data = np.array(vectors, dtype=np.float32)
    reducer = umap.UMAP(
        n_components=2,
        n_neighbors=n_neighbors,
        min_dist=UMAP_MIN_DIST,
        metric=UMAP_METRIC,
        random_state=UMAP_RANDOM_STATE,
        low_memory=False,
    )
    embedding = reducer.fit_transform(data)

    # Normalise to [-1, 1] so the coordinate space matches PCA output
    mins = embedding.min(axis=0)
    maxs = embedding.max(axis=0)
    ranges = maxs - mins
    ranges[ranges == 0] = 1.0
    normalised = (embedding - mins) / ranges * 2.0 - 1.0

    return [(float(row[0]), float(row[1])) for row in normalised]


def write_projections(
    db_path: Path,
    workspace_id: str,
    scope: str,
    model_id: str,
    target_ids: list[str],
    coords: list[tuple[float, float]],
) -> None:
    now = datetime.now(timezone.utc).isoformat()
    rows = []
    for target_id, (x, y) in zip(target_ids, coords):
        row_id = str(uuid.uuid4())
        rows.append((row_id, workspace_id, scope, target_id, model_id, PROJECTION_METHOD, x, y, now))

    con = sqlite3.connect(str(db_path))
    try:
        con.executemany(
            """
            INSERT INTO embedding_projections
                (id, workspace_id, scope, target_id, model_id, projection_method, x, y, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(workspace_id, scope, target_id, model_id, projection_method)
            DO UPDATE SET x = excluded.x, y = excluded.y, created_at = excluded.created_at
            """,
            rows,
        )
        con.commit()
    finally:
        con.close()


def main() -> None:
    args = parse_args()

    print(f"[umap] reading embeddings: workspace={args.workspace_id} scope={args.scope} model={args.model_id}", file=sys.stderr)
    items = read_embedding_vectors(args.db_path, args.workspace_id, args.scope, args.model_id)

    if not items:
        print("[umap] no embedding vectors found; nothing to project", file=sys.stderr)
        print(json.dumps({"projected": 0, "method": PROJECTION_METHOD}))
        return

    if len(items) < 2:
        print(f"[umap] only {len(items)} vector(s); need at least 2 for UMAP", file=sys.stderr)
        sys.exit(1)

    if len(items) < UMAP_N_NEIGHBORS + 1:
        print(
            f"[umap] {len(items)} vectors < n_neighbors+1 ({UMAP_N_NEIGHBORS + 1}); "
            f"n_neighbors will be clamped to {len(items) - 1}",
            file=sys.stderr,
        )

    target_ids = [t for t, _ in items]
    vectors = [v for _, v in items]

    print(f"[umap] running UMAP on {len(vectors)} vectors (dim={len(vectors[0])})", file=sys.stderr)
    coords = run_umap(vectors)

    print(f"[umap] writing {len(coords)} projections to DB", file=sys.stderr)
    write_projections(args.db_path, args.workspace_id, args.scope, args.model_id, target_ids, coords)

    print(json.dumps({"projected": len(coords), "method": PROJECTION_METHOD}))


if __name__ == "__main__":
    main()

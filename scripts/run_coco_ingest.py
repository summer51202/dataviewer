"""
Ingest a COCO-format dataset into a new (or existing) DataViewer workspace.

Creates a GUI-compatible `.dataviewer/` workspace alongside your images:
    <workspace-root>/.dataviewer/workspace.json   (manifest)
    <workspace-root>/.dataviewer/workspace.db     (SQLite)
and populates images / annotations / categories from a COCO instances JSON.

Only the tables this tool writes are created here; when the DataViewer GUI later
opens the workspace it runs its own `CREATE TABLE IF NOT EXISTS` for the rest, so
the workspace stays GUI-compatible. Source images are never modified.

Usage:
    # single COCO file + images dir
    python run_coco_ingest.py \
        --coco path/to/instances.json \
        --images-dir path/to/images \
        --workspace-root path/to/workspace \
        [--name "My Dataset"] [--workspace-id my-dataset]

    # Roboflow-style dataset root: ingests every split (train/valid/test, each a
    # folder containing _annotations.coco.json + images) into ONE workspace,
    # with categories unified by name and one source folder per split.
    python run_coco_ingest.py \
        --dataset-root path/to/roboflow-export \
        --workspace-root path/to/workspace [--name "My Dataset"]

    python run_coco_ingest.py --self-test    # offline synthetic check, no real data

Exit codes: 0 success; 1 error (stderr).
Stdout on success: single-line JSON, e.g.
    {"workspace_id": "my-dataset", "workspace_db": ".../workspace.db",
     "images": 120, "annotations": 530, "categories": 4}
"""
from __future__ import annotations

import argparse
import json
import re
import sqlite3
import sys
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path

APP_VERSION = "0.1.0"
SCHEMA_VERSION = 1

# Tables this tool populates. The GUI backfills the remaining tables on open.
INIT_SQL = """
CREATE TABLE IF NOT EXISTS workspace_meta (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, workspace_path TEXT NOT NULL,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL, app_version TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS source_folders (
    id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, path TEXT NOT NULL,
    source_type TEXT NOT NULL, status TEXT NOT NULL, last_scan_at TEXT,
    image_count INTEGER NOT NULL DEFAULT 0, category_count INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS images (
    id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, source_id TEXT NOT NULL,
    file_name TEXT NOT NULL, original_path TEXT NOT NULL, relative_path TEXT,
    width INTEGER, height INTEGER,
    annotation_status TEXT NOT NULL DEFAULT 'unannotated',
    health_status TEXT NOT NULL DEFAULT 'healthy', health_error TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, source_id TEXT,
    name TEXT NOT NULL, normalized_name TEXT,
    category_role TEXT NOT NULL DEFAULT 'source',
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS annotations (
    id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, image_id TEXT NOT NULL,
    source_id TEXT NOT NULL, source_category_id TEXT, category_id TEXT,
    annotation_version_id TEXT, bbox_x REAL, bbox_y REAL, bbox_width REAL,
    bbox_height REAL, annotation_format TEXT NOT NULL,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_images_workspace_id ON images(workspace_id);
CREATE INDEX IF NOT EXISTS idx_annotations_image_id ON annotations(image_id);
CREATE INDEX IF NOT EXISTS idx_annotations_workspace_id ON annotations(workspace_id);
"""


def log(msg: str) -> None:
    print(f"[ingest] {msg}", file=sys.stderr, flush=True)


def fail(msg: str, code: int = 1):
    print(f"[ingest] ERROR: {msg}", file=sys.stderr, flush=True)
    sys.exit(code)


def slugify(name: str) -> str:
    s = re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")
    return s or "workspace"


ANNOTATION_FILE = "_annotations.coco.json"


def _init_workspace(workspace_root: Path, name: str, workspace_id: str, now: str) -> Path:
    """Create the .dataviewer dirs, DB schema, workspace_meta row and manifest."""
    hidden = workspace_root / ".dataviewer"
    for sub in ("", "cache", "temp", "exports"):
        (hidden / sub).mkdir(parents=True, exist_ok=True)
    db_path = hidden / "workspace.db"
    con = sqlite3.connect(str(db_path))
    try:
        con.executescript(INIT_SQL)
        con.execute(
            "INSERT OR REPLACE INTO workspace_meta (id, name, workspace_path, created_at, updated_at, app_version) "
            "VALUES (?,?,?,?,?,?)",
            (workspace_id, name, str(workspace_root), now, now, APP_VERSION),
        )
        con.commit()
    finally:
        con.close()
    manifest = {
        "id": workspace_id, "name": name, "workspacePath": str(workspace_root),
        "createdAt": now, "appVersion": APP_VERSION, "schemaVersion": SCHEMA_VERSION,
    }
    (hidden / "workspace.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    return db_path


def _write_split(con, workspace_id, source_id, source_path, coco, images_dir,
                 category_by_name, now):
    """Write one COCO split into an open connection. Categories are unified by
    normalized name across calls via the shared `category_by_name` map (so the
    same class across train/valid/test becomes one workspace category).
    Returns (n_images, n_annotations, skipped)."""
    coco_images = coco.get("images", [])
    coco_anns = coco.get("annotations", [])
    coco_cats = coco.get("categories", [])

    cat_map = {}  # this split's COCO category id -> workspace category uuid
    new_cat_rows = []
    for c in coco_cats:
        cname = str(c.get("name", c.get("id")))
        key = cname.strip().lower()
        if key not in category_by_name:
            category_by_name[key] = str(uuid.uuid4())
            new_cat_rows.append(
                (category_by_name[key], workspace_id, source_id, cname, key, "source", now, now)
            )
        cat_map[c["id"]] = category_by_name[key]

    con.execute(
        "INSERT OR REPLACE INTO source_folders "
        "(id, workspace_id, path, source_type, status, last_scan_at, image_count, category_count) "
        "VALUES (?,?,?,?,?,?,?,?)",
        (source_id, workspace_id, str(source_path), "COCO", "ready", now, len(coco_images), len(coco_cats)),
    )
    if new_cat_rows:
        con.executemany(
            "INSERT OR REPLACE INTO categories "
            "(id, workspace_id, source_id, name, normalized_name, category_role, created_at, updated_at) "
            "VALUES (?,?,?,?,?,?,?,?)",
            new_cat_rows,
        )

    anns_per_image = {}
    for a in coco_anns:
        anns_per_image[a["image_id"]] = anns_per_image.get(a["image_id"], 0) + 1

    img_map = {im["id"]: str(uuid.uuid4()) for im in coco_images}
    con.executemany(
        "INSERT OR REPLACE INTO images "
        "(id, workspace_id, source_id, file_name, original_path, relative_path, width, height, "
        "annotation_status, health_status, health_error, created_at, updated_at) "
        "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
        [
            (
                img_map[im["id"]], workspace_id, source_id,
                im.get("file_name", ""),
                str(images_dir / im.get("file_name", "")),
                im.get("file_name", ""),
                im.get("width"), im.get("height"),
                "annotated" if anns_per_image.get(im["id"], 0) > 0 else "unannotated",
                "healthy", None, now, now,
            )
            for im in coco_images
        ],
    )

    ann_rows = []
    skipped = 0
    for a in coco_anns:
        if a["image_id"] not in img_map:
            skipped += 1
            continue
        bbox = a.get("bbox") or [None, None, None, None]
        cat_uuid = cat_map.get(a.get("category_id"))
        ann_rows.append((
            str(uuid.uuid4()), workspace_id, img_map[a["image_id"]], source_id,
            cat_uuid, cat_uuid, None,
            bbox[0], bbox[1], bbox[2], bbox[3],
            "coco", now, now,
        ))
    con.executemany(
        "INSERT OR REPLACE INTO annotations "
        "(id, workspace_id, image_id, source_id, source_category_id, category_id, annotation_version_id, "
        "bbox_x, bbox_y, bbox_width, bbox_height, annotation_format, created_at, updated_at) "
        "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        ann_rows,
    )
    if skipped:
        log(f"skipped {skipped} annotation(s) referencing unknown image_id")
    return len(coco_images), len(ann_rows), skipped


def ingest(coco: dict, images_dir: Path, workspace_root: Path, name: str, workspace_id: str) -> dict:
    """Single-file ingest (one COCO json + one images dir)."""
    now = datetime.now(timezone.utc).isoformat()
    db_path = _init_workspace(workspace_root, name, workspace_id, now)
    category_by_name: dict = {}
    con = sqlite3.connect(str(db_path))
    try:
        n_img, n_ann, _ = _write_split(
            con, workspace_id, str(uuid.uuid4()), images_dir, coco, images_dir, category_by_name, now
        )
        con.commit()
    finally:
        con.close()
    return {
        "workspace_id": workspace_id,
        "workspace_db": str(db_path),
        "images": n_img,
        "annotations": n_ann,
        "categories": len(category_by_name),
    }


def discover_splits(root: Path):
    """Find Roboflow-style splits: (split_name, annotation_json, images_dir).
    Looks for `_annotations.coco.json` at the root and in each immediate subdir."""
    found = []
    if (root / ANNOTATION_FILE).exists():
        found.append((root.name, root / ANNOTATION_FILE, root))
    for child in sorted(p for p in root.iterdir() if p.is_dir()):
        if (child / ANNOTATION_FILE).exists():
            found.append((child.name, child / ANNOTATION_FILE, child))
    return found


def ingest_dataset_root(root: Path, workspace_root: Path, name: str, workspace_id: str) -> dict:
    """Ingest every Roboflow split under `root` into one workspace (categories
    unified by name; one source folder per split)."""
    splits = discover_splits(root)
    if not splits:
        fail(f"no '{ANNOTATION_FILE}' found under {root} (expected Roboflow-style split folders)")

    now = datetime.now(timezone.utc).isoformat()
    db_path = _init_workspace(workspace_root, name, workspace_id, now)
    category_by_name: dict = {}
    split_summaries = []
    total_images = total_anns = 0
    con = sqlite3.connect(str(db_path))
    try:
        for split_name, ann_path, images_dir in splits:
            try:
                coco = json.loads(ann_path.read_text(encoding="utf-8"))
            except Exception as e:  # noqa: BLE001
                fail(f"failed to read {ann_path}: {e}")
            n_img, n_ann, _ = _write_split(
                con, workspace_id, str(uuid.uuid4()), images_dir, coco, images_dir, category_by_name, now
            )
            log(f"split '{split_name}': {n_img} images, {n_ann} annotations")
            split_summaries.append({"split": split_name, "images": n_img, "annotations": n_ann})
            total_images += n_img
            total_anns += n_ann
        con.commit()
    finally:
        con.close()

    return {
        "workspace_id": workspace_id,
        "workspace_db": str(db_path),
        "images": total_images,
        "annotations": total_anns,
        "categories": len(category_by_name),
        "splits": split_summaries,
    }


def parse_args(argv=None):
    p = argparse.ArgumentParser(description="Ingest a COCO dataset into a DataViewer workspace.")
    p.add_argument("--self-test", action="store_true")
    p.add_argument("--coco", type=Path)
    p.add_argument("--images-dir", type=Path)
    p.add_argument("--dataset-root", type=Path, help="Roboflow-style export root with split folders")
    p.add_argument("--workspace-root", type=Path)
    p.add_argument("--name")
    p.add_argument("--workspace-id")
    return p.parse_args(argv)


def validate(args):
    if args.workspace_root is None:
        fail("--workspace-root is required")
    if args.dataset_root is not None:
        if not args.dataset_root.exists():
            fail(f"dataset root not found: {args.dataset_root}")
        return
    if args.coco is None or args.images_dir is None:
        fail("provide --dataset-root, or both --coco and --images-dir")
    if not args.coco.exists():
        fail(f"coco json not found: {args.coco}")
    if not args.images_dir.exists():
        log(f"warning: images dir does not exist yet: {args.images_dir}")


def self_test():
    log("self-test: building synthetic COCO")
    tmp = Path(tempfile.mkdtemp())
    coco = {
        "images": [{"id": i, "file_name": f"{i}.jpg", "width": 640, "height": 480} for i in range(5)],
        "categories": [{"id": 1, "name": "cat"}, {"id": 2, "name": "dog"}],
        "annotations": [
            {"id": 100 + k, "image_id": k % 5, "category_id": (k % 2) + 1, "bbox": [10, 10, 50, 50]}
            for k in range(12)
        ] + [{"id": 999, "image_id": 9999, "category_id": 1, "bbox": [0, 0, 1, 1]}],  # dangling
    }
    coco_path = tmp / "instances.json"
    coco_path.write_text(json.dumps(coco))
    summary = ingest(coco, tmp / "imgs", tmp / "ws", "Self Test", slugify("Self Test"))
    log("self-test summary: " + json.dumps(summary))
    assert summary["images"] == 5 and summary["categories"] == 2, summary
    assert summary["annotations"] == 12, summary  # dangling skipped
    # verify DB readable
    con = sqlite3.connect(summary["workspace_db"])
    try:
        n_img = con.execute("SELECT COUNT(*) FROM images").fetchone()[0]
        n_ann = con.execute("SELECT COUNT(*) FROM annotations").fetchone()[0]
        n_cat = con.execute("SELECT COUNT(*) FROM categories").fetchone()[0]
        joinable = con.execute(
            "SELECT COUNT(*) FROM annotations a JOIN images i ON i.id = a.image_id"
        ).fetchone()[0]
    finally:
        con.close()
    assert (n_img, n_ann, n_cat) == (5, 12, 2), (n_img, n_ann, n_cat)
    assert joinable == 12, joinable
    assert (tmp / "ws" / ".dataviewer" / "workspace.json").exists()

    # multi-split (Roboflow-style) ingest into one workspace
    log("self-test: building synthetic Roboflow-style root")
    root = Path(tempfile.mkdtemp())
    for split in ("train", "valid"):
        d = root / split
        d.mkdir(parents=True)
        scoco = {
            "images": [{"id": i, "file_name": f"{split}_{i}.jpg", "width": 100, "height": 100} for i in range(3)],
            "categories": [
                {"id": 0, "name": "none", "supercategory": "none"},
                {"id": 1, "name": "ball", "supercategory": "none"},
            ],
            "annotations": [
                {"id": i, "image_id": i, "category_id": 1, "bbox": [1, 1, 10, 10]} for i in range(3)
            ],
        }
        (d / "_annotations.coco.json").write_text(json.dumps(scoco))
    ms = ingest_dataset_root(root, root / "ws", "Multi", slugify("Multi"))
    log("multi-split summary: " + json.dumps(ms))
    assert ms["images"] == 6 and ms["annotations"] == 6, ms          # 3+3
    assert ms["categories"] == 2, ms                                  # 'none'+'ball' unified, not 4
    assert len(ms["splits"]) == 2, ms
    con = sqlite3.connect(ms["workspace_db"])
    try:
        n_src = con.execute("SELECT COUNT(*) FROM source_folders").fetchone()[0]
        n_cat2 = con.execute("SELECT COUNT(*) FROM categories").fetchone()[0]
    finally:
        con.close()
    assert n_src == 2, n_src                                          # one source per split
    assert n_cat2 == 2, n_cat2                                        # categories unified by name
    log("self-test: ALL PASSED")


def main():
    args = parse_args()
    if args.self_test:
        self_test()
        return
    validate(args)
    name = args.name or args.workspace_root.name
    workspace_id = args.workspace_id or slugify(name)
    if args.dataset_root is not None:
        summary = ingest_dataset_root(args.dataset_root, args.workspace_root, name, workspace_id)
    else:
        try:
            coco = json.loads(args.coco.read_text(encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            fail(f"failed to read coco json: {e}")
        summary = ingest(coco, args.images_dir, args.workspace_root, name, workspace_id)
    print(json.dumps(summary))


if __name__ == "__main__":
    main()

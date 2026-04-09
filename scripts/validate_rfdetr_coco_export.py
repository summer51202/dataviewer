from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate a DataViewer COCO export for rf-detr-advanced-aug-main."
    )
    parser.add_argument("dataset_root", type=Path, help="Root folder of the exported dataset.")
    parser.add_argument(
        "--expected-classes",
        type=int,
        default=None,
        help="Optional expected category count for a stricter check.",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def validate_split(dataset_root: Path, split: str, expected_classes: int | None) -> list[str]:
    errors: list[str] = []
    split_dir = dataset_root / split
    annotations_path = split_dir / "_annotations.coco.json"

    if not split_dir.is_dir():
        errors.append(f"[{split}] missing split directory: {split_dir}")
        return errors

    if not annotations_path.is_file():
        errors.append(f"[{split}] missing annotation file: {annotations_path}")
        return errors

    try:
        payload = load_json(annotations_path)
    except Exception as exc:  # noqa: BLE001
        errors.append(f"[{split}] failed to read annotation json: {exc}")
        return errors

    for key in ("images", "annotations", "categories"):
        if key not in payload or not isinstance(payload[key], list):
            errors.append(f"[{split}] missing or invalid '{key}' list")

    if errors:
        return errors

    images = payload["images"]
    annotations = payload["annotations"]
    categories = payload["categories"]

    if expected_classes is not None and len(categories) != expected_classes:
        errors.append(
            f"[{split}] category count mismatch: expected {expected_classes}, got {len(categories)}"
        )

    category_ids = set()
    for category in categories:
        if "id" not in category or "name" not in category:
            errors.append(f"[{split}] category missing required fields: {category}")
            continue
        category_ids.add(category["id"])

    image_ids = set()
    for image in images:
        image_id = image.get("id")
        file_name = image.get("file_name")
        width = image.get("width")
        height = image.get("height")

        if image_id is None or file_name is None:
            errors.append(f"[{split}] image missing id/file_name: {image}")
            continue

        if width is None or height is None:
            errors.append(f"[{split}] image missing width/height: {file_name}")

        image_ids.add(image_id)
        image_path = split_dir / file_name
        if not image_path.is_file():
            errors.append(f"[{split}] referenced image file missing: {image_path}")

    for annotation in annotations:
        image_id = annotation.get("image_id")
        category_id = annotation.get("category_id")
        bbox = annotation.get("bbox")

        if image_id not in image_ids:
            errors.append(f"[{split}] annotation references unknown image_id: {image_id}")
        if category_id not in category_ids:
            errors.append(f"[{split}] annotation references unknown category_id: {category_id}")
        if not isinstance(bbox, list) or len(bbox) != 4:
            errors.append(f"[{split}] annotation has invalid bbox: {annotation}")

    return errors


def main() -> int:
    args = parse_args()
    dataset_root = args.dataset_root.resolve()

    if not dataset_root.is_dir():
        print(f"Dataset root does not exist: {dataset_root}", file=sys.stderr)
        return 1

    all_errors: list[str] = []
    for split in ("train", "valid", "test"):
        all_errors.extend(validate_split(dataset_root, split, args.expected_classes))

    if all_errors:
        print("RF-DETR COCO validation failed:")
        for error in all_errors:
            print(f"- {error}")
        return 1

    print(f"RF-DETR COCO validation passed: {dataset_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

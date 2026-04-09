from __future__ import annotations

import argparse
from pathlib import Path
import sys
import yaml


IMAGE_SUFFIXES = {".jpg", ".jpeg", ".png", ".bmp", ".webp", ".tif", ".tiff"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate a DataViewer YOLO export."
    )
    parser.add_argument("dataset_root", type=Path, help="Root folder of the exported dataset.")
    parser.add_argument(
        "--expected-classes",
        type=int,
        default=None,
        help="Optional expected class count for a stricter check.",
    )
    return parser.parse_args()


def validate_split(dataset_root: Path, split: str) -> list[str]:
    errors: list[str] = []
    split_root = dataset_root / split
    images_dir = split_root / "images"
    labels_dir = split_root / "labels"

    if not images_dir.is_dir():
        errors.append(f"[{split}] missing images directory: {images_dir}")
    if not labels_dir.is_dir():
        errors.append(f"[{split}] missing labels directory: {labels_dir}")
    if errors:
        return errors

    image_files = sorted(
        path for path in images_dir.iterdir() if path.is_file() and path.suffix.lower() in IMAGE_SUFFIXES
    )
    label_files = sorted(path for path in labels_dir.iterdir() if path.is_file() and path.suffix.lower() == ".txt")

    image_stems = {path.stem for path in image_files}
    label_stems = {path.stem for path in label_files}

    missing_labels = sorted(image_stems - label_stems)
    extra_labels = sorted(label_stems - image_stems)

    if missing_labels:
        errors.append(f"[{split}] missing labels for images: {missing_labels[:5]}")
    if extra_labels:
        errors.append(f"[{split}] labels without images: {extra_labels[:5]}")

    for label_path in label_files[:50]:
        for line_no, line in enumerate(label_path.read_text(encoding="utf-8").splitlines(), start=1):
            parts = line.split()
            if len(parts) != 5:
                errors.append(f"[{split}] invalid label format at {label_path.name}:{line_no}")
                continue
            try:
                int(parts[0])
                values = [float(value) for value in parts[1:]]
            except ValueError:
                errors.append(f"[{split}] non-numeric label at {label_path.name}:{line_no}")
                continue
            if any(value < 0 or value > 1 for value in values):
                errors.append(f"[{split}] normalized bbox out of range at {label_path.name}:{line_no}")

    return errors


def main() -> int:
    args = parse_args()
    dataset_root = args.dataset_root.resolve()
    if not dataset_root.is_dir():
        print(f"Dataset root does not exist: {dataset_root}", file=sys.stderr)
        return 1

    data_yaml = dataset_root / "data.yaml"
    if not data_yaml.is_file():
        print(f"Missing data.yaml: {data_yaml}", file=sys.stderr)
        return 1

    data = yaml.safe_load(data_yaml.read_text(encoding="utf-8"))
    names = data.get("names")
    if not isinstance(names, dict):
        print("data.yaml 'names' must be a mapping of class ids to names.", file=sys.stderr)
        return 1

    if args.expected_classes is not None and len(names) != args.expected_classes:
        print(
            f"Class count mismatch: expected {args.expected_classes}, got {len(names)}",
            file=sys.stderr,
        )
        return 1

    all_errors: list[str] = []
    for split in ("train", "valid", "test"):
        all_errors.extend(validate_split(dataset_root, split))

    if all_errors:
        print("YOLO export validation failed:")
        for error in all_errors:
            print(f"- {error}")
        return 1

    print(f"YOLO export validation passed: {dataset_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

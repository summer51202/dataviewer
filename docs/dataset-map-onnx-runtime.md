# Dataset Map ONNX Runtime Notes

Dataset Map embeddings now require real local model assets before an embedding job can run.

## Model Asset Location

Place model files in the workspace-local model cache:

```text
{workspace-root}/.dataviewer/models/
```

Expected first-pass filenames:

- `clip-vit-b32.onnx`
- `dinov2-small.onnx`

## Runtime Behavior

- `Run / Refresh Embeddings` resolves the selected encoder to the matching ONNX file.
- If the file is missing, the job fails with `model asset not found`.
- The backend no longer writes deterministic placeholder embeddings in the production job path.
- CPU is the reliable baseline path for the first packaged Windows runtime.
- CUDA / Windows GPU remain user-selectable preferences, but provider probing still falls back to CPU until packaged provider support is completed.

## Current Pipeline

```text
annotation/image manifest
-> crop or full-image preprocessing
-> RGB resize to model input size
-> NCHW f32 normalization
-> ONNX provider batch inference
-> L2 normalization
-> embeddings table
-> pca-v1 projection
-> Dataset Map points
```

## Verification

Backend checks:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml --features onnx-runtime
```

App check:

1. Open Dataset Map.
2. Select an encoder.
3. Run embeddings without a model file.
4. Confirm the page shows the missing model error instead of changing points.

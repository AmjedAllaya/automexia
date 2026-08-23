
# Future Video / Media License Evidence

**State:** research evidence only; video is not a current Automexia dependency or active roadmap item.
**Access date:** 2026-08-23

Before any future video implementation ships, record the exact source artifact, model artifact, hash, version/tag, license, redistribution decision, and required notices.

## Current research evidence

### Silero VAD

- Upstream repository license file: MIT.
- Treat the exact ONNX/model asset as a separately hashed release artifact in Automexia evidence.
- Source: https://github.com/snakers4/silero-vad/blob/master/LICENSE

### YuNet face detector

- The model-specific `face_detection_yunet/LICENSE` file is MIT.
- OpenCV Zoo itself states that model licenses may differ from the repository-level license, so use the model-specific file for the exact model.
- Source: https://github.com/opencv/opencv_zoo/blob/main/models/face_detection_yunet/LICENSE

### RNNoise

- Upstream repository is BSD-3-Clause.
- The repository notes that its build flow downloads model files from Xiph servers.
- Therefore a future Automexia package must pin/hash the exact model data and include that model provenance in addition to the source-code license.
- Source: https://github.com/xiph/rnnoise

### DeepFilterNet

- Source code is dual MIT / Apache-2.0.
- Do **not** infer the pretrained-weight license from the code license.
- A July 15, 2026 upstream issue explicitly asks whether pretrained weights are also distributed under MIT/Apache-2.0.
- Until exact weight redistribution rights are resolved for the selected artifact, pretrained-weight redistribution remains blocked.
- Sources:
  - https://github.com/Rikorose/DeepFilterNet/blob/main/LICENSE-MIT
  - https://github.com/Rikorose/DeepFilterNet/issues/697

### whisper.cpp

- `whisper.cpp` source license is MIT.
- Model weights still require separate provenance/license review.
- Optional FFmpeg-related example code may carry different licensing; do not infer all examples share the core license.
- Sources:
  - https://github.com/ggml-org/whisper.cpp/blob/master/LICENSE
  - https://github.com/ggml-org/whisper.cpp/issues/3838

### resvg

- Current resvg releases are dual Apache-2.0 OR MIT; releases 0.44.0 and earlier were also available under the historical MPL-2.0 regime.
- Pin the exact renderer version because rendering behavior and license history can differ by release.
- Sources:
  - https://github.com/linebender/resvg/blob/main/README.md
  - https://github.com/linebender/resvg/blob/main/CHANGELOG.md

### FFmpeg

- FFmpeg is LGPL-2.1-or-later by default.
- Enabling optional GPL components changes the resulting FFmpeg build to GPL.
- A future managed-binary distribution therefore needs an exact build/configure manifest, matching source publication/compliance plan, codec/patent review, signing/update policy, and platform distribution decision.
- Source: https://www.ffmpeg.org/legal.html

## Release rule

No model/runtime may move from research to distribution on the basis of a repository-level license badge alone.

Required evidence:

```text
exact artifact
exact hash
source URL/tag
code license
model/weight license
redistribution decision
notices
patent/codec considerations
security/update owner
```

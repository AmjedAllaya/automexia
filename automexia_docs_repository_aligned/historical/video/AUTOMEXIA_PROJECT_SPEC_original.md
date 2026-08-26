# Automexia — Architecture & Video Automation Specification

> **Status:** Audited architecture baseline
> **Project:** Automexia
> **Primary language:** Rust
> **Architecture:** lightweight terminal core + isolated extensions
> **Primary video principle:** **Models observe. Rust decides. FFmpeg executes.**
> **Design goal:** accomplish repetitive work with a few simple commands, local processing, deterministic behavior, and no GUI dependency.

---

# 1. Executive Summary

Automexia is a lightweight Rust terminal whose domain capabilities live in extensions. The core terminal should remain small, fast, and independent from video, ML, DevOps, database, or other domain-specific runtimes.

The video capability is an extension named conceptually `automexia-video`. It is not a full non-linear editor and does not attempt to recreate Premiere Pro, DaVinci Resolve, or a motion-graphics GUI. Its purpose is to automate repetitive post-production tasks that are expensive to perform manually:

- smart silence shortening and cut cleanup
- voice enhancement
- conservative color correction
- face-aware reframing for multiple platform formats
- simple automatic graphic placement
- restrained punch-ins, transitions, and animations
- multi-target rendering
- export of the edit to professional NLE interchange formats

The normal user experience should stay close to:

```bash
video run recording.mp4
```

or:

```bash
video run recording.mp4 \
  --preset social \
  --to youtube,tiktok,reel
```

Professional users can stop before render and continue elsewhere:

```bash
video run recording.mov --no-render
video export recording.amxv --to resolve
```

The default video extension must require **no LLM, no AI agent, no paid API, no cloud inference, no Python, no PyTorch, no Node.js, and no Chromium**.

---

# 2. Non-Negotiable Architecture Rules

These rules prevent architectural drift.

## 2.1 Core stays domain-agnostic

The Automexia core must not know about:

- FFmpeg
- video codecs
- audio DSP
- ONNX
- ML models
- Whisper
- Docker
- Kubernetes
- databases
- cloud providers

The core owns execution and extension infrastructure only.

## 2.2 Domain features live in extensions

If a capability is useful only for one domain, it belongs outside core.

```text
Video processing       -> video extension
Docker/Kubernetes      -> DevOps extension
Database drivers       -> database extension
HTTP collections       -> API extension
Image/audio conversion -> media/files extension
```

## 2.3 No GUI dependency

Automexia can have a polished terminal UI, but the product must not depend on:

- a visual timeline
- drag-and-drop
- a node editor
- an embedded video editor
- an effect browser
- a waveform GUI

Commands, presets, workflows, project files, logs, and generated media are the primary interface.

## 2.4 Local-first and offline-first

The base video workflow must operate without network access after required components are installed.

Network access is allowed only for explicit operations such as installing/updating extensions or model packs.

## 2.5 Deterministic automation first

Preferred order:

```text
Algorithms
    -> deterministic rules
    -> DSP / computer vision
    -> tiny specialized local ML
    -> optional heavier local features
```

LLMs and agents are not part of the video architecture.

## 2.6 Original media is immutable

Automexia must never modify source footage in place.

All edits are represented as metadata in a project file and are only baked during rendering or explicit export/bake operations.

---

# 3. Explicit Non-Goals

The first versions of Automexia should **not** attempt to provide:

- a complete NLE
- a complete motion-graphics engine
- generative video
- generative B-roll
- text-to-video
- LLM-based editing
- AI agents
- automatic script writing
- cloud rendering
- cloud transcription
- collaborative editing
- a plugin-inside-plugin ecosystem
- reverse-engineered `.prproj` or `.drp` writing
- custom video/audio codecs
- custom full-resolution GPU compositor

These are only reconsidered if real user demand proves they are worth the complexity.

---

# 4. Automexia Core Responsibilities

The initial core should provide only capabilities that several extensions need.

## Required core capabilities

- terminal rendering and shell/process execution
- tabs/panes/sessions if already part of Automexia UX
- extension discovery and installation
- versioned extension protocol
- worker-process lifecycle
- permission declarations
- configuration loading
- workflow orchestration
- background job execution
- cancellation
- extension version/update management
- machine-readable command mode

## Deferred core capabilities

Do not make these requirements for V1:

- rich extension-defined GUI panels
- universal media preview UI
- typed object pipes between every command
- global CPU/GPU resource scheduler
- cloud sync
- collaboration
- full public marketplace services
- embedded AI runtime

---

# 5. Core Process Model

```text
┌────────────────────────────────────────┐
│            AUTOMEXIA CORE              │
│                                        │
│ terminal / shell                       │
│ extension registry                     │
│ workflow runner                        │
│ job manager                            │
│ configuration                          │
│ permission metadata                    │
└──────────────────┬─────────────────────┘
                   │ versioned IPC
        ┌──────────┼──────────┐
        ▼          ▼          ▼
      Video      DevOps      Files
      Worker     Worker      Worker
```

Large domain extensions should run as child worker processes rather than load native libraries directly into the Automexia process.

Benefits:

- core idle RAM stays low
- model memory is released when a worker exits
- a decoder/model crash does not crash the terminal
- domain dependencies can update independently
- extensions can use different implementation languages if needed later

---

# 6. Extension Package Structure

A native extension should have an explicit package structure.

Example:

```text
automexia-video/
├── manifest.toml
├── bin/
│   ├── linux-x86_64/
│   │   └── automexia-video-worker
│   ├── macos-aarch64/
│   │   └── automexia-video-worker
│   └── windows-x86_64/
│       └── automexia-video-worker.exe
├── assets/
│   ├── presets/
│   ├── platform-presets/
│   └── graphics/
├── models/
│   ├── manifests/
│   └── optional/
├── licenses/
└── checksums.json
```

Large optional components should not be placed in the base package unless installed.

---

# 7. Extension Manifest

A manifest must describe compatibility and required capabilities before execution.

Example:

```toml
id = "automexia.video"
name = "Video"
version = "1.0.0"
protocol = 1

[compatibility]
automexia = ">=0.8,<2.0"

[entrypoints]
linux_x86_64 = "bin/linux-x86_64/automexia-video-worker"
macos_aarch64 = "bin/macos-aarch64/automexia-video-worker"
windows_x86_64 = "bin/windows-x86_64/automexia-video-worker.exe"

[permissions]
network = false
gpu = "optional"
filesystem = ["project", "cache", "temp"]
subprocess = ["ffmpeg", "ffprobe"]

[features]
transcription = false
vision_plus = false
voice_quality = false
vision_advanced = false
```

Unknown manifest fields should be rejected or explicitly version-gated rather than silently ignored.

---

# 8. Native Worker IPC

For V1, use a simple cross-platform protocol rather than sockets or a custom RPC framework.

Recommended transport:

```text
stdin/stdout + newline-delimited JSON (JSONL)
```

Rules:

- Automexia spawns the worker directly; do not invoke through a shell.
- stdin carries requests.
- stdout carries protocol messages only.
- stderr carries human/debug logs.
- media bytes are passed through files or file descriptors/pipes, never base64 JSON.
- every request has an ID.
- every message has a protocol version.
- malformed messages terminate the request cleanly.

Example handshake:

```json
{"type":"hello","protocol":1,"core_version":"0.8.0"}
```

Worker:

```json
{
  "type":"hello_ok",
  "protocol":1,
  "extension":"automexia.video",
  "version":"1.0.0",
  "capabilities":["video.run","video.render","video.export"]
}
```

Progress event:

```json
{
  "type":"progress",
  "request_id":"r17",
  "stage":"analysis",
  "progress":0.62,
  "message":"Analyzing faces"
}
```

Completion:

```json
{
  "type":"result",
  "request_id":"r17",
  "ok":true,
  "outputs":["recording.amxv","recording.edited.mp4"]
}
```

Do not stabilize Rust struct ABI across dynamic libraries. Stabilize the serialized protocol instead.

---

# 9. WASM vs Native Extensions

WASM can be valuable later for small third-party extensions, but it is not required to deliver the video extension.

## Native workers

Use when an extension needs:

- FFmpeg
- native codecs
- local ML runtimes
- GPU/device APIs
- heavy system integrations

The first-party video extension should be a native worker.

## WASM

Use later for:

- small workflow integrations
- text/data processing
- safer third-party extensions

Important: a permission declaration alone does **not** securely sandbox an arbitrary native executable. Native third-party workers require OS-level sandboxing or must be treated as trusted code. WASM is the preferred future path when strong sandboxing is required.

---

# 10. Security Rules for Extension Execution

- Never build shell command strings from user paths.
- Use `std::process::Command` with separate arguments.
- Prefer FFmpeg filter script files over enormous shell-escaped filter strings.
- Canonicalize or validate paths before granting project access.
- Restrict temporary files to an Automexia-controlled temp directory.
- Do not allow SVG/graphics assets to fetch arbitrary network resources.
- Model files installed by Automexia must have pinned checksums.
- Native model loading should reject files outside approved model directories by default.
- Worker crashes must be reported as extension failures, not terminal failures.

---

# 11. Configuration, Presets, and Workflows

These are different concepts.

## Configuration

Controls machine/user behavior:

```toml
[video.runtime]
ffmpeg = "auto"
threads = "auto"
hardware_encode = "auto"
cache = true
```

## Preset

Controls parameters for one domain operation.

```toml
[preset.clean.silence]
enabled = true
min_pause_ms = 650
keep_ms = 180
vad_threshold = 0.65

[preset.clean.voice]
denoise = "rnnoise"
target_lufs = -16.0

[preset.clean.color]
auto_exposure = true
auto_white_balance = true
```

## Workflow

Controls a sequence of extension actions.

Example using TOML to avoid requiring another config language:

```toml
name = "publish-social"

[[steps]]
action = "video.run"
input = "${input}"
preset = "social"
targets = ["youtube", "tiktok", "reel"]

[[steps]]
action = "files.move"
input = "${previous.outputs}"
destination = "./published"
```

Workflows should pass paths, project references, scalar values, and JSON-compatible metadata. A universal typed-pipe protocol is not required for V1.

---

# 12. Configuration Precedence

Use a predictable override order:

```text
built-in defaults
    < user configuration
    < project configuration
    < named preset
    < command-line flags
```

Projects must snapshot the resolved parameters that affected editing decisions. Rerendering a project should not silently change because a global preset was later edited.

Unknown configuration keys should produce an error in strict mode.

---

# 13. Background Jobs

A long render can run as a core-managed job.

V1 needs:

- progress
- logs
- cancellation
- failure state
- completion state
- output paths

Pause/resume is not a V1 requirement because not every native process or encoder can resume safely.

Normal command execution may remain foreground by default. An optional common flag can submit it to the job manager:

```bash
video run recording.mp4 --background
```

---

# 14. Machine-Readable CLI Contract

Every major command should support:

```bash
--json
```

In JSON mode:

- stdout contains a single result document or JSONL events.
- human-readable logs go to stderr.
- scripts never need to parse decorated terminal text.

Suggested exit codes:

```text
0   success
2   invalid arguments/config
3   missing runtime/dependency
4   unsupported input/capability
5   analysis failure
6   render failure
7   export failure
8   project/schema failure
130 cancelled
```

---

# 15. Automexia Video Extension — Purpose

`automexia-video` is an automated post-production extension.

It should excel at:

- cleaning spoken-word video
- reducing repetitive manual editing
- producing multiple aspect-ratio variants
- preparing clean audio/video for publishing
- creating an editable project file
- exporting the timeline to an NLE

It should not compete with a creative NLE for manual editing.

---

# 16. Video Command Surface

Keep the top-level command surface very small.

## Run

```bash
video run INPUT
```

Examples:

```bash
video run recording.mp4
video run recording.mp4 --preset clean
video run recording.mp4 --preset social --to youtube,tiktok,reel
video run recording.mov --no-render
video run recording.mp4 --dry-run
video run recording.mp4 --json
```

Pipeline:

```text
probe -> analyze -> decide -> save project -> render
```

## Render

```bash
video render PROJECT
```

Examples:

```bash
video render recording.amxv --to youtube
video render recording.amxv --codec prores --output master.mov
```

## Export

```bash
video export PROJECT --to TARGET
```

Examples:

```bash
video export recording.amxv --to resolve
video export recording.amxv --to premiere
video export recording.amxv --format otio
video export recording.amxv --format fcp7xml
video export recording.amxv --format edl
```

There should be no separate mandatory `reframe`, `adapt`, `enhance`, or `effects` command for normal users. Those capabilities are pipeline stages and preset options.

---

# 17. Video Dependency Strategy

The default stack should prioritize reliability and installation simplicity over minimizing every last megabyte inside the optional video extension.

```text
Required for video extension
├── Rust worker
├── FFmpeg / ffprobe
├── serde / JSON
├── ONNX Runtime CPU backend
├── Silero VAD
├── UltraFace
├── RNNoise
├── resvg / usvg / tiny-skia
└── small Rust math/image/FFT utilities as required

Optional
├── transcription
│   └── whisper.cpp tiny/base
├── voice-quality
│   └── DeepFilterNet
├── vision+
│   └── small permissively licensed person/object detector
└── vision-advanced
    └── OpenCV only if advanced optical flow/stabilization is justified
```

Do not include by default:

- WhisperX
- pyannote
- Python
- PyTorch
- TensorFlow
- HyperFrames
- Node.js
- Chromium
- wgpu
- active-speaker models
- large general-purpose vision models

---

# 18. ML Inference Runtime Decision

Use **CPU-only ONNX Runtime** as the initial generic inference backend.

Why:

- mature operator coverage
- predictable ONNX deployment
- cross-platform
- tiny models do not require GPU acceleration
- the runtime lives only inside the optional video extension, not Automexia core

Do not bundle CUDA/CoreML/DirectML/OpenVINO providers by default merely because they exist. Add platform accelerators only after profiling demonstrates a real benefit for the shipped models.

A pure-Rust runtime such as `tract` can be evaluated later as a binary-footprint optimization. Do not replace ONNX Runtime until the exact shipped models pass accuracy, performance, and compatibility tests.

---

# 19. Default Local Models

## Silero VAD

Purpose:

- speech probability
- pause boundaries
- silence candidate generation

It analyzes 16 kHz mono audio generated from the source audio.

The model only returns observations. Rust decides whether a pause is edited.

## UltraFace

Purpose:

- face detection
- face-aware reframing
- face avoidance for graphics
- face-track initialization

Inference should run on low-resolution frames, not full-resolution source frames.

## RNNoise

Purpose:

- lightweight local speech denoising

RNNoise is a specialized speech enhancer, not a general audio restoration system. Aggressive denoising should not be applied to music-heavy material automatically.

---

# 20. Optional Local Components

## `transcription`

Use `whisper.cpp` tiny/base for:

- generated subtitles
- transcript search
- sentence boundaries
- filler-word workflows
- chapters

WhisperX is intentionally excluded from the default path because it introduces a heavier Python/alignment/diarization stack. If precise forced alignment later becomes a proven requirement, build a dedicated optional alignment component rather than forcing WhisperX into the base install.

## `voice-quality`

DeepFilterNet may replace or supplement RNNoise when a user explicitly chooses higher-quality voice enhancement.

## `vision+`

A small permissively licensed object/person detector can improve:

- no-face nature/B-roll framing
- person tracking
- object-following callouts

Do not install it for users who only need talking-head workflows.

## `vision-advanced`

OpenCV is deferred until features such as robust optical flow, homography, or stabilization justify the dependency.

---

# 21. Dependency and Model License Policy

Every runtime/model asset must have a manifest containing:

```toml
name = "silero-vad"
version = "..."
kind = "model"
license = "MIT"
runtime = "onnx"
sha256 = "..."
source = "..."
```

Track separately:

- source-code license
- model-weight license
- dataset/use restrictions if relevant
- redistribution permission
- codec/patent considerations

Current architectural candidates use permissive licenses for their code repositories: Silero VAD is MIT, UltraFace is MIT, RNNoise is BSD-3-Clause, ONNX Runtime is MIT, DeepFilterNet is MIT/Apache-2.0, whisper.cpp is MIT, and resvg is MIT/Apache-2.0. These facts must still be re-verified for the exact version and model asset before release.

FFmpeg requires special care. FFmpeg is LGPL by default, while enabling certain optional GPL components changes the licensing obligations of that build. Automexia should have an explicit FFmpeg distribution policy before shipping managed FFmpeg binaries.

---

# 22. FFmpeg Runtime Policy

Prefer this order:

```text
1. compatible system FFmpeg if available
2. Automexia-managed FFmpeg runtime if installed
3. actionable dependency error
```

The video extension must probe capabilities instead of assuming them.

At startup/first run, discover:

- FFmpeg version
- available decoders
- available encoders
- filters
- hardware accelerators
- pixel formats

Do not promise H.264, H.265, ProRes, AV1, or hardware encoding unless the current FFmpeg build reports a compatible encoder.

A managed runtime should be installed with the video extension or as an extension component, never in Automexia core.

---

# 23. FFmpeg Invocation Rules

Rust plans; FFmpeg executes.

Use:

```rust
Command::new(ffmpeg_path)
    .args([...])
```

Never:

```text
sh -c "ffmpeg ... ${user_input} ..."
```

For complex graphs, generate a temporary filter script and invoke it with FFmpeg's filter-script support. This avoids shell escaping problems and command-line length limits.

---

# 24. Capability Detection and Graceful Fallback

Before processing, the worker should build a capability record:

```json
{
  "ffmpeg_version":"...",
  "encoders":["..."],
  "filters":["..."],
  "hardware":["..."],
  "onnx_provider":"cpu",
  "models":["silero-vad","ultraface"],
  "features":["base"]
}
```

Rules:

- CPU inference is always the baseline.
- hardware encoding is optional.
- missing hardware encoder falls back to software if a compatible software encoder exists.
- missing optional feature pack produces a clear error only when that feature is requested.
- never silently switch to a materially different edit algorithm without reporting it.

An extension-management diagnostic command can expose this without increasing the normal video command surface:

```bash
automexia ext doctor video
```

---

# 25. Video Processing Architecture

```text
INPUT
  │
  ▼
ffprobe
  │
  ▼
Media normalization metadata
  │
  ├─────────────┐
  ▼             ▼
low-res video   analysis audio
  │             │
  └──────┬──────┘
         ▼
   ONE ANALYSIS PASS
         │
         ▼
    Analysis Cache
         │
         ▼
  Rust Decision Engine
         │
         ▼
     .amxv Project
         │
         ▼
   FFmpeg Compiler
         │
      ┌──┴──────────────┐
      ▼                 ▼
  Final Render       NLE Export
```

The four implementation phases are:

```text
ANALYZE -> DECIDE -> COMPILE -> EXECUTE
```

---

# 26. Input Probe and Media Normalization

Every input must be inspected with `ffprobe` before editing.

Record at minimum:

## Video

- stream index
- codec
- width/height
- sample aspect ratio
- display aspect ratio
- rotation/display matrix
- pixel format
- average frame rate
- real frame rate if reported
- stream time base
- duration
- color primaries
- transfer characteristics
- matrix coefficients
- color range
- field order/interlacing
- HDR indicators/metadata when present

## Audio

- stream index
- codec
- sample rate
- channel count
- channel layout
- duration
- stream time base

## Container

- format
- duration
- start time
- metadata/timecode if present

Do not assume the first stream is always the correct stream. Default to FFmpeg's default stream disposition and allow explicit stream selection when needed.

---

# 27. Time and Timestamp Model

This is critical. Do not base editing on integer frame numbers.

Video may be variable frame rate and different streams may have different time bases.

Store edit points using exact source timestamp information:

```rust
struct MediaTime {
    pts: i64,
    time_base_num: i32,
    time_base_den: i32,
}
```

Seconds are:

```text
pts * time_base_num / time_base_den
```

Rules:

- preserve source PTS relationships
- never assume `1 / fps` equals the stream time base
- never convert everything to floating-point seconds early
- perform rational conversion only at boundaries that require it
- keep audio/video cut boundaries synchronized from the same timeline edit decision

Project-level derived positions may use a canonical rational time base, but source references must retain their original timing information.

---

# 28. Variable Frame Rate, Rotation, and Display Space

## VFR

The project timeline is timestamp-based, so VFR inputs are valid.

Render presets declare one of:

```text
frame_rate_mode = preserve
frame_rate_mode = cfr
```

Do not silently convert VFR to 30 fps.

## Rotation

Analysis should occur in **display orientation**, not raw coded orientation. Face boxes and crop coordinates must refer to display-space geometry.

## Pixel aspect ratio

Reframing decisions should operate in display space. The compiler converts them back to coded-pixel coordinates as needed.

---

# 29. HDR and Color-Space Safety

Automatic SDR-oriented color correction must **not** be applied blindly to HDR footage.

V1 policy:

```text
SDR / supported Rec.709-like input
    -> automatic correction allowed

HDR / HLG / PQ / Dolby Vision-like metadata
    -> preserve by default
    -> disable automatic SDR correction
    -> warn if output preset would force SDR
```

Dedicated HDR tone mapping is a separate future feature.

Color metadata should be preserved or explicitly transformed during rendering.

---

# 30. Interlaced Input

Detect field order before processing.

For analysis, the worker may create a deinterlaced low-resolution analysis stream.

For output:

- preserve interlace only when requested by a professional preset
- social/web presets may explicitly deinterlace to progressive output
- never accidentally treat interlaced fields as progressive frames during motion/reframe analysis

---

# 31. Shared Low-Resolution Analysis Pass

The extension must not re-run expensive analysis independently for every feature.

Create analysis streams such as:

```text
video: display-oriented 320p/480p stream
       low pixel cost
       enough temporal sampling for face/scene analysis

audio: 16 kHz mono for VAD
       plus required audio statistics
```

Different detectors can request different temporal sampling while sharing one decode path when practical.

Example output cache:

```json
{
  "speech":[],
  "silence_candidates":[],
  "faces":[],
  "face_tracks":[],
  "scene_cuts":[],
  "luma_stats":[],
  "color_stats":[],
  "motion_activity":[]
}
```

Full-resolution source frames should not be sent through ML models.

---

# 32. Analysis Cache

Cache analysis metadata, not huge rendered intermediates by default.

Cache key should include:

- source fingerprint
- selected stream IDs
- analysis resolution/rate
- algorithm versions
- model names/versions/hashes
- relevant thresholds
- analysis pipeline version

Example conceptual path:

```text
<automexia-cache>/video/<cache-key>/analysis.json
```

A source fingerprint should combine inexpensive identity signals such as:

- canonical path
- file size
- modification time
- partial content hash

A full-file hash can be optional for high-integrity workflows.

If a fingerprint changes, invalidate the relevant cache.

---

# 33. Reproducibility

A project must snapshot:

- Automexia video extension version
- project schema version
- resolved preset parameters
- model identifiers and hashes
- algorithm versions
- selected media streams
- source fingerprints

Two reproducibility levels must be distinguished:

## Decision reproducibility

Given identical source, preset snapshot, algorithm version, and model versions, edit decisions should be deterministic.

## Bit-exact render reproducibility

Not guaranteed across different FFmpeg builds, hardware encoders, operating systems, or codec implementations.

The project should promise deterministic **edit intent**, not identical compressed bytes everywhere.

---

# 34. `.amxv` Project Format

Use a versioned, human-readable JSON project.

Extension:

```text
.amxv
```

Goals:

- inspectable
- Git-diffable
- scriptable
- migration-friendly
- independent of Automexia session state

Paths should be project-relative when possible.

The schema supports multiple source records so it does not need to be redesigned later. V1 automation, however, should optimize for **one primary video/program source plus optional graphics, subtitles, and auxiliary audio assets**. Full multi-camera/multi-clip creative editing is not a V1 goal.

Example high-level structure:

```json
{
  "schema":"automexia.video.project",
  "version":1,
  "created_with":{
    "extension":"1.0.0"
  },
  "sources":[],
  "analysis":{},
  "timeline":{},
  "operations":[],
  "variants":{},
  "graphics":{},
  "render":{},
  "exports":{}
}
```

---

# 35. Required `.amxv` Source Record

Example:

```json
{
  "id":"source-1",
  "path":"media/recording.mp4",
  "fingerprint":{
    "size":123456789,
    "mtime_ns":123456789000,
    "partial_sha256":"..."
  },
  "streams":{
    "video":0,
    "audio":1
  },
  "probe":{
    "width":3840,
    "height":2160,
    "video_time_base":{"num":1,"den":90000},
    "audio_time_base":{"num":1,"den":48000},
    "color_space":"bt709",
    "rotation":0
  }
}
```

Do not embed the entire raw ffprobe response unless placed in an optional diagnostics section/cache. Store the normalized subset required to compile the project.

---

# 36. Edit Operation Model

Operations should be typed, versioned, and explainable.

Example:

```json
{
  "id":"op-42",
  "kind":"shorten_silence",
  "source":"source-1",
  "range":{
    "start":{"pts":1287720,"tb":{"num":1,"den":90000}},
    "end":{"pts":1458000,"tb":{"num":1,"den":90000}}
  },
  "parameters":{
    "keep_ms":180
  },
  "confidence":0.96,
  "reason":[
    "vad_non_speech",
    "low_audio_energy",
    "duration_above_threshold"
  ],
  "generator":{
    "algorithm":"silence-v1",
    "version":1
  }
}
```

Manual edits made directly to the project may set:

```json
{"locked":true}
```

A recomputation pass should not overwrite locked operations unless explicitly requested.

---

# 37. Decision Confidence

Confidence is useful only when it affects behavior.

Default policy can be preset-controlled:

```text
high confidence      -> apply
medium confidence    -> keep proposal in project but disable by default
low confidence       -> ignore
```

Do not hard-code one global threshold for every feature. Silence, color, graphic placement, and reframing have different confidence models.

Every automated operation should store reason codes so a dry run can explain what the system intends to do.

---

# 38. Dry Run

```bash
video run recording.mp4 --dry-run
```

Dry run performs probe, analysis, and decision generation but does not render final media.

Example:

```text
Input: recording.mp4
Duration: 18:32

Would apply:
  37 pause edits
   8 subtle punch-ins
   1 voice-enhancement chain
  11 shot-level exposure corrections
   1 9:16 reframe track

Estimated edited duration: 15:47

Project preview: recording.amxv
No final video rendered.
```

The project may still be written because it is the main inspectable artifact.

---

# 39. Smart Silence Analysis

Use multiple signals rather than a raw dB threshold.

Inputs:

- Silero VAD probability
- RMS/energy
- pause duration
- speech padding requirements
- adjacent audio continuity

Initial preset behavior:

```text
very short pauses      -> preserve
normal speech pauses   -> preserve or shorten slightly
long pauses            -> shorten
very long dead air     -> shorten aggressively
```

Exact thresholds belong in presets and should be tuned with a regression dataset.

---

# 40. Smart Silence Guardrails

Silence editing must avoid robotic speech and audio discontinuities.

Rules:

- keep padding before and after detected speech
- enforce a minimum retained gap
- do not cut through low-confidence speech boundaries
- avoid repeated cuts separated by tiny speech islands
- merge nearby silence candidates when appropriate
- preserve breaths when the default `clean` preset considers them natural
- use short audio fades/crossfades to avoid clicks

Important limitation:

A video containing a continuous music bed mixed into the same audio stream cannot have arbitrary time removed without also jumping the music. The default algorithm should detect meaningful non-speech background energy and become more conservative, or warn. Best results occur when music is on a separate stream/track or silence shortening is disabled for music-heavy mixes.

---

# 41. Cut Smoothing

## Audio

Use:

- zero-crossing-aware boundaries where practical
- micro fades
- short crossfades
- optional room-tone preservation

Do not use long crossfades between spoken words because they can smear phonemes.

## Video

Normal behavior is a hard cut.

A jump-cut mitigation can alter crop/scale after the cut when:

- the cut remains in the same scene
- a stable face exists
- the new crop remains compositionally safe

Do not add a transition merely because a cut exists.

---

# 42. Voice Enhancement Pipeline

Before processing, perform a cheap audio preflight:

- peak level and clipping count
- DC offset
- integrated/short-term loudness measurements as needed
- channel imbalance
- 50/60 Hz hum evidence and harmonics
- speech occupancy from VAD
- continuous non-speech bed energy

This preflight determines whether optional cleanup such as a hum notch is safe. If the source is already digitally clipped, report it; normal compression/normalization cannot reconstruct missing peaks.

Default spoken-word chain:

```text
decode / channel selection
    -> DC/rumble cleanup
    -> optional high-confidence 50/60 Hz dehum notch
    -> RNNoise
    -> edit/cut assembly
    -> conservative EQ
    -> de-esser
    -> compression
    -> loudness normalization
    -> true-peak limiting
```

Use FFmpeg filters where they are mature and appropriate; RNNoise can be integrated as a small native processing stage.

---

# 43. Voice Enhancement Safety

V1 should optimize for spoken-word content, not arbitrary music mastering.

Rules:

- inspect speech occupancy before enabling aggressive voice processing in `auto` mode
- keep denoise conservative when strong continuous non-speech content is present
- do not assume a multi-channel surround mix is a dialogue track
- for more than stereo or multiple audio streams, choose a clear default stream and allow explicit selection
- never silently downmix the final output merely because VAD analysis uses mono

Suggested mode:

```text
voice_mode = auto | on | off
```

`auto` enables the full voice chain only when the input appears to be primarily spoken-word content.

---

# 44. Loudness Processing

Use measurement-based loudness normalization rather than a single gain adjustment.

For high-quality presets, use a measurement pass followed by a correction pass.

Targets belong to presets, not platform mythology. Example starting points may include:

```text
clean spoken-word: configurable around -16 LUFS
social:           configurable around -14 to -16 LUFS
true peak:        configurable ceiling
```

These are starting defaults, not claims about mandatory platform requirements.

---

# 45. Color Analysis

V1 color automation should be conservative and correction-oriented.

Analyze per shot:

- luminance percentiles
- clipping
- channel balance
- neutral/low-saturation regions
- basic contrast distribution
- face-region luma as an additional guard when a face exists

Possible algorithms:

- histogram percentiles
- gray-world/shades-of-gray white balance
- gamma/exposure correction
- restrained tone curve

Do not attempt artistic grading automatically.

---

# 46. Color Correction Guardrails

Suggested initial safety clamps to tune during testing:

- exposure correction limited to a conservative range
- white-balance gains clamped to prevent strong color casts
- saturation changes small by default
- avoid correction when neutral-reference confidence is low
- smooth correction within a shot
- allow discontinuity at an actual scene cut

If the algorithm is not confident, preserve the source rather than force a correction.

Creative looks such as `warm`, `film`, or `high-contrast` are explicit user presets, not part of automatic correction.

---

# 47. Scene Detection

Scene boundaries support:

- color analysis
- crop reset
- transition policy
- graphic placement stability
- punch-in reset

V1 can use FFmpeg scene scoring or a low-resolution histogram/frame-difference algorithm.

Scene detection should process the low-resolution decode at sufficient temporal resolution to avoid missing one-frame cuts.

---

# 48. V1 Face Tracking Without OpenCV

Do not add OpenCV just to maintain a face track.

V1 approach:

1. run UltraFace on sampled low-resolution frames
2. associate detections between samples using:
   - IoU
   - center distance
   - size consistency
3. predict/smooth track state with a small Kalman implementation
4. expire a track after a configurable miss window

The detector can be sampled at a configurable rate; it does not need to run on every full-resolution frame.

---

# 49. Smart Reframing

Supported targets are data-driven presets:

```text
16:9
9:16
4:5
1:1
custom WxH/aspect
```

The crop algorithm should operate in source **display space**.

V1 inputs:

- stable face tracks
- face group geometry
- simple motion/activity estimate
- output safe zones
- previous crop state

---

# 50. Reframing Behavior

## One face

Track it while respecting composition.

## Multiple nearby faces

Prefer a crop that contains the group.

## Multiple separated faces

V1 should prefer the largest/stablest subject unless the crop can include the group cleanly.

Active-speaker switching is not a default V1 dependency.

## No face

Be conservative:

- preserve original composition when possible
- use a centered/stable crop
- use simple motion/activity to avoid obviously bad framing
- do not invent camera motion unnecessarily

Advanced object-aware nature/B-roll framing belongs in `vision+`.

---

# 51. Composition Rules

The crop target is not simply face center.

Use:

- headroom
- rule-of-thirds preference
- body/face margin
- look direction only if a reliable cheap signal exists
- movement direction only if reliable
- multiple-person inclusion
- platform safe zone
- previous crop position

When uncertain, prefer stable composition over continuous movement.

---

# 52. Reframe Smoothing

Raw detections must never become direct crop coordinates.

Pipeline:

```text
detections
   -> association
   -> target rectangle
   -> dead zone
   -> Kalman/low-pass smoothing
   -> velocity limit
   -> acceleration limit
   -> keyframes
```

The virtual camera should not chase small head movements.

---

# 53. Reframe Keyframes

Store reframing as metadata, not baked pixels.

Example:

```json
{
  "variant":"tiktok",
  "keyframes":[
    {"t":"...","cx":0.52,"cy":0.48,"scale":1.42},
    {"t":"...","cx":0.54,"cy":0.48,"scale":1.42}
  ]
}
```

`cx`/`cy` can be normalized display-space coordinates. Round persisted floating values consistently to avoid noisy diffs.

---

# 54. Platform Presets and Safe Zones

Platform behavior changes over time, so safe zones must be data, not algorithm code.

Example:

```toml
[target.tiktok]
width = 1080
height = 1920
aspect = "9:16"

[safe_zone]
top = 0.06
bottom = 0.16
left = 0.05
right = 0.12
```

Safe-zone values are conservative extension presets, not guarantees about a platform's current UI.

Platform preset files should be versioned and updateable independently from the core algorithm.

---

# 55. Automatic Graphic Placement — Scope

Automexia can automatically decide **where** and **how** to place a supplied graphic.

Without language understanding it should not claim to automatically decide the semantic moment when an arbitrary chart/logo/callout should exist.

Timing comes from:

- workflow configuration
- subtitle timing
- a user-provided time range
- a reusable template
- optional transcription-based rules when transcription is installed

Supported V1 graphic types:

- logo/watermark
- title
- lower third
- subtitle/caption asset
- supplied image/SVG

Object-following callouts are later/optional.

---

# 56. V1 Graphic Placement Cost Map

Default inputs should only depend on capabilities actually installed.

Base cost signals:

```text
face overlap              very expensive
Automexia-owned text      expensive
platform unsafe zone      forbidden/highest cost
frame edge/clutter        moderate
local edge density        moderate
local variance            moderate
motion/activity           moderate
placement movement        penalty
empty/quiet region        preferred
```

Do **not** claim base V1 understands arbitrary people, objects, active speakers, or baked-in text unless the corresponding optional detector is installed.

---

# 57. Graphic Placement Algorithm

Evaluate a small set of stable candidate anchors rather than searching every pixel.

Examples:

```text
top-left
top-center
top-right
middle-left
middle-right
bottom-left
bottom-center
bottom-right
```

For each candidate compute:

```text
cost =
    face_overlap
  + known_graphic_overlap
  + unsafe_zone_penalty
  + edge_density
  + visual_activity
  + movement_from_previous_position
```

Choose the lowest-cost candidate satisfying minimum readability constraints.

Placement should be optimized per shot/segment, not every frame.

---

# 58. Graphic Readability

Measure the region under text/graphics:

- average luminance
- contrast
- local variance
- edge density

Possible reactions:

- choose light/dark text variant
- add shadow
- add opaque/semi-opaque background plate
- move to another candidate zone
- reduce size within preset limits

If no candidate is safe, prefer a background plate or a less-obstructive placement instead of covering a face.

---

# 59. Graphic Rendering Stack

Do not use Chromium/HyperFrames in the default build.

Use static Rust-native assets:

```text
SVG template + supplied text/images
          |
     usvg/resvg
          |
      raster RGBA
          |
     FFmpeg overlay
```

`resvg` is a static SVG renderer; animation comes from Automexia keyframes/FFmpeg transforms, not from SVG animation.

Fonts must be handled deliberately:

- built-in templates should reference fonts that Automexia is allowed to redistribute, or
- use a documented system-font fallback strategy
- project files should record the resolved font family/file identity when reproducibility matters

Do not silently substitute a dramatically different font without warning for a professional export.

---

# 60. Subtitle Input

Subtitles do not require transcription if a subtitle file already exists.

Support ingest of common text subtitle formats as appropriate, such as:

- SRT
- WebVTT

Optional `whisper.cpp` can generate subtitle timing/text when the transcription feature is installed.

Subtitle placement uses known caption boxes and face/safe-zone avoidance.

---

# 61. Effects and Transitions — V1 Philosophy

Automatic effects should solve editing problems, not decorate footage.

V1 effects:

- hard cuts
- subtle crop/punch-in changes
- short fades/dissolves only when an explicit rule/preset allows them
- basic title/lower-third fade or slide
- reframe motion

Not V1:

- whip transitions
- flash transitions
- automatic motion blur
- background segmentation
- cinematic masking
- complex compositing
- automatic speed ramps
- beat-synchronized editing

---

# 62. Punch-In Rules

Punch-ins primarily hide or soften talking-head jump cuts.

Possible base triggers:

- a silence-removal cut inside the same detected scene
- a long static shot exceeding a preset interval

Optional triggers only when their feature exists:

- sentence boundary from transcription
- speaker change from future speaker tracking

Rules:

- use small zoom changes by default
- enforce a minimum interval
- avoid zooming if the face would violate safe margins
- reset behavior at scene changes
- do not alternate zoom mechanically on every cut

---

# 63. Transition Rules

Default transition is a hard cut.

A clean preset may allow a short dissolve/fade only when:

- the segments represent a clear scene/section change
- the edit is not simply a silence-removal jump cut
- the transition does not damage speech/audio continuity

If confidence is low, use a hard cut.

---

# 64. Common Keyframe Representation

Keep the animation model small.

Initial properties:

```text
position.x
position.y
scale
opacity
crop
volume
speed
```

Initial interpolation:

- linear
- ease-in
- ease-out
- ease-in-out

This is enough for V1 reframe, punch-ins, graphic movement, fades, and volume transitions.

Spring physics, masks, blur animation, and complex curves are later features.

---

# 65. Platform Variants

One master edit should create multiple output compositions.

```text
                    Master edit
                        |
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
        16:9           9:16           4:5
          |             |             |
       YouTube        TikTok        Feed/Reel
```

Time edits and common audio corrections belong to the master edit.

Reframe and graphic placement are variant-specific because output geometry and safe zones differ.

---

# 66. Render Graph Ordering

A typical video branch should compile in roughly this order:

```text
source orientation / normalization
    -> timeline trims/concats
    -> shot-level color correction
    -> variant crop/reframe
    -> scale/output geometry
    -> graphic overlays
    -> output pixel/color format
    -> encoder
```

Audio branch:

```text
selected source audio
    -> source-level cleanup / RNNoise where enabled
    -> timeline cuts/assembly
    -> EQ / de-ess / compression
    -> loudness normalization / limiting
    -> output audio encoder
```

Exact ordering may be adjusted after quality testing, but it must be deterministic and documented.

---

# 67. Multi-Target Render Optimization

Default caching should focus on analysis metadata.

For rendering multiple variants, the compiler should prefer a **single FFmpeg process with shared decode/common filters and split branches** when practical:

```text
source decode
     |
common edits/color/audio
     |
   split
  /  |  \
16:9 9:16 4:5
 |    |    |
encode each target
```

This avoids creating large intermediate master files solely for caching.

If memory/encoder constraints make a combined graph unreliable, fall back to sequential target renders.

Rendered intermediate caching can be an opt-in optimization for repeated professional workflows, not the default.

---

# 68. Render Presets

Separate **edit presets** from **output presets**.

Edit preset:

```text
clean
social
podcast
```

Output target:

```text
youtube
tiktok
reel
feed
square
master
```

Output presets specify:

- width/height or aspect
- frame-rate behavior
- container preference
- encoder preference order
- quality mode
- audio codec preference
- pixel format
- color metadata policy
- safe zone

Do not bury editing behavior inside a codec preset.

---

# 69. Codec Selection

The output preset expresses preferences, not assumptions.

Example preference:

```text
H.264 target
    -> preferred available hardware H.264 encoder
    -> compatible software H.264 encoder
    -> clear unsupported-capability error
```

Professional master:

```text
ProRes target
    -> available compatible ProRes encoder
```

The extension must report the chosen encoder in the project/render report.

---

# 70. Output Collision and Atomicity

Rules:

- never overwrite an existing output by default
- require `--overwrite` or generate a new deterministic filename
- render to a temporary path first when possible
- atomically rename completed output into place
- remove incomplete files on failure unless `--keep-temp` is set

Project writes should also use write-temp-then-rename to avoid corrupting `.amxv` on crash.

---

# 71. Project Export vs Render

These are separate operations.

```text
video render
    -> final media

video export
    -> editable timeline/project interchange
```

Do not call a rendered MP4 an exported project.

---

# 72. V1 NLE Interchange Strategy

Use a neutral internal Automexia timeline and dedicated exporters.

Initial formats:

- **OTIO JSON (`.otio`)** — preferred structural interchange where the target supports it
- **Final Cut Pro 7 XML / legacy Final Cut Pro XML** — useful for Premiere-compatible XML workflows
- **CMX 3600 EDL** — simple universal fallback

Important terminology:

- legacy Final Cut Pro XML and modern Final Cut Pro `.fcpxml` are not the same format
- current Premiere documentation supports Final Cut Pro XML workflows but does not directly import modern Final Cut Pro X `.fcpxml` without conversion
- do not label a legacy XML exporter simply `fcpxml`

Suggested CLI names:

```text
--format otio
--format fcp7xml
--format edl
```

Modern Final Cut Pro `.fcpxml`, AAF, proprietary Premiere project files, and Resolve project files are later work.

---

# 73. NLE Target Mapping

V1 logical target mapping:

```text
--to resolve
    -> prefer OTIO after compatibility test
    -> export report

--to premiere
    -> prefer legacy Final Cut Pro XML
    -> EDL fallback for very simple timelines
    -> export report
```

Do not assume all NLE versions support the same features. Maintain automated interoperability fixtures for supported target versions.

---

# 74. Preserve / Translate / Bake

Every operation needs an export policy.

## Preserve

When the target format can represent the edit directly.

```text
clip cut -> clip cut
marker   -> marker
```

## Translate

When the target has a close equivalent.

```text
Automexia crossfade -> standard dissolve
```

## Bake

When an operation cannot be represented.

Examples:

- RNNoise/DeepFilterNet voice processing -> rendered audio stem
- complex unsupported visual processing -> optional rendered intermediate

By default, avoid baking video effects if that would unnecessarily reduce editability. Export an incompatibility report and let the user choose `--bake-effects` when needed.

---

# 75. Export Compatibility Report

Every NLE export should produce a human-readable and optionally JSON report.

Example:

```text
Premiere export

Preserved:
✓ 93 cuts
✓ 4 video tracks
✓ 2 audio tracks
✓ markers

Translated:
✓ 8 simple dissolves

Not represented:
⚠ automatic color correction on 11 shots

Baked:
✓ enhanced dialogue stem -> audio/dialogue-clean.wav
```

Never silently drop a meaningful edit.

---

# 76. EDL Limitations

EDL is intentionally a fallback format.

Use it only for simple timelines.

Potential limitations include:

- limited video/audio track complexity
- frame/timecode assumptions
- weak support for effects/graphics
- limited transition vocabulary
- poor representation of modern VFR workflows

If the project cannot be faithfully represented, refuse or export with prominent warnings rather than pretending the file is complete.

---

# 77. Source Relinking in Exports

Exporters should support a media reference policy:

```text
relative
absolute
copy/bundle (later or explicit)
```

Relative paths are preferred when source media lives under a project directory.

If source filenames/path characters are incompatible with a target interchange format, report the issue before writing the export.

---

# 78. Temp Files

Video processing will sometimes require temporary assets.

Examples:

- FFmpeg filter scripts
- rasterized graphics
- enhanced audio stem
- analysis PCM

Rules:

- use a dedicated request temp directory
- use collision-resistant names
- delete on success
- delete on failure unless debugging requests `--keep-temp`
- never write temp files next to source footage unless explicitly requested

---

# 79. Failure Recovery

A failed render should not discard successful analysis.

Persist or cache:

- probe metadata
- analysis results
- generated `.amxv`

On retry, reuse valid analysis cache and recompute only the failed compile/render stage.

A worker crash must leave enough state to explain which stage failed.

---

# 80. Logging

Use two log modes.

## Human

Concise progress:

```text
Analyzing audio       ✓
Detecting faces       ✓
Building 42 edits     ✓
Rendering TikTok      63%
```

## Diagnostic

Structured detailed logs containing:

- request ID
- stage
- command executable/arguments with secrets removed
- FFmpeg version
- selected encoder
- model versions
- cache hit/miss
- elapsed stage time
- error stderr snippets

Do not log secrets or arbitrary media content.

---

# 81. Performance Principles

The terminal itself must have zero video/ML idle cost.

For video:

- low-resolution analysis
- CPU-only tiny-model inference by default
- model lazy loading
- one shared decode/analysis pass
- analysis cache reuse
- full-resolution work only at render
- FFmpeg handles optimized codec/filter execution
- no Chromium
- no Python runtime
- no GPU ML provider unless measured useful

The largest normal resource consumer should usually be video decode/encode, not the default ML models.

---

# 82. Performance Benchmark Suite

Do not guess performance from architecture. Measure it.

Maintain benchmark fixtures for at least:

- 1080p 30 fps talking head
- 4K 30 fps talking head
- VFR phone recording
- long 60-minute podcast
- noisy voice recording
- two-face interview
- no-face landscape footage

Measure:

- analysis wall time
- peak worker RAM
- core idle RAM before/after worker
- render speed
- cache reuse speedup
- face detection throughput
- VAD throughput
- output A/V sync drift

Set numerical release budgets after collecting baseline data on representative low/mid/high hardware.

---

# 83. Correctness Test Suite

## Time math

Unit tests for:

- rational conversions
- VFR timestamps
- negative/start offsets
- stream time bases
- drop/non-drop timecode export where applicable

## Edit decisions

Golden tests for:

- silence ranges
- retained padding
- cut count
- reframe keyframes
- graphic anchor choice

## Audio

Check:

- no clipping
- target loudness tolerance
- no unexpected channel-layout change
- bounded A/V sync error

## Video

Check:

- aspect ratio
- crop inside source bounds
- rotation correctness
- color metadata
- no accidental HDR->SDR transform

## Project

Snapshot tests for `.amxv` serialization and migrations.

---

# 84. Media Regression Corpus

Automexia should maintain a legally distributable internal regression corpus containing short representative media clips.

Cases:

- clean speech
- low-level speech
- breathing pauses
- background fan noise
- background music
- clipped audio
- two speakers
- face enters/exits frame
- camera rotation metadata
- VFR phone capture
- SDR Rec.709
- HDR metadata case
- interlaced sample
- multiple audio streams

Each release should compare generated decisions against accepted baselines.

---

# 85. Cross-Platform Test Matrix

Primary practical targets can be defined by release planning, but architecture should support:

- Linux x86_64
- macOS Apple Silicon
- Windows x86_64

Additional targets such as Linux ARM64, Intel macOS, or Windows ARM can be added based on demand.

CI should test:

- extension protocol handshake
- path handling
- process cancellation
- Unicode filenames
- spaces in filenames
- FFmpeg discovery
- project-relative paths
- model loading

---

# 86. Privacy

Default video processing is local.

The base extension should not make network requests while processing a project.

A project file should not contain:

- credentials
- hidden telemetry identifiers
- uploaded URLs unless explicitly part of source metadata

Optional update/install operations can use network access through Automexia's extension manager.

---

# 87. Model Asset Management

Model packs should be installed by the extension manager, not downloaded ad hoc during an edit.

A model package contains:

- model binary
- manifest
- checksum
- license text/reference
- expected input shape/sample rate
- preprocessing version
- postprocessing version

Example:

```toml
id = "ultraface-rfb-320"
version = "1"
runtime = "onnx"
sha256 = "..."
input_width = 320
input_height = 240
license = "MIT"
```

If a model hash does not match the project snapshot, rerendering can still use existing edit decisions; recomputing analysis requires an explicit compatible model or migration.

---

# 88. Feature Pack Installation

Keep one video extension. Do not fragment it into many user-visible extensions.

Example management commands:

```bash
automexia ext install video
automexia ext feature add video transcription
automexia ext feature add video voice-quality
automexia ext feature remove video transcription
```

Normal editing commands remain unchanged.

---

# 89. V1 Presets

Keep built-in edit presets few and understandable.

## `clean`

- natural pause shortening
- conservative RNNoise
- spoken-word EQ/compression
- conservative color correction
- subtle jump-cut mitigation
- no automatic aspect conversion unless requested

## `social`

- tighter pauses
- stronger but bounded voice presence
- face-aware social reframing when target is vertical/square
- platform safe zones
- slightly more frequent punch-ins
- still conservative transitions

## `podcast`

- natural pauses by default
- strong focus on voice consistency
- minimal visual effects
- multi-person framing tries to preserve the group

Users can create custom presets rather than requiring dozens of built-ins.

---

# 90. V1 Deliverable

V1 should solve this job well:

> Take ordinary talking-head, course, podcast, or creator footage and produce a clean, polished, correctly framed output with one command.

## V1 editing

- media probe
- one shared low-resolution analysis pass
- smart silence shortening
- audio/video cut smoothing
- subtle jump-cut punch-ins

## V1 audio

- RNNoise
- high-pass/rumble cleanup
- conservative EQ
- de-essing
- compression
- loudness normalization
- limiter

## V1 video

- scene detection
- SDR-only conservative exposure/white-balance correction
- basic contrast correction
- UltraFace detection
- light face tracking

## V1 reframing

- 16:9
- 9:16
- 4:5
- 1:1
- custom aspect
- face-aware crop
- dead zone and smooth crop
- conservative no-face fallback
- platform safe zones

## V1 graphics

- logo
- title
- lower third
- existing subtitle input
- safe automatic placement
- fade/slide/simple scale animation

## V1 project/output

- `.amxv` JSON project
- dry run
- multi-target rendering
- logical H.264/H.265/ProRes/WebM/master presets subject to detected encoder support
- OTIO export
- legacy Final Cut Pro XML export for Premiere workflows
- CMX 3600 EDL fallback
- export compatibility report

---

# 91. V2 Candidates

Only add based on usage data and user demand.

Candidates:

- `whisper.cpp` transcription pack
- generated subtitles
- filler-word workflow
- sentence-aware punch-in triggers
- DeepFilterNet quality pack
- small object/person detector
- improved nature/B-roll reframing
- active-speaker tracking
- more robust graphic placement
- better export fidelity

---

# 92. V3 / Later Candidates

- advanced stabilization
- optical-flow tracking
- object-following callouts
- speed ramps
- beat-aware editing
- segmentation/background effects
- richer animation templates
- multi-camera workflows
- AAF
- modern Final Cut Pro `.fcpxml`
- advanced shot matching

None of these justify adding a GUI requirement.

---

# 93. Things Explicitly Excluded from the Default Path

```text
WhisperX
HyperFrames
Python
PyTorch
TensorFlow
Node.js
Chromium
large language models
AI agents
paid inference APIs
cloud media processing
OpenCV unless vision-advanced is installed
DeepFilterNet unless voice-quality is installed
whisper.cpp unless transcription is installed
large object/segmentation models
full NLE GUI/TUI
custom GPU compositor
```

---

# 94. Example Full User Flows

## Simple cleanup

```bash
video run recording.mp4
```

Produces:

```text
recording.amxv
recording.edited.mp4
```

## Social variants

```bash
video run recording.mp4 \
  --preset social \
  --to youtube,tiktok,reel
```

Produces:

```text
recording.amxv
output/youtube.mp4
output/tiktok.mp4
output/reel.mp4
```

## Inspect before rendering

```bash
video run recording.mp4 --dry-run
```

## Professional round trip

```bash
video run interview.mov --preset clean --no-render
video export interview.amxv --to resolve
```

## Master render

```bash
video render interview.amxv \
  --codec prores \
  --output interview-master.mov
```

---

# 95. Example Internal Pipeline for Social Run

```text
video run recording.mp4 --preset social --to youtube,tiktok,reel

1. ffprobe
2. validate streams / HDR / VFR / rotation / channel layout
3. resolve preset snapshot
4. compute analysis cache key
5. create low-resolution analysis streams
6. run VAD / audio statistics
7. run scene detection
8. run UltraFace + lightweight track association
9. compute shot luma/color statistics
10. build deterministic edit operations
11. write recording.amxv atomically
12. compile common master edit
13. build target-specific reframe tracks
14. place target-specific graphics
15. compile one multi-output FFmpeg graph when practical
16. render
17. validate outputs
18. write render report
19. clean temporary files
```

---

# 96. Post-Render Validation

Every render should also write a small render record into the `.amxv` project or a sidecar JSON containing the selected FFmpeg build, encoders, output hashes/paths, elapsed time, and warnings. This makes support and regression analysis substantially easier.

A successful FFmpeg exit is not enough.

Validate output with ffprobe:

- file exists and non-zero
- expected duration within tolerance
- expected resolution
- expected video/audio streams
- expected channel count
- expected frame-rate mode
- expected color metadata policy
- no gross A/V duration mismatch

For multi-target runs, one failed target should be reported distinctly from successful targets.

---

# 97. Source and Output Naming

Default output naming should be predictable.

Example:

```text
recording.mp4
recording.amxv
recording.clean.mp4
recording.youtube.mp4
recording.tiktok.mp4
recording.reel.mp4
```

A project can define a custom output directory.

Never overwrite the source filename.

---

# 98. Schema Migration

`.amxv` must use explicit schema versions.

On load:

```text
current version
    -> open

older supported version
    -> migrate in memory
    -> optionally save migrated copy

newer unsupported version
    -> fail safely with required version
```

Never partially parse a newer schema and guess the missing semantics.

Maintain migration tests with real historical fixtures.

---

# 99. Extension Protocol Versioning

The extension protocol and project schema are separate versions.

Example:

```text
Automexia core protocol: 1
Video extension:         1.4.0
.amxv schema:            2
Analysis pipeline:       3
```

Changing an algorithm does not necessarily require changing IPC or project schema.

---

# 100. Implementation Priority

## Automexia Core P0

- stable terminal
- process spawning
- minimal extension registry
- versioned JSONL worker IPC
- config/preset loading
- permissions metadata
- foreground/background jobs
- cancellation

## Video P0

- FFmpeg discovery/probe
- `.amxv` schema/time model
- VAD + silence decisions
- RNNoise/voice chain
- scene detection
- UltraFace
- basic reframe
- render compiler

## Video P1

- graphics placement
- multi-target single-graph optimization
- NLE exporters
- more robust cache/recovery

## Later

Everything in V2/V3 only after the P0/P1 pipeline is stable.

---

# 101. Architectural Acceptance Criteria

A design change should be rejected or challenged if it violates one of these tests:

1. **Does it add idle cost to Automexia core for a domain-specific feature?**
   If yes, move it into an extension.

2. **Does it require a GUI to access core functionality?**
   If yes, redesign it as commands/presets/project metadata.

3. **Does it add a heavy runtime for a feature algorithms can solve adequately?**
   If yes, prefer algorithms.

4. **Does a model make an edit decision directly?**
   If yes, move the decision into deterministic Rust logic.

5. **Does it require network access during normal video processing?**
   If yes, reject unless the user explicitly installed an online feature.

6. **Can the operation be represented non-destructively?**
   If yes, store metadata and defer baking.

7. **Can FFmpeg perform the full-resolution transform reliably?**
   If yes, compile to FFmpeg instead of implementing another renderer.

8. **Will the feature complicate the normal command surface?**
   If yes, prefer a preset/workflow/internal stage.

---

# 102. Final Architecture

```text
                         AUTOMEXIA
                    lightweight Rust core
                            |
           +----------------+----------------+
           |                                 |
      workflow/jobs                    extension IPC
                                             |
                                  automexia-video worker
                                             |
                 +---------------------------+------------------+
                 |                           |                  |
              FFmpeg                    Rust rules          Tiny ML
                 |                           |                  |
                 |                           |          Silero / UltraFace
                 |                           |                  |
                 +---------------------------+------------------+
                                             |
                                      .amxv edit project
                                             |
                            +----------------+----------------+
                            |                                 |
                         render                           NLE export
                            |
                 +----------+-----------+
                 |          |           |
               16:9        9:16        4:5
```

The permanent mental model is:

> **Automexia core = execute, orchestrate, extend.**
>
> **Video extension = analyze, decide, compile, render/export.**
>
> **Models observe. Rust decides. FFmpeg executes.**

---

# 103. Technical Reference Notes

These links are implementation references, not a substitute for a release-time license audit.

- FFmpeg legal/licensing: https://ffmpeg.org/legal.html
- FFmpeg time-base documentation: https://ffmpeg.org/doxygen/trunk/structAVCodecContext.html
- Silero VAD: https://github.com/snakers4/silero-vad
- UltraFace: https://github.com/Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB
- RNNoise: https://github.com/xiph/rnnoise
- ONNX Runtime: https://github.com/microsoft/onnxruntime
- ONNX Runtime execution providers: https://onnxruntime.ai/docs/execution-providers/
- DeepFilterNet: https://github.com/Rikorose/DeepFilterNet
- whisper.cpp: https://github.com/ggml-org/whisper.cpp
- resvg: https://github.com/linebender/resvg
- OpenTimelineIO: https://github.com/AcademySoftwareFoundation/OpenTimelineIO
- OTIO adapter documentation: https://github.com/AcademySoftwareFoundation/OpenTimelineIO/blob/main/docs/tutorials/adapters.md
- Adobe Premiere Final Cut Pro XML export documentation (2026): https://helpx.adobe.com/premiere/desktop/render-and-export/export-files/export-a-project-as-a-final-cut-pro-xml-file.html
- Adobe Premiere EDL documentation (2026): https://helpx.adobe.com/premiere/desktop/render-and-export/export-files/export-a-project-as-an-edl-file.html
- Blackmagic Design support/manuals: https://www.blackmagicdesign.com/support

---

# 104. Final Guiding Principle

Automexia should become powerful by allowing users to install capability, **not by making every installation carry every capability**.

```text
Install Automexia
    -> small fast Rust terminal

Install Video
    -> local automated post-production

Add transcription only if needed
    -> whisper.cpp pack

Add stronger voice cleanup only if needed
    -> DeepFilterNet pack

Add advanced vision only if needed
    -> object/vision pack
```

A user who never edits video should pay **zero** video or ML cost.

A user who installs the video extension should still get a simple experience centered on:

```bash
video run input.mp4
```

That simplicity is a product requirement, not merely a CLI preference.

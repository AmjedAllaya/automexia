
# Video Automation

**Type:** Research/Request-for-Discussion note — not a project ADR
**Repository integration note:** Future product discovery only.

This file preserves proposal reasoning without claiming an ADR number or acceptance status.

---


## Candidate direction

`automexia-video` is a first-party domain extension built around:

```text
Models observe.
Rust decides.
Automexia authorizes.
FFmpeg executes.
Humans review uncertainty.
```

A future extension must use the existing application-owned external-tool authority and accepted capability policy; it may not assume a separate ProcessBroker or Policy Engine already exists.

## Runtime

Keep managed and compatible system FFmpeg/ffprobe as competing candidates until a future distribution ADR settles license, provenance, platform, size, signing, update, and rollback requirements.

## Process authority

FFmpeg/ffprobe would be launched from typed `MediaProcessPlan`s through the existing application-owned `ExternalToolRunner` authority. The extension receives no general shell or arbitrary subprocess authority.

## Project/cache model

`.amxv` is the non-destructive source of edit intent. Large observations use a separate disposable/rebuildable analysis cache.

## Models

Silero VAD is an observation source, not an edit-decision authority.

Face detection is behind `FaceDetector`; YuNet is the primary candidate and UltraFace remains a benchmark/legacy alternative.

Speech enhancement is behind `SpeechEnhancer`. RNNoise is optional/legacy, not mandatory. WebRTC AudioProcessing is an active reference/fallback. DeepFilterNet is a quality candidate only after exact pretrained-weight redistribution approval.

## Quality/review

Automated decisions are `AutoApply`, `NeedsReview`, or `Ignore`. Medium-confidence operations are reviewable through keyboard-first overlays and proxy clips.

## HDR/timing

HDR support is operation-specific. Timing accounts for rational PTS/time bases plus priming, pre-skip, edit-list, start-offset, negative-timestamp, and DTS/PTS behavior.

## Testing

If video work is accepted, a VideoLab evidence harness is required before smart-feature expansion and would own corpus regression, hostile-media tests, failure injection, audio/face bake-offs, output validation, resource limits, quality metrics, and real NLE import fixtures.

## Delivery strategy

Implement vertical slices:

```text
media correctness
-> silence
-> voice
-> face/reframe
-> social variants
-> review
-> graphics
-> NLE export
```

Do not implement all smart features concurrently.

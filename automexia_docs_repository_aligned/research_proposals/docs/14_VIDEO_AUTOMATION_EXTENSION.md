
# Video Automation — Future Product Discovery

**Current repository:** no implementation.
**Roadmap status:** defer platform integration until terminal/provider/security/release hardening is substantially complete.

The revised design remains a useful future proposal:

```text
models observe
Rust decides
Automexia authorizes
media tool executes
human reviews uncertainty
```

No large LLM, paid inference API, or cloud AI service is required.

If revisited:
- CLI must be complete;
- renderer-native review UI may be optional/first-class and keyboard-complete;
- media execution needs explicit process/sandbox authority;
- model/code/weight licenses are separate evidence;
- FFmpeg distribution policy requires codec/license/signing/security review;
- human review/proxies remain important;
- media timing/HDR/backpressure/corpus requirements remain.

Do not add FFmpeg/ONNX/model dependencies to the current terminal simply to preserve this future design.

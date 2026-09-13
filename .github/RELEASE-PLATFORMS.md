# Release platforms

| Artifact | Native build runner | Final verification |
|---|---|---|
| Windows x86_64 MSVC | `windows-2025` | signed MSI/ZIP smoke during packaging |
| Windows ARM64 MSVC | `windows-11-arm` | final signed MSI/ZIP on `windows-11-arm` |
| Linux x86_64 GNU | `ubuntu-22.04` | packaging/install on 22.04 + final package execution on 24.04 |
| Linux ARM64 GNU | `ubuntu-22.04-arm` | packaging/install on 22.04 ARM + final package execution on 24.04 ARM |
| macOS Intel | `macos-26-intel` | final notarized universal DMG on Intel |
| macOS Apple Silicon | `macos-26` | final notarized universal DMG on Apple Silicon |

Linux support is an explicit GNU/Linux ABI/runtime contract, not a claim that every possible Linux distribution is identical. The production ceiling is GLIBC 2.35; native desktop/audio/runtime shared libraries are still required by the application/package.

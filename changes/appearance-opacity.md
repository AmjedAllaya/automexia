# Readable appearance and continuous opacity

Correct Windows' default renderer selection to use its compiled GPU support.
Use WGPU's DirectComposition presentation and consistent premultiplied alpha,
including intermediate opacity and resize. Initialize backend families separately
and reserve a bounded Windows GUI stack for bundled HLSL shader compilation.
Explicit CPU rendering stays opaque
and explains the native-transparency limitation without erasing saved choices.
Unavailable macOS Liquid Glass retains the configured opacity during fallback.
Opaque defaults and text remain intact. Header contrast follows custom surfaces;
header/footer opacity resets preserve RGB. Add catalog-wide boundary, persistence
and reset checks, contrast/alpha regressions and an isolated native compositor
oracle with dark and light backdrops. No preference schema or dependency changes.

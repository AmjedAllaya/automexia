use crate::sugarloaf::{Colorspace, SugarloafWindow, SugarloafWindowSize};
use crate::SugarloafRenderer;

pub struct WgpuContext<'a> {
    pub device: wgpu::Device,
    pub surface: wgpu::Surface<'a>,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    surface_color_space: wgpu::SurfaceColorSpace,
    alpha_mode: wgpu::CompositeAlphaMode,
    pub adapter_info: wgpu::AdapterInfo,
    surface_caps: wgpu::SurfaceCapabilities,
    pub size: SugarloafWindowSize,
    pub scale: f32,
    pub supports_f16: bool,
    pub colorspace: Colorspace,
    pub max_texture_dimension_2d: u32,
}

impl<'a> WgpuContext<'a> {
    pub fn new(
        sugarloaf_window: SugarloafWindow,
        renderer_config: SugarloafRenderer,
        wgpu_backend: wgpu::Backends,
    ) -> WgpuContext<'a> {
        let size = sugarloaf_window.size;
        let scale = sugarloaf_window.scale;

        // The backend can be configured using the `WGPU_BACKEND`
        // environment variable. Otherwise the configured backends are used.
        // Comma-separated values can restrict fallback. Supported names include:
        // - `vulkan`
        // - `metal`
        // - `dx12`
        // - `gl`
        // - `webgpu`
        let backend = wgpu::Backends::from_env().unwrap_or(wgpu_backend);
        let (surface, adapter) = backend_attempts(backend)
            .into_iter()
            .flatten()
            .find_map(|attempt| {
                let result = create_surface_adapter(&sugarloaf_window, attempt);
                if result.is_none() {
                    tracing::warn!(
                        "Requested graphics backend {attempt:?} is unavailable"
                    );
                }
                result
            })
            .expect("No compatible renderer surface and adapter");

        let adapter_info = adapter.get_info();
        tracing::info!("Selected adapter: {:?}", adapter_info);

        let surface_caps = surface.get_capabilities(&adapter);

        #[cfg(target_os = "macos")]
        let format = get_macos_texture_format(renderer_config.colorspace);
        #[cfg(not(target_os = "macos"))]
        let format = find_best_texture_format(
            surface_caps.formats.as_slice(),
            renderer_config.colorspace,
        );
        let surface_color_space = output_surface_color_space();

        let (device, queue) = {
            {
                if let Ok(result) = futures::executor::block_on(
                    adapter.request_device(&wgpu::DeviceDescriptor::default()),
                ) {
                    (result.0, result.1)
                } else {
                    // These downlevel limits will allow the code to run on all possible hardware
                    let result = futures::executor::block_on(adapter.request_device(
                        &wgpu::DeviceDescriptor {
                            memory_hints: wgpu::MemoryHints::Performance,
                            label: None,
                            required_features: wgpu::Features::empty(),
                            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                            ..Default::default()
                        },
                    ))
                    .expect("Request device");
                    (result.0, result.1)
                }
            }
        };

        let alpha_mode = composition_alpha_mode(&surface_caps.alpha_modes);

        // Configure view formats for wide color gamut support
        let view_formats = match renderer_config.colorspace {
            Colorspace::DisplayP3 | Colorspace::Rec2020 => {
                // For wide color gamut, we may want to support additional view formats
                // This allows the surface to be viewed in different formats
                vec![format]
            }
            Colorspace::Srgb => {
                vec![]
            }
        };

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: Self::get_texture_usage(&surface_caps),
                format,
                width: size.width as u32,
                height: size.height as u32,
                view_formats,
                alpha_mode,
                color_space: surface_color_space,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
            },
        );

        let max_texture_dimension_2d = device.limits().max_texture_dimension_2d;

        tracing::info!("Configured colorspace: {:?}", renderer_config.colorspace);
        tracing::info!("Surface format: {:?}", format);

        WgpuContext {
            device,
            queue,
            surface,
            format,
            surface_color_space,
            alpha_mode,
            size: SugarloafWindowSize {
                width: size.width,
                height: size.height,
            },
            scale,
            adapter_info,
            surface_caps,
            // Always disabled on webgpu
            supports_f16: false,
            colorspace: renderer_config.colorspace,
            max_texture_dimension_2d,
        }
    }

    pub fn supports_transparency(&self) -> bool {
        self.alpha_mode == wgpu::CompositeAlphaMode::PreMultiplied
    }

    fn get_texture_usage(caps: &wgpu::SurfaceCapabilities) -> wgpu::TextureUsages {
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;

        // COPY_DST and COPY_SRC are required for FiltersBrush
        // But some backends like OpenGL might not support COPY_DST and COPY_SRC
        // https://github.com/emilk/egui/pull/3078

        if caps.usages.contains(wgpu::TextureUsages::COPY_DST) {
            usage |= wgpu::TextureUsages::COPY_DST;
        }

        if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) {
            usage |= wgpu::TextureUsages::COPY_SRC;
        }

        usage
    }

    pub fn max_texture_dimension_2d(&self) -> u32 {
        self.max_texture_dimension_2d
    }

    #[inline]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.size.width = width as f32;
        self.size.height = height as f32;

        // Configure view formats for wide color gamut support
        let view_formats = match self.colorspace {
            Colorspace::DisplayP3 | Colorspace::Rec2020 => {
                vec![self.format]
            }
            Colorspace::Srgb => {
                vec![]
            }
        };

        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: Self::get_texture_usage(&self.surface_caps),
                format: self.format,
                width,
                height,
                view_formats,
                alpha_mode: self.alpha_mode,
                color_space: self.surface_color_space,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
            },
        );
    }

    #[inline]
    pub fn surface_caps(&self) -> &wgpu::SurfaceCapabilities {
        &self.surface_caps
    }

    #[inline]
    pub fn supports_f16(&self) -> bool {
        self.supports_f16
    }

    #[inline]
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    pub fn get_optimal_texture_format(&self) -> wgpu::TextureFormat {
        // wgpu always uses f32 formats, not f16
        wgpu::TextureFormat::Rgba8Unorm
    }

    pub fn get_optimal_texture_sample_type(&self) -> wgpu::TextureSampleType {
        // wgpu uses Rgba8Unorm (f32) with Float sample type and filtering
        wgpu::TextureSampleType::Float { filterable: true }
    }

    pub fn convert_rgba8_to_optimal_format(&self, rgba8_data: &[u8]) -> Vec<u8> {
        // wgpu always uses f32 (Rgba8Unorm), no f16 conversion needed
        rgba8_data.to_vec()
    }
}

#[inline]
#[cfg(not(target_os = "macos"))]
fn find_best_texture_format(
    formats: &[wgpu::TextureFormat],
    colorspace: Colorspace,
) -> wgpu::TextureFormat {
    let mut format: wgpu::TextureFormat = formats.first().unwrap().to_owned();

    // TODO: Fix formats with signs
    // FIXME: On Nvidia GPUs usage Rgba16Float texture format causes driver to enable HDR.
    // Reason for this is currently output color space is poorly defined in wgpu and
    // anything other than Srgb texture formats can cause undeterministic output color
    // space selection which also causes colors to mismatch. Optionally we can whitelist
    // only the Srgb texture formats for now until output color space selection lands in wgpu. See #205
    // TODO: use output color format for the CanvasConfiguration when it lands on the wgpu
    #[cfg(windows)]
    let unsupported_formats = [
        wgpu::TextureFormat::Rgba8Snorm,
        wgpu::TextureFormat::Rgba16Float,
    ];

    // not reproduce-able on mac
    #[cfg(not(windows))]
    let unsupported_formats = [
        wgpu::TextureFormat::Rgba8Snorm,
        // Features::TEXTURE_FORMAT_16BIT_NORM must be enabled to use these texture format.
        wgpu::TextureFormat::R16Unorm,
        wgpu::TextureFormat::R16Snorm,
    ];

    // Bgra8Unorm is the most widely supported and guaranteed format in wgpu
    // Prefer it explicitly if available
    if formats.contains(&wgpu::TextureFormat::Bgra8Unorm) {
        format = wgpu::TextureFormat::Bgra8Unorm;
        tracing::info!(
            "Sugarloaf selected format: {format:?} from {:?} for colorspace {:?}",
            formats,
            colorspace
        );
        return format;
    }

    let filtered_formats: Vec<wgpu::TextureFormat> = formats
        .iter()
        .copied()
        .filter(|&x| {
            // On non-macOS platforms, always avoid sRGB formats
            // This maintains compatibility with existing Linux/Windows color handling
            !wgpu::TextureFormat::is_srgb(&x) && !unsupported_formats.contains(&x)
        })
        .collect();

    // If no compatible formats found, fall back to any non-unsupported format
    let final_formats = if filtered_formats.is_empty() {
        formats
            .iter()
            .copied()
            .filter(|&x| !unsupported_formats.contains(&x))
            .collect()
    } else {
        filtered_formats
    };

    if !final_formats.is_empty() {
        final_formats.first().unwrap().clone_into(&mut format);
    }

    tracing::info!(
        "Sugarloaf selected format: {format:?} from {:?} for colorspace {:?}",
        formats,
        colorspace
    );

    format
}

#[inline]
#[cfg(target_os = "macos")]
fn get_macos_texture_format(colorspace: Colorspace) -> wgpu::TextureFormat {
    match colorspace {
        Colorspace::Srgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        Colorspace::DisplayP3 | Colorspace::Rec2020 => wgpu::TextureFormat::Bgra8Unorm,
    }
}

// Every draw pipeline accumulates premultiplied source-over color. Requesting
// PostMultiplied makes the compositor multiply it again, darkening edges/text.
fn composition_alpha_mode(
    supported: &[wgpu::CompositeAlphaMode],
) -> wgpu::CompositeAlphaMode {
    use wgpu::CompositeAlphaMode::*;
    // Non-premultiplied-only surfaces receive an opaque clear, so source-over
    // still produces alpha 1 everywhere. Never request an unsupported mode.
    [PreMultiplied, Opaque, Inherit, PostMultiplied, Auto]
        .into_iter()
        .find(|mode| supported.contains(mode))
        .unwrap_or(Opaque)
}

// Creating an unused WGL surface sets an opaque HWND pixel format and breaks
// DirectComposition alpha. Create one backend family at a time, preferring the
// Windows compositor, then falling back only if that family is unavailable.
fn backend_attempts(requested: wgpu::Backends) -> [Option<wgpu::Backends>; 2] {
    if cfg!(windows) && requested.contains(wgpu::Backends::DX12) {
        let fallback = requested - wgpu::Backends::DX12;
        [
            Some(wgpu::Backends::DX12),
            (!fallback.is_empty()).then_some(fallback),
        ]
    } else {
        [Some(requested), None]
    }
}

fn create_surface_adapter<'a>(
    window: &SugarloafWindow,
    backends: wgpu::Backends,
) -> Option<(wgpu::Surface<'a>, wgpu::Adapter)> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    #[cfg(windows)]
    {
        descriptor.backend_options.dx12.presentation_system =
            wgpu::Dx12SwapchainKind::DxgiFromVisual;
    }
    let instance = wgpu::Instance::new(descriptor);
    // The surface owns this handle wrapper; the host retains the native window
    // for the full Sugarloaf lifetime, exactly as on the original single path.
    let surface = instance
        .create_surface(SugarloafWindow {
            handle: window.handle,
            display: window.display,
            size: window.size,
            scale: window.scale,
        })
        .ok()?;
    let adapter = futures::executor::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        },
    ))
    .ok()?;
    Some((surface, adapter))
}

#[inline]
fn output_surface_color_space() -> wgpu::SurfaceColorSpace {
    #[cfg(windows)]
    {
        // Keep Windows in a deterministic SDR path across borderless
        // fullscreen transitions. `Srgb` describes the encoded output while
        // the selected non-*Srgb BGRA format stores already-encoded values.
        wgpu::SurfaceColorSpace::Srgb
    }
    #[cfg(not(windows))]
    {
        wgpu::SurfaceColorSpace::Auto
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn backend_attempts_preserve_explicit_choices_and_bound_fallback() {
        use wgpu::Backends as B;
        for explicit in [B::DX12, B::VULKAN, B::GL] {
            assert_eq!(super::backend_attempts(explicit), [Some(explicit), None]);
        }
        let all = super::backend_attempts(B::all());
        if cfg!(windows) {
            assert_eq!(all, [Some(B::DX12), Some(B::all() - B::DX12)]);
        } else {
            assert_eq!(all, [Some(B::all()), None]);
        }
    }

    #[test]
    fn source_over_prefers_premultiplied_and_falls_back_only_to_supported_modes() {
        use wgpu::CompositeAlphaMode::*;
        assert_eq!(
            super::composition_alpha_mode(&[Opaque, PostMultiplied, PreMultiplied]),
            PreMultiplied
        );
        assert_eq!(
            super::composition_alpha_mode(&[Opaque, PostMultiplied]),
            Opaque
        );
        assert_eq!(super::composition_alpha_mode(&[Opaque]), Opaque);
        assert_eq!(super::composition_alpha_mode(&[Inherit]), Inherit);
        assert_eq!(
            super::composition_alpha_mode(&[PostMultiplied]),
            PostMultiplied
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_output_is_explicit_sdr_srgb() {
        assert_eq!(
            super::output_surface_color_space(),
            wgpu::SurfaceColorSpace::Srgb
        );
    }
}

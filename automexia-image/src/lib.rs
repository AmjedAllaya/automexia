//! Renderer-neutral, resource-bounded local image decoding.
//!
//! Terminal output is untrusted. Only validated local raster candidates are
//! accepted. Final-path links are not followed, dimensions are checked before
//! full decode, and cached thumbnails are invalidated by file version.

use image_rs::{ImageFormat, ImageReader};
use std::collections::VecDeque;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime};

pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_DECODED_PIXELS: u64 = 16_777_216;
pub const MAX_DECODE_ALLOCATION_BYTES: u64 = 96 * 1024 * 1024;
pub const MAX_DECODE_DIMENSION: u32 = 4_096;
pub const MAX_UPLOAD_WIDTH: u32 = 1_280;
pub const MAX_UPLOAD_HEIGHT: u32 = 960;
pub const THUMBNAIL_CACHE_BYTES: usize = 32 * 1024 * 1024;
pub const THUMBNAIL_CACHE_ENTRIES: usize = 16;

const EXTENSIONS: &[&str] = &[
    "bmp", "gif", "ico", "jpeg", "jpg", "pbm", "pgm", "png", "pnm", "ppm", "tif", "tiff",
    "webp",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageCandidate {
    pub text: String,
    pub cwd: Option<PathBuf>,
    pub wsl_distro: Option<String>,
}

impl ImageCandidate {
    pub fn new(
        text: impl Into<String>,
        cwd: Option<PathBuf>,
        wsl_distro: Option<String>,
    ) -> Option<Self> {
        let text = trim_path_token(&text.into()).to_string();
        if is_non_local_token(&text)
            || text.chars().any(char::is_control)
            || !has_supported_extension(&text)
        {
            return None;
        }
        Some(Self {
            text,
            cwd,
            wsl_distro,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImagePreviewError {
    NotLocal,
    NotFound,
    NotRegularFile,
    Symlink,
    TooLarge,
    ChangedDuringRead,
    UnsupportedFormat,
    DecodeFailed,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ThumbnailCacheKey {
    path: PathBuf,
    file_bytes: u64,
    modified: SystemTime,
}

#[derive(Debug)]
pub struct DecodedThumbnail {
    pub render_id: u64,
    pub decoded_at: Instant,
    pub width: u32,
    pub height: u32,
    pub original_width: u32,
    pub original_height: u32,
    pub file_bytes: u64,
    pub rgba: Arc<[u8]>,
}

impl DecodedThumbnail {
    pub fn allocation_bytes(&self) -> usize {
        self.rgba.len()
    }
}

#[derive(Debug)]
pub struct DecodeOutcome {
    pub path: PathBuf,
    pub thumbnail: Arc<DecodedThumbnail>,
    pub reused: bool,
}

#[derive(Debug)]
struct CacheEntry {
    key: ThumbnailCacheKey,
    thumbnail: Arc<DecodedThumbnail>,
}

#[derive(Debug)]
pub struct ThumbnailCache {
    max_bytes: usize,
    max_entries: usize,
    used_bytes: usize,
    entries: VecDeque<CacheEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThumbnailCacheStats {
    pub entries: usize,
    pub used_bytes: usize,
    pub max_entries: usize,
    pub max_bytes: usize,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new(THUMBNAIL_CACHE_BYTES, THUMBNAIL_CACHE_ENTRIES)
    }
}

impl ThumbnailCache {
    pub fn new(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            max_bytes: max_bytes.max(1),
            max_entries: max_entries.max(1),
            used_bytes: 0,
            entries: VecDeque::new(),
        }
    }

    pub fn stats(&self) -> ThumbnailCacheStats {
        ThumbnailCacheStats {
            entries: self.entries.len(),
            used_bytes: self.used_bytes,
            max_entries: self.max_entries,
            max_bytes: self.max_bytes,
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    #[cfg(test)]
    fn clear(&mut self) {
        self.entries.clear();
        self.used_bytes = 0;
    }

    fn get(&mut self, key: &ThumbnailCacheKey) -> Option<Arc<DecodedThumbnail>> {
        let index = self.entries.iter().position(|entry| &entry.key == key)?;
        let entry = self.entries.remove(index)?;
        let thumbnail = Arc::clone(&entry.thumbnail);
        self.entries.push_back(entry);
        Some(thumbnail)
    }

    fn insert(&mut self, key: ThumbnailCacheKey, thumbnail: Arc<DecodedThumbnail>) {
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            if let Some(previous) = self.entries.remove(index) {
                self.used_bytes = self
                    .used_bytes
                    .saturating_sub(previous.thumbnail.allocation_bytes());
            }
        }

        let bytes = thumbnail.allocation_bytes();
        if bytes > self.max_bytes {
            return;
        }
        while self.entries.len() >= self.max_entries
            || self.used_bytes.saturating_add(bytes) > self.max_bytes
        {
            let Some(evicted) = self.entries.pop_front() else {
                break;
            };
            self.used_bytes = self
                .used_bytes
                .saturating_sub(evicted.thumbnail.allocation_bytes());
        }
        self.used_bytes = self.used_bytes.saturating_add(bytes);
        self.entries.push_back(CacheEntry { key, thumbnail });
    }
}

pub fn decode_candidate(
    candidate: &ImageCandidate,
    cache: &mut ThumbnailCache,
) -> Result<DecodeOutcome, ImagePreviewError> {
    let path = resolve_candidate_path(candidate)?;
    let (mut file, before) = open_regular_file_without_links(&path)?;
    if before.len() == 0 || before.len() > MAX_FILE_BYTES {
        return Err(ImagePreviewError::TooLarge);
    }

    let before_key = cache_key(&path, &before)?;
    if let Some(thumbnail) = cache.get(&before_key) {
        return Ok(DecodeOutcome {
            path,
            thumbnail,
            reused: true,
        });
    }

    let expected_len = before.len();
    let mut bytes = Vec::with_capacity(expected_len as usize);
    (&mut file)
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ImagePreviewError::DecodeFailed)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ImagePreviewError::TooLarge);
    }

    let after = file
        .metadata()
        .map_err(|_| ImagePreviewError::ChangedDuringRead)?;
    let after_key = cache_key(&path, &after)?;
    if bytes.len() as u64 != expected_len || before_key != after_key {
        return Err(ImagePreviewError::ChangedDuringRead);
    }

    let thumbnail = Arc::new(decode_bounded_bytes(&bytes)?);
    cache.insert(before_key.clone(), Arc::clone(&thumbnail));
    Ok(DecodeOutcome {
        path,
        thumbnail,
        reused: false,
    })
}

pub fn decode_bounded_bytes(bytes: &[u8]) -> Result<DecodedThumbnail, ImagePreviewError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ImagePreviewError::TooLarge);
    }
    let file_bytes = bytes.len() as u64;
    let format = detected_format(bytes)?;
    let (original_width, original_height) =
        ImageReader::with_format(Cursor::new(bytes), format)
            .into_dimensions()
            .map_err(|_| ImagePreviewError::DecodeFailed)?;
    let pixels = u64::from(original_width).saturating_mul(u64::from(original_height));
    if original_width == 0
        || original_height == 0
        || original_width > MAX_DECODE_DIMENSION
        || original_height > MAX_DECODE_DIMENSION
        || pixels > MAX_DECODED_PIXELS
        || pixels.saturating_mul(4) > MAX_DECODE_ALLOCATION_BYTES
    {
        return Err(ImagePreviewError::TooLarge);
    }

    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image_rs::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOCATION_BYTES);
    reader.limits(limits);
    let image = reader
        .decode()
        .map_err(|_| ImagePreviewError::DecodeFailed)?;
    let image =
        if original_width > MAX_UPLOAD_WIDTH || original_height > MAX_UPLOAD_HEIGHT {
            image.thumbnail(MAX_UPLOAD_WIDTH, MAX_UPLOAD_HEIGHT)
        } else {
            image
        }
        .into_rgba8();
    let width = image.width();
    let height = image.height();
    let expected_rgba_bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(ImagePreviewError::TooLarge)?;
    let rgba = image.into_raw();
    if rgba.len() != expected_rgba_bytes {
        return Err(ImagePreviewError::DecodeFailed);
    }

    static NEXT_RENDER_ID: AtomicU64 = AtomicU64::new(1);

    Ok(DecodedThumbnail {
        render_id: NEXT_RENDER_ID.fetch_add(1, Ordering::Relaxed),
        decoded_at: Instant::now(),
        width,
        height,
        original_width,
        original_height,
        file_bytes,
        rgba: Arc::from(rgba),
    })
}

fn detected_format(bytes: &[u8]) -> Result<ImageFormat, ImagePreviewError> {
    let format = image_rs::guess_format(bytes)
        .map_err(|_| ImagePreviewError::UnsupportedFormat)?;
    matches!(
        format,
        ImageFormat::Png
            | ImageFormat::Jpeg
            | ImageFormat::Gif
            | ImageFormat::WebP
            | ImageFormat::Bmp
            | ImageFormat::Ico
            | ImageFormat::Pnm
            | ImageFormat::Tiff
    )
    .then_some(format)
    .ok_or(ImagePreviewError::UnsupportedFormat)
}

fn cache_key(
    path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<ThumbnailCacheKey, ImagePreviewError> {
    Ok(ThumbnailCacheKey {
        path: path.to_path_buf(),
        file_bytes: metadata.len(),
        modified: metadata
            .modified()
            .map_err(|_| ImagePreviewError::ChangedDuringRead)?,
    })
}

fn open_regular_file_without_links(
    path: &Path,
) -> Result<(std::fs::File, std::fs::Metadata), ImagePreviewError> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| ImagePreviewError::NotFound)?;
    if metadata.file_type().is_symlink() {
        return Err(ImagePreviewError::Symlink);
    }
    if !metadata.is_file() {
        return Err(ImagePreviewError::NotRegularFile);
    }

    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        options.custom_flags(0x0020_0000);
    }
    let file = options
        .open(path)
        .map_err(|_| ImagePreviewError::NotFound)?;
    let opened_metadata = file.metadata().map_err(|_| ImagePreviewError::NotFound)?;
    if opened_metadata.file_type().is_symlink() {
        return Err(ImagePreviewError::Symlink);
    }
    if !opened_metadata.is_file() {
        return Err(ImagePreviewError::NotRegularFile);
    }
    Ok((file, opened_metadata))
}

fn is_non_local_token(token: &str) -> bool {
    token.contains("://")
        || token
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
}

fn resolve_candidate_path(
    candidate: &ImageCandidate,
) -> Result<PathBuf, ImagePreviewError> {
    let token = trim_path_token(&candidate.text);
    if is_non_local_token(token) || token.chars().any(char::is_control) {
        return Err(ImagePreviewError::NotLocal);
    }
    #[cfg(windows)]
    if token.starts_with("\\\\") || token.starts_with("//") {
        return Err(ImagePreviewError::NotLocal);
    }

    let raw = PathBuf::from(token);
    let path = if raw.is_absolute() {
        raw
    } else {
        candidate
            .cwd
            .as_ref()
            .ok_or(ImagePreviewError::NotLocal)?
            .join(raw)
    };
    #[cfg(windows)]
    let path =
        if candidate.wsl_distro.is_some() || path.to_string_lossy().starts_with('/') {
            map_wsl_path(&path, candidate.wsl_distro.as_deref())?
        } else {
            path
        };
    Ok(path)
}

#[cfg(windows)]
fn map_wsl_path(path: &Path, distro: Option<&str>) -> Result<PathBuf, ImagePreviewError> {
    let text = path.to_string_lossy().replace('\\', "/");
    if let Some(rest) = text.strip_prefix("/mnt/") {
        let mut parts = rest.splitn(2, '/');
        let drive = parts.next().unwrap_or_default();
        if drive.len() == 1 && drive.as_bytes()[0].is_ascii_alphabetic() {
            let tail = parts.next().unwrap_or_default().replace('/', "\\");
            return Ok(PathBuf::from(format!(
                "{}:\\{}",
                drive.to_ascii_uppercase(),
                tail
            )));
        }
    }
    let distro = distro
        .filter(|value| {
            !value.is_empty()
                && value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                })
        })
        .ok_or(ImagePreviewError::NotLocal)?;
    Ok(PathBuf::from(format!(
        "\\\\wsl.localhost\\{}\\{}",
        distro,
        text.trim_start_matches('/').replace('/', "\\")
    )))
}

pub fn has_supported_extension(text: &str) -> bool {
    Path::new(trim_path_token(text))
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            EXTENSIONS
                .iter()
                .any(|supported| ext.eq_ignore_ascii_case(supported))
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImagePathToken {
    pub text: String,
    /// Inclusive character offset in the rendered terminal row.
    pub start: usize,
    /// Exclusive character offset in the rendered terminal row.
    pub end: usize,
}

/// Find every local raster-looking path in one rendered terminal row.
///
/// This function is deliberately IO-free. It recognizes quoted paths with
/// spaces and the private-use glyphs emitted by the Automexia `ls` formatters,
/// but leaves existence and file-safety validation to the bounded decoder.
pub fn image_path_tokens_in_line(line: &str) -> Vec<ImagePathToken> {
    let chars = line.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index].is_whitespace() || matches!(chars[index], '|' | '<' | '>') {
            index += 1;
            continue;
        }

        if matches!(chars[index], '\'' | '"' | '\u{60}') {
            let quote = chars[index];
            let start = index;
            index += 1;
            let content_start = index;
            while index < chars.len() && chars[index] != quote {
                index += 1;
            }
            if index < chars.len() {
                let candidate = chars[content_start..index].iter().collect::<String>();
                if has_supported_extension(&candidate) && !is_non_local_token(&candidate)
                {
                    tokens.push(ImagePathToken {
                        text: candidate,
                        start,
                        end: index + 1,
                    });
                }
                index += 1;
                continue;
            }
            index = start;
        }

        let start = index;
        while index < chars.len()
            && !chars[index].is_whitespace()
            && !matches!(chars[index], '|' | '<' | '>')
        {
            index += 1;
        }
        let raw = chars[start..index].iter().collect::<String>();
        let candidate = strip_listing_icon(trim_path_token(&raw));
        if has_supported_extension(candidate) && !is_non_local_token(candidate) {
            tokens.push(ImagePathToken {
                text: candidate.to_string(),
                start,
                end: index,
            });
        }
    }

    tokens
}

pub fn path_token_at_line(line: &str, column: usize) -> Option<String> {
    image_path_tokens_in_line(line)
        .into_iter()
        .find(|token| column >= token.start && column < token.end)
        .map(|token| token.text)
}

fn strip_listing_icon(text: &str) -> &str {
    text.trim_start_matches(|character: char| {
        let codepoint = character as u32;
        (0xE000..=0xF8FF).contains(&codepoint)
            || (0xF0000..=0xFFFFD).contains(&codepoint)
            || (0x100000..=0x10FFFD).contains(&codepoint)
            || (!character.is_alphanumeric()
                && !matches!(character, '.' | '_' | '-' | '~' | '/' | '\\'))
    })
}

fn trim_path_token(text: &str) -> &str {
    text.trim().trim_matches(|character| {
        matches!(
            character,
            '\'' | '"' | '\u{60}' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(path: &Path) -> ImageCandidate {
        ImageCandidate::new(path.to_string_lossy(), None, None).unwrap()
    }

    fn assert_thumbnail_invariants(thumbnail: &DecodedThumbnail) {
        assert!(thumbnail.width > 0);
        assert!(thumbnail.height > 0);
        assert!(thumbnail.width <= MAX_UPLOAD_WIDTH);
        assert!(thumbnail.height <= MAX_UPLOAD_HEIGHT);
        assert!(thumbnail.original_width > 0);
        assert!(thumbnail.original_height > 0);
        assert!(thumbnail.original_width <= MAX_DECODE_DIMENSION);
        assert!(thumbnail.original_height <= MAX_DECODE_DIMENSION);
        assert_eq!(
            thumbnail.rgba.len(),
            thumbnail.width as usize * thumbnail.height as usize * 4
        );
        assert!(thumbnail.allocation_bytes() <= MAX_DECODE_ALLOCATION_BYTES as usize);
        assert!(thumbnail.file_bytes <= MAX_FILE_BYTES);
    }

    #[test]
    fn supported_extensions_and_path_tokens_are_safe_and_io_free() {
        assert!(has_supported_extension("photo.PNG"));
        assert!(has_supported_extension("'folder/a picture.webp'"));
        assert!(!has_supported_extension("script.svg"));
        assert!(!has_supported_extension("https://example.com/a.png?x=1"));
        assert_eq!(
            path_token_at_line("  photo.png  ", 4).as_deref(),
            Some("photo.png")
        );
        assert_eq!(
            path_token_at_line("open 'folder/my image.JPEG' now", 16).as_deref(),
            Some("folder/my image.JPEG")
        );
        assert!(path_token_at_line("README.md", 4).is_none());
    }

    #[test]
    fn row_tokenizer_supports_icons_quotes_unicode_and_multiple_images() {
        let line = "\u{f1c5} photo.png  'folder/my image.JPEG'  données.webp README.md";
        let tokens = image_path_tokens_in_line(line);
        assert_eq!(
            tokens
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>(),
            ["photo.png", "folder/my image.JPEG", "données.webp"]
        );
        assert_eq!(
            path_token_at_line(line, line.chars().position(|ch| ch == 'p').unwrap())
                .as_deref(),
            Some("photo.png")
        );
    }

    #[test]
    fn row_tokenizer_rejects_remote_paths_and_preserves_punctuation_in_names() {
        let tokens = image_path_tokens_in_line(
            "https://example.com/a.png file:///tmp/b.png ./screens/[final]-v2.png",
        );
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "./screens/[final]-v2.png");
    }

    #[test]
    fn candidates_reject_remote_and_control_character_inputs() {
        assert!(ImageCandidate::new("https://example.com/a.png", None, None).is_none());
        assert!(ImageCandidate::new("file:///tmp/a.png", None, None).is_none());
        assert!(ImageCandidate::new("FILE:C:\\temp\\a.png", None, None).is_none());
        assert!(ImageCandidate::new("image.png\nnext-command", None, None).is_none());
        assert!(ImageCandidate::new("image.png\u{1b}", None, None).is_none());
    }

    #[test]
    fn encoded_size_is_rejected_before_image_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.png");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_FILE_BYTES + 1).unwrap();
        assert_eq!(
            decode_candidate(&candidate(&path), &mut ThumbnailCache::default())
                .unwrap_err(),
            ImagePreviewError::TooLarge
        );
    }

    #[test]
    fn header_dimensions_are_rejected_before_full_pixel_decode() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.png");
        image_rs::RgbaImage::from_pixel(4_097, 1, image_rs::Rgba([10, 20, 30, 255]))
            .save(&path)
            .unwrap();
        assert_eq!(
            decode_candidate(&candidate(&path), &mut ThumbnailCache::default())
                .unwrap_err(),
            ImagePreviewError::TooLarge
        );
    }

    #[test]
    fn magic_bytes_must_name_an_explicitly_supported_raster_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.png");
        std::fs::write(&path, b"<svg><script>bad()</script></svg>").unwrap();
        assert_eq!(
            decode_candidate(&candidate(&path), &mut ThumbnailCache::default())
                .unwrap_err(),
            ImagePreviewError::UnsupportedFormat
        );
    }

    #[test]
    fn warm_cache_reuses_the_exact_pixel_allocation_and_render_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.png");
        image_rs::RgbaImage::from_pixel(4, 3, image_rs::Rgba([10, 20, 30, 255]))
            .save(&path)
            .unwrap();
        let mut cache = ThumbnailCache::default();
        let first = decode_candidate(&candidate(&path), &mut cache).unwrap();
        let second = decode_candidate(&candidate(&path), &mut cache).unwrap();
        assert!(!first.reused);
        assert!(second.reused);
        assert_eq!((first.thumbnail.width, first.thumbnail.height), (4, 3));
        assert_eq!(first.thumbnail.render_id, second.thumbnail.render_id);
        assert_eq!(first.thumbnail.decoded_at, second.thumbnail.decoded_at);
        assert!(Arc::ptr_eq(&first.thumbnail, &second.thumbnail));
        assert!(Arc::ptr_eq(&first.thumbnail.rgba, &second.thumbnail.rgba));
    }

    #[test]
    fn png_and_jpeg_decode_to_visible_opaque_rgba_pixels() {
        let dir = tempfile::tempdir().unwrap();
        for (name, format) in [
            ("bright.png", ImageFormat::Png),
            ("bright.jpg", ImageFormat::Jpeg),
        ] {
            let path = dir.path().join(name);
            let source = image_rs::RgbImage::from_fn(32, 24, |x, y| {
                if (x / 8 + y / 8) % 2 == 0 {
                    image_rs::Rgb([255, 244, 32])
                } else {
                    image_rs::Rgb([32, 220, 255])
                }
            });
            source.save_with_format(&path, format).unwrap();

            let decoded =
                decode_candidate(&candidate(&path), &mut ThumbnailCache::default())
                    .unwrap();
            assert_thumbnail_invariants(&decoded.thumbnail);
            assert_eq!(
                (decoded.thumbnail.width, decoded.thumbnail.height),
                (32, 24)
            );
            assert_eq!(decoded.thumbnail.rgba.len(), 32 * 24 * 4);
            assert!(decoded
                .thumbnail
                .rgba
                .chunks_exact(4)
                .all(|pixel| pixel[3] == 255));
            let mean_luminance = decoded
                .thumbnail
                .rgba
                .chunks_exact(4)
                .map(|pixel| {
                    (u64::from(pixel[0]) * 54
                        + u64::from(pixel[1]) * 183
                        + u64::from(pixel[2]) * 19)
                        >> 8
                })
                .sum::<u64>()
                / (32 * 24);
            assert!(
                mean_luminance >= 160,
                "{name} decoded unexpectedly dark: mean luminance {mean_luminance}"
            );
        }
    }

    #[test]
    fn every_enabled_raster_codec_decodes_with_exact_rgba_accounting() {
        let dir = tempfile::tempdir().unwrap();
        let source = image_rs::RgbImage::from_fn(9, 7, |x, y| {
            image_rs::Rgb([
                32 + (x * 17) as u8,
                48 + (y * 19) as u8,
                220_u8.saturating_sub((x + y) as u8 * 5),
            ])
        });
        for (extension, format) in [
            ("bmp", ImageFormat::Bmp),
            ("gif", ImageFormat::Gif),
            ("ico", ImageFormat::Ico),
            ("jpg", ImageFormat::Jpeg),
            ("png", ImageFormat::Png),
            ("pnm", ImageFormat::Pnm),
            ("tiff", ImageFormat::Tiff),
            ("webp", ImageFormat::WebP),
        ] {
            let path = dir.path().join(format!("codec.{extension}"));
            if format == ImageFormat::Ico {
                image_rs::DynamicImage::ImageRgba8(image_rs::RgbaImage::from_fn(
                    9,
                    7,
                    |x, y| {
                        let pixel = source.get_pixel(x, y);
                        image_rs::Rgba([pixel[0], pixel[1], pixel[2], 255])
                    },
                ))
                .save_with_format(&path, format)
                .unwrap_or_else(|error| {
                    panic!("{format:?} fixture encode failed: {error}")
                });
            } else {
                source
                    .save_with_format(&path, format)
                    .unwrap_or_else(|error| {
                        panic!("{format:?} fixture encode failed: {error}")
                    });
            }
            let decoded =
                decode_candidate(&candidate(&path), &mut ThumbnailCache::default())
                    .unwrap_or_else(|error| {
                        panic!("{format:?} decode failed: {error:?}")
                    });
            assert_eq!(
                (
                    decoded.thumbnail.original_width,
                    decoded.thumbnail.original_height
                ),
                (9, 7),
                "{format:?}"
            );
            assert_thumbnail_invariants(&decoded.thumbnail);
            assert!(
                decoded
                    .thumbnail
                    .rgba
                    .chunks_exact(4)
                    .any(|pixel| pixel[..3] != [0, 0, 0]),
                "{format:?} unexpectedly decoded as an all-black image"
            );
        }
    }

    #[test]
    fn transparent_png_preserves_straight_alpha_and_exact_channel_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("alpha.png");
        let mut source = image_rs::RgbaImage::new(2, 1);
        source.put_pixel(0, 0, image_rs::Rgba([250, 10, 20, 128]));
        source.put_pixel(1, 0, image_rs::Rgba([30, 40, 240, 0]));
        source.save_with_format(&path, ImageFormat::Png).unwrap();

        let decoded =
            decode_candidate(&candidate(&path), &mut ThumbnailCache::default()).unwrap();
        assert_thumbnail_invariants(&decoded.thumbnail);
        assert_eq!(
            decoded.thumbnail.rgba.as_ref(),
            &[250, 10, 20, 128, 30, 40, 240, 0]
        );
    }

    #[test]
    fn portrait_jpeg_downscale_preserves_visible_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("portrait.jpg");
        image_rs::RgbImage::from_fn(1_080, 1_374, |x, y| {
            if (x / 120 + y / 120) % 2 == 0 {
                image_rs::Rgb([248, 248, 248])
            } else {
                image_rs::Rgb([40, 210, 250])
            }
        })
        .save_with_format(&path, ImageFormat::Jpeg)
        .unwrap();

        let decoded =
            decode_candidate(&candidate(&path), &mut ThumbnailCache::default()).unwrap();
        assert_eq!(
            (
                decoded.thumbnail.original_width,
                decoded.thumbnail.original_height
            ),
            (1_080, 1_374)
        );
        assert_eq!(decoded.thumbnail.height, MAX_UPLOAD_HEIGHT);
        assert!(decoded.thumbnail.width <= MAX_UPLOAD_WIDTH);
        let mean_luminance = decoded
            .thumbnail
            .rgba
            .chunks_exact(4)
            .map(|pixel| {
                (u64::from(pixel[0]) * 54
                    + u64::from(pixel[1]) * 183
                    + u64::from(pixel[2]) * 19)
                    >> 8
            })
            .sum::<u64>()
            / u64::from(decoded.thumbnail.width * decoded.thumbnail.height);
        assert!(
            mean_luminance >= 150,
            "downscaled portrait JPEG is unexpectedly dark: {mean_luminance}"
        );
    }

    #[test]
    fn changed_file_version_invalidates_cached_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("changing.png");
        image_rs::RgbaImage::from_pixel(4, 3, image_rs::Rgba([1, 2, 3, 255]))
            .save(&path)
            .unwrap();
        let mut cache = ThumbnailCache::default();
        let first = decode_candidate(&candidate(&path), &mut cache).unwrap();
        image_rs::RgbaImage::from_pixel(5, 3, image_rs::Rgba([4, 5, 6, 255]))
            .save(&path)
            .unwrap();
        let second = decode_candidate(&candidate(&path), &mut cache).unwrap();
        assert!(!second.reused);
        assert_eq!(second.thumbnail.width, 5);
        assert_ne!(first.thumbnail.render_id, second.thumbnail.render_id);
        assert!(!Arc::ptr_eq(&first.thumbnail, &second.thumbnail));
    }

    #[test]
    fn cache_enforces_byte_and_entry_budgets_with_lru_promotion() {
        fn thumbnail(bytes: usize) -> Arc<DecodedThumbnail> {
            Arc::new(DecodedThumbnail {
                render_id: bytes as u64,
                decoded_at: Instant::now(),
                width: 1,
                height: 1,
                original_width: 1,
                original_height: 1,
                file_bytes: bytes as u64,
                rgba: Arc::from(vec![0_u8; bytes]),
            })
        }
        fn key(name: &str, bytes: u64) -> ThumbnailCacheKey {
            ThumbnailCacheKey {
                path: PathBuf::from(name),
                file_bytes: bytes,
                modified: SystemTime::UNIX_EPOCH,
            }
        }

        let mut cache = ThumbnailCache::new(8, 2);
        let a = key("a.png", 4);
        let b = key("b.png", 4);
        let c = key("c.png", 4);
        cache.insert(a.clone(), thumbnail(4));
        cache.insert(b.clone(), thumbnail(4));
        assert_eq!(
            cache.stats(),
            ThumbnailCacheStats {
                entries: 2,
                used_bytes: 8,
                max_entries: 2,
                max_bytes: 8,
            }
        );
        assert!(cache.get(&a).is_some());
        cache.insert(c.clone(), thumbnail(4));
        assert!(cache.get(&b).is_none());
        assert!(cache.get(&a).is_some());
        assert!(cache.get(&c).is_some());
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.used_bytes(), 8);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.used_bytes(), 0);
    }

    #[test]
    fn cache_accounting_remains_bounded_under_replacement_storms() {
        fn thumbnail(render_id: u64, bytes: usize) -> Arc<DecodedThumbnail> {
            Arc::new(DecodedThumbnail {
                render_id,
                decoded_at: Instant::now(),
                width: 1,
                height: 1,
                original_width: 1,
                original_height: 1,
                file_bytes: bytes as u64,
                rgba: Arc::from(vec![0_u8; bytes]),
            })
        }

        let mut cache = ThumbnailCache::new(4_096, 8);
        for index in 0..10_000_u64 {
            let bytes = ((index as usize % 17) + 1) * 64;
            let key = ThumbnailCacheKey {
                path: PathBuf::from(format!("{}.png", index % 23)),
                file_bytes: index % 7,
                modified: SystemTime::UNIX_EPOCH,
            };
            cache.insert(key, thumbnail(index, bytes));
            let stats = cache.stats();
            assert!(stats.entries <= stats.max_entries);
            assert!(stats.used_bytes <= stats.max_bytes);
            assert_eq!(
                stats.used_bytes,
                cache
                    .entries
                    .iter()
                    .map(|entry| entry.thumbnail.allocation_bytes())
                    .sum::<usize>()
            );
        }

        cache.insert(
            ThumbnailCacheKey {
                path: PathBuf::from("too-large.png"),
                file_bytes: 4_097,
                modified: SystemTime::UNIX_EPOCH,
            },
            thumbnail(99_999, 4_097),
        );
        assert!(cache.stats().used_bytes <= 4_096);
        assert!(cache.stats().entries <= 8);
    }

    #[test]
    fn malformed_inputs_never_escape_as_successful_images() {
        for bytes in [
            &b""[..],
            &b"\x89PNG\r\n\x1a\n"[..],
            &b"GIF89a\xff\xff\xff\xff"[..],
            &b"RIFF\xff\xff\xff\xffWEBP"[..],
            &b"II*\0\xff\xff\xff\xff"[..],
        ] {
            assert!(decode_bounded_bytes(bytes).is_err());
        }
    }

    #[test]
    fn deterministic_malformed_and_mutated_input_storm_preserves_all_bounds() {
        use image_rs::ImageEncoder as _;

        let mut valid_png = Vec::new();
        image_rs::codecs::png::PngEncoder::new(&mut valid_png)
            .write_image(
                &[20, 40, 60, 255, 200, 180, 160, 128],
                2,
                1,
                image_rs::ExtendedColorType::Rgba8,
            )
            .unwrap();

        let mut inputs = Vec::new();
        for end in 0..=valid_png.len() {
            inputs.push(valid_png[..end].to_vec());
        }
        for index in 0..512_usize {
            let mut mutated = valid_png.clone();
            let byte = index % mutated.len();
            mutated[byte] ^= 1 << (index % 8);
            inputs.push(mutated);
        }
        let mut state = 0xC0FF_EE12_3456_789A_u64;
        for length in 0..512_usize {
            let mut bytes = vec![0_u8; length];
            for byte in &mut bytes {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
            }
            inputs.push(bytes);
        }

        for bytes in inputs {
            if let Ok(thumbnail) = decode_bounded_bytes(&bytes) {
                assert_thumbnail_invariants(&thumbnail);
            }
        }
    }

    #[test]
    fn repeated_decode_and_cache_hits_do_not_retain_file_handles_or_write_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reusable.png");
        let moved = dir.path().join("moved.png");
        image_rs::RgbaImage::from_pixel(64, 48, image_rs::Rgba([10, 80, 220, 255]))
            .save(&path)
            .unwrap();
        let mut cache = ThumbnailCache::default();
        for _ in 0..1_000 {
            let outcome = decode_candidate(&candidate(&path), &mut cache).unwrap();
            assert_thumbnail_invariants(&outcome.thumbnail);
        }
        assert_eq!(cache.stats().entries, 1);
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            1,
            "preview decoding must never create thumbnails or sidecar files"
        );

        std::fs::rename(&path, &moved)
            .expect("the decoder must close every source handle after each lookup");
        std::fs::remove_file(&moved)
            .expect("cached pixels must not retain an operating-system file handle");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn missing_directories_and_non_files_are_rejected_without_cache_growth() {
        let dir = tempfile::tempdir().unwrap();
        let directory = dir.path().join("folder.png");
        std::fs::create_dir(&directory).unwrap();
        let missing = dir.path().join("missing.png");
        let mut cache = ThumbnailCache::default();
        assert_eq!(
            decode_candidate(&candidate(&directory), &mut cache).unwrap_err(),
            ImagePreviewError::NotRegularFile
        );
        assert_eq!(
            decode_candidate(&candidate(&missing), &mut cache).unwrap_err(),
            ImagePreviewError::NotFound
        );
        assert_eq!(cache.stats().entries, 0);
        assert_eq!(cache.stats().used_bytes, 0);
    }

    #[cfg(windows)]
    #[test]
    fn wsl_paths_map_only_through_validated_local_routes() {
        assert_eq!(
            map_wsl_path(Path::new("/mnt/d/work/image.png"), Some("Ubuntu")).unwrap(),
            PathBuf::from(r"D:\work\image.png")
        );
        assert_eq!(
            map_wsl_path(Path::new("/home/user/image.png"), Some("Ubuntu-24.04"))
                .unwrap(),
            PathBuf::from(r"\\wsl.localhost\Ubuntu-24.04\home\user\image.png")
        );
        assert_eq!(
            map_wsl_path(Path::new("/home/user/image.png"), Some("bad\\name")),
            Err(ImagePreviewError::NotLocal)
        );
    }

    #[cfg(unix)]
    #[test]
    fn final_component_symlinks_are_rejected() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.png");
        let link = dir.path().join("link.png");
        image_rs::RgbaImage::from_pixel(1, 1, image_rs::Rgba([1, 2, 3, 255]))
            .save(&target)
            .unwrap();
        symlink(&target, &link).unwrap();
        assert_eq!(
            decode_candidate(&candidate(&link), &mut ThumbnailCache::default())
                .unwrap_err(),
            ImagePreviewError::Symlink
        );
    }
}

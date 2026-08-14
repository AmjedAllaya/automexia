//! Renderer-neutral, resource-bounded local image decoding.
//!
//! Terminal output is untrusted. Only explicit local raster candidates are
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
    if original_width > 4_096
        || original_height > 4_096
        || pixels > MAX_DECODED_PIXELS
        || pixels.saturating_mul(4) > MAX_DECODE_ALLOCATION_BYTES
    {
        return Err(ImagePreviewError::TooLarge);
    }

    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image_rs::Limits::default();
    limits.max_image_width = Some(4_096);
    limits.max_image_height = Some(4_096);
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

    static NEXT_RENDER_ID: AtomicU64 = AtomicU64::new(1);

    Ok(DecodedThumbnail {
        render_id: NEXT_RENDER_ID.fetch_add(1, Ordering::Relaxed),
        decoded_at: Instant::now(),
        width: image.width(),
        height: image.height(),
        original_width,
        original_height,
        file_bytes,
        rgba: Arc::from(image.into_raw()),
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

pub fn path_token_at_line(line: &str, column: usize) -> Option<String> {
    let chars = line.chars().collect::<Vec<_>>();
    if chars.is_empty() || column >= chars.len() {
        return None;
    }

    for quote in ['\'', '"', '\u{60}'] {
        let left = chars[..=column]
            .iter()
            .rposition(|character| *character == quote);
        let right = chars[column..]
            .iter()
            .position(|character| *character == quote)
            .map(|offset| column + offset);
        if let (Some(left), Some(right)) = (left, right) {
            if left < right {
                let candidate = chars[left + 1..right].iter().collect::<String>();
                if has_supported_extension(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }

    let is_boundary = |character: char| {
        character.is_whitespace() || matches!(character, '|' | '<' | '>')
    };
    let mut start = column;
    while start > 0 && !is_boundary(chars[start - 1]) {
        start -= 1;
    }
    let mut end = column + 1;
    while end < chars.len() && !is_boundary(chars[end]) {
        end += 1;
    }
    let candidate =
        trim_path_token(&chars[start..end].iter().collect::<String>()).to_string();
    has_supported_extension(&candidate).then_some(candidate)
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

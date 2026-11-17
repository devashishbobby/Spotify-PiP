use super::{state::AppError, windows::wait};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::sync::atomic::AtomicBool;
use windows::{
    Graphics::Imaging::BitmapDecoder,
    Storage::Streams::{
        DataReader, IRandomAccessStreamReference, IRandomAccessStreamWithContentType,
    },
    core::Interface,
};
use windows_future::IAsyncOperation;

const MAX_BYTES: u64 = 2 * 1024 * 1024;
struct Stream(IRandomAccessStreamWithContentType);
impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}
struct Reader(DataReader);
impl Drop for Reader {
    fn drop(&mut self) {
        let _ = self.0.Close();
    }
}

fn mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else {
        None
    }
}
fn valid_dimensions(width: u32, height: u32) -> bool {
    width > 0
        && height > 0
        && width <= 4096
        && height <= 4096
        && u64::from(width) * u64::from(height) <= 16_000_000
}
pub fn read(
    reference: &IRandomAccessStreamReference,
    stop: &AtomicBool,
) -> Result<String, AppError> {
    let stream = Stream(wait(
        reference
            .OpenReadAsync()
            .map_err(|e| AppError::windows("open artwork", e))?,
        stop,
    )?);
    let size = stream
        .0
        .Size()
        .map_err(|e| AppError::windows("read artwork size", e))?;
    if size == 0 || size > MAX_BYTES {
        return Err(AppError::new(
            "artwork_size",
            "Artwork is absent or exceeds the size limit.",
        ));
    }
    let declared = stream
        .0
        .ContentType()
        .ok()
        .map(|s| s.to_string().to_ascii_lowercase())
        .unwrap_or_default();
    let decoder = wait(
        BitmapDecoder::CreateAsync(&stream.0)
            .map_err(|e| AppError::windows("inspect artwork", e))?,
        stop,
    )?;
    let width = decoder
        .PixelWidth()
        .map_err(|e| AppError::windows("inspect artwork width", e))?;
    let height = decoder
        .PixelHeight()
        .map_err(|e| AppError::windows("inspect artwork height", e))?;
    if !valid_dimensions(width, height) {
        return Err(AppError::new(
            "artwork_dimensions",
            "Artwork dimensions exceed the limit.",
        ));
    }
    let input = stream
        .0
        .GetInputStreamAt(0)
        .map_err(|e| AppError::windows("read artwork stream", e))?;
    let reader = Reader(
        DataReader::CreateDataReader(&input)
            .map_err(|e| AppError::windows("create artwork reader", e))?,
    );
    let operation = reader
        .0
        .LoadAsync(size as u32)
        .and_then(|operation| operation.cast::<IAsyncOperation<u32>>())
        .map_err(|e| AppError::windows("load artwork", e))?;
    let loaded = wait(operation, stop)?;
    if u64::from(loaded) != size {
        return Err(AppError::new(
            "artwork_incomplete",
            "Artwork stream was incomplete.",
        ));
    }
    let mut bytes = vec![0u8; size as usize];
    reader
        .0
        .ReadBytes(&mut bytes)
        .map_err(|e| AppError::windows("read artwork bytes", e))?;
    let detected = mime(&bytes)
        .ok_or_else(|| AppError::new("artwork_type", "Artwork image format is not supported."))?;
    if !declared.is_empty() && declared != "application/octet-stream" && declared != detected {
        return Err(AppError::new(
            "artwork_type",
            "Artwork image type does not match its content.",
        ));
    }
    Ok(format!("data:{detected};base64,{}", STANDARD.encode(bytes)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_types_and_excessive_pixels() {
        assert_eq!(mime(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
        assert_eq!(mime(&[0xff, 0xd8, 0xff, 0]), Some("image/jpeg"));
        assert_eq!(mime(b"<svg>"), None);
        assert!(!valid_dimensions(0, 1));
        assert!(!valid_dimensions(4096, 4096));
        assert!(!valid_dimensions(5000, 50));
        assert!(valid_dimensions(640, 640));
    }
}

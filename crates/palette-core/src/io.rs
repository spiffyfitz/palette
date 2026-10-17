//! Explicit project saving versus flattened export, with bounded decoding and
//! same-directory atomic replacement. A failed encoder never touches the target.
use crate::{composite, document::Document, raster::Raster};
mod avif;
pub mod paint;
mod project;
pub use avif::{AvifOptions, export_avif};
use flate2::read::ZlibDecoder;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"PALETTE\0";
const VERSION: u32 = 3;
const MAX_PROJECT_BYTES: u64 = 256 * 1024 * 1024;

/// Recoverable format, codec and filesystem failures suitable for a user dialog.
#[derive(Debug, Error)]
pub enum FileError {
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image codec failed: {0}")]
    Image(#[from] image::ImageError),
    #[error("Project metadata is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

/// Lowercase UTF-8 extension used for explicit format dispatch.
pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Import a native project or supported raster with bounded decoding.
pub fn open(path: &Path) -> Result<Document, FileError> {
    match extension(path).as_str() {
        "palette" => read_project(path),
        "paint" => paint::read(path),
        "avif" => avif::read(path),
        "heif" | "heic" => Err(FileError::Invalid("HEIF/HEIC is not enabled.".into())),
        _ => {
            let mut reader = ImageReader::open(path)?.with_guessed_format()?;
            let mut limits = Limits::default();
            limits.max_image_width = Some(16384);
            limits.max_image_height = Some(16384);
            limits.max_alloc = Some(MAX_PROJECT_BYTES);
            reader.limits(limits);
            let mut decoder = reader.into_decoder()?;
            let metadata = crate::document::DocumentMetadata {
                icc_profile: std::sync::Arc::new(decoder.icc_profile()?.unwrap_or_default()),
                exif: std::sync::Arc::new(decoder.exif_metadata()?.unwrap_or_default()),
            };
            if metadata.icc_profile.len() > 4 * 1024 * 1024 || metadata.exif.len() > 4 * 1024 * 1024
            {
                return Err(FileError::Invalid("ICC/Exif metadata exceeds 4 MiB".into()));
            }
            let orientation = decoder.orientation()?;
            let mut image = DynamicImage::from_decoder(decoder)?;
            image.apply_orientation(orientation);
            let image = image.to_rgba8();
            let mut document = Document::from_raster(
                Raster::from_rgba(image.width(), image.height(), image.as_raw())
                    .map_err(FileError::Invalid)?,
            );
            document.metadata = metadata;
            document.validate().map_err(FileError::Invalid)?;
            Ok(document)
        }
    }
}

/// Read the versioned Palette container and validate all model invariants.
pub fn read_project(path: &Path) -> Result<Document, FileError> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() > project::MAX_FILE_BYTES {
        return Err(FileError::Invalid(
            "Project exceeds the 1 GiB input limit".into(),
        ));
    }
    let mut header = [0; 12];
    file.read_exact(&mut header)?;
    if &header[..8] != MAGIC {
        return Err(FileError::Invalid("Not a Palette project".into()));
    }
    let version = u32::from_le_bytes(header[8..12].try_into().expect("version"));
    if matches!(version, 2 | 3) {
        return project::read(file);
    }
    if version != 1 {
        return Err(FileError::Invalid(format!(
            "Unsupported project version {version}; this build supports version {VERSION}"
        )));
    }
    if file.metadata()?.len() > MAX_PROJECT_BYTES {
        return Err(FileError::Invalid("Version 1 input exceeds 256 MiB".into()));
    }
    let mut json = Vec::new();
    ZlibDecoder::new(file)
        .take(MAX_PROJECT_BYTES + 1)
        .read_to_end(&mut json)?;
    if json.len() as u64 > MAX_PROJECT_BYTES {
        return Err(FileError::Invalid(
            "Decompressed project exceeds the 256 MiB limit".into(),
        ));
    }
    let document: Document = serde_json::from_slice(&json)?;
    document.validate().map_err(FileError::Invalid)?;
    Ok(document)
}

fn atomic_write(
    path: &Path,
    encode: impl FnOnce(&mut File) -> Result<(), FileError>,
) -> Result<(), FileError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    encode(temporary.as_file_mut())?;
    temporary.as_file_mut().sync_all()?;
    temporary
        .persist(path)
        .map_err(|e| FileError::Io(e.error))?;
    Ok(())
}

/// Atomically preserve all currently represented editable document state.
pub fn save_project(path: &Path, document: &Document) -> Result<(), FileError> {
    if extension(path) != "palette" {
        return Err(FileError::Invalid(
            "Editable projects must use the .palette extension".into(),
        ));
    }
    document.validate().map_err(FileError::Invalid)?;
    atomic_write(path, |file| {
        file.write_all(MAGIC)?;
        file.write_all(&VERSION.to_le_bytes())?;
        project::write(file, document)
    })
}

/// Atomically encode the composite, with an explicit white background for JPEG.
pub fn export(path: &Path, document: &Document) -> Result<(), FileError> {
    if extension(path) == "avif" {
        return export_avif(path, document, AvifOptions::default());
    }
    document.validate().map_err(FileError::Invalid)?;
    let format = match extension(path).as_str() {
        "png" => ImageFormat::Png,
        "jpg" | "jpeg" => ImageFormat::Jpeg,
        "bmp" => ImageFormat::Bmp,
        "gif" => ImageFormat::Gif,
        "tif" | "tiff" => ImageFormat::Tiff,
        "webp" => ImageFormat::WebP,
        _ => {
            return Err(FileError::Invalid(
                "Choose PNG, JPEG, BMP, GIF, TIFF, WebP or AVIF for flattened export".into(),
            ));
        }
    };
    let raster = composite::render(document);
    let mut rgba = raster.rgba();
    // JPEG has no alpha. Composite on white explicitly rather than dropping alpha.
    if format == ImageFormat::Jpeg {
        for pixel in rgba.as_chunks_mut::<4>().0 {
            for c in 0..3 {
                pixel[c] = ((u32::from(pixel[c]) * u32::from(pixel[3])
                    + 255 * (255 - u32::from(pixel[3]))
                    + 127)
                    / 255) as u8;
            }
            pixel[3] = 255;
        }
    }
    let image = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(document.width, document.height, rgba).expect("validated raster"),
    );
    atomic_write(path, |file| {
        if format == ImageFormat::Jpeg {
            image.to_rgb8().write_to(file, format)?;
        } else {
            image.write_to(file, format)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_after_partial_encoding_preserves_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("existing.palette");
        std::fs::write(&path, b"original data").unwrap();
        let result = atomic_write(&path, |file| {
            file.write_all(b"partial new encoding")?;
            Err(FileError::Io(std::io::Error::other(
                "simulated encoder failure",
            )))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original data");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

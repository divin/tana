//! Image file management module for Tana
//!
//! Handles validation, copying, and directory management for image files.
//! Supported image formats: PNG, JPG, JPEG, WebP, GIF, BMP

use crate::error::{Result, TanaError};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

/// Supported image file extensions
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// Maximum image file size (50 MB)
const MAX_IMAGE_SIZE: u64 = 50 * 1024 * 1024;

/// Download timeout in seconds
const DOWNLOAD_TIMEOUT_SECS: u64 = 30;

/// Magic bytes (file signatures) for supported image formats
const IMAGE_MAGIC_BYTES: &[(&[u8], &str)] = &[
    (b"\x89PNG\r\n\x1a\n", "png"),
    (b"\xFF\xD8\xFF", "jpg"),
    (b"RIFF", "webp"), // RIFF signature for WebP
    (b"GIF87a", "gif"),
    (b"GIF89a", "gif"),
    (b"BM", "bmp"),
];

/// Validate an image file path and expand home directory
///
/// # Arguments
/// * `path` - The image file path to validate (can include ~)
///
/// # Returns
/// * `Ok(String)` - The expanded absolute path
/// * `Err(TanaError)` - If the file doesn't exist or has unsupported format
///
/// # Supported formats
/// - PNG (.png)
/// - JPG (.jpg)
/// - JPEG (.jpeg)
/// - WebP (.webp)
/// - GIF (.gif)
/// - BMP (.bmp)
pub fn validate_image_path(path: &str) -> Result<String> {
    let expanded_path = expand_home(path);

    // Check if file exists
    if !expanded_path.exists() {
        return Err(TanaError::InvalidInput(format!(
            "Image file not found: {}",
            path
        )));
    }

    // Check if it's a file (not a directory)
    if !expanded_path.is_file() {
        return Err(TanaError::InvalidInput(format!(
            "Path is not a file: {}",
            path
        )));
    }

    // Check file extension
    let extension = expanded_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
        .ok_or_else(|| TanaError::InvalidInput("Image file has no extension".to_string()))?;

    if !SUPPORTED_FORMATS.contains(&extension.as_str()) {
        return Err(TanaError::InvalidInput(format!(
            "Unsupported image format: .{}. Supported formats: {}",
            extension,
            SUPPORTED_FORMATS.join(", ")
        )));
    }

    Ok(expanded_path.to_string_lossy().to_string())
}

/// Ensure that an images directory exists, creating it if necessary
///
/// # Arguments
/// * `path` - The directory path to ensure exists (can include ~)
///
/// # Returns
/// * `Ok(())` - If the directory exists or was created successfully
/// * `Err(TanaError)` - If the directory couldn't be created
pub fn ensure_images_directory(path: &str) -> Result<()> {
    let expanded_path = expand_home(path);

    // If the directory already exists, we're done
    if expanded_path.exists() {
        if !expanded_path.is_dir() {
            return Err(TanaError::InvalidInput(format!(
                "Path exists but is not a directory: {}",
                path
            )));
        }
        return Ok(());
    }

    // Create the directory and all parent directories
    fs::create_dir_all(&expanded_path).map_err(|e| {
        TanaError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to create images directory {}: {}", path, e),
        ))
    })?;

    Ok(())
}

/// Copy an image file to a destination directory
///
/// # Arguments
/// * `src` - The source image file path (can include ~)
/// * `dest_dir` - The destination directory path (can include ~)
///
/// # Returns
/// * `Ok(String)` - The path to the copied file
/// * `Err(TanaError)` - If validation or copying fails
///
/// # Process
/// 1. Validates the source image file
/// 2. Ensures the destination directory exists
/// 3. Copies the file to the destination
/// 4. Returns the destination file path
pub fn copy_image_file(src: &str, dest_dir: &str) -> Result<String> {
    // Validate source image
    let validated_src = validate_image_path(src)?;

    // Ensure destination directory exists
    ensure_images_directory(dest_dir)?;

    let src_path = PathBuf::from(&validated_src);
    let dest_path = expand_home(dest_dir);

    // Get the filename from the source
    let filename = src_path
        .file_name()
        .ok_or_else(|| TanaError::InvalidInput("Invalid source file path".to_string()))?;

    let dest_file = dest_path.join(filename);

    // Copy the file
    fs::copy(&src_path, &dest_file).map_err(|e| {
        TanaError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to copy image from {} to {}: {}", src, dest_dir, e),
        ))
    })?;

    // Return just the filename (relative path)
    Ok(filename.to_string_lossy().to_string())
}

/// Check if a string is a URL
fn is_url(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

/// Check if a path is a finalized image filename
///
/// A finalized image is just a simple filename (no directory separators,
/// no URL scheme). This indicates it's an already-processed image that
/// doesn't need to be reprocessed.
fn is_finalized_image(path: &str) -> bool {
    !path.contains('/') && !path.contains('\\') && !path.starts_with("http")
}

/// Validate image file by checking magic bytes
///
/// # Arguments
/// * `data` - The file data to validate
///
/// # Returns
/// * `Ok(())` - If the data is a valid image format
/// * `Err(TanaError)` - If the data is not a recognized image format
fn validate_image_magic_bytes(data: &[u8]) -> Result<()> {
    if data.is_empty() {
        return Err(TanaError::InvalidInput("Image file is empty".to_string()));
    }

    for (magic, _format) in IMAGE_MAGIC_BYTES {
        if data.starts_with(magic) {
            return Ok(());
        }
    }

    Err(TanaError::InvalidInput(
        "File is not a valid image format (PNG, JPG, JPEG, WebP, GIF, or BMP)".to_string(),
    ))
}

/// Download an image from a URL and save it locally
///
/// # Arguments
/// * `url` - The URL to download the image from
/// * `dest_dir` - The destination directory to save the image
///
/// # Returns
/// * `Ok(String)` - The path to the downloaded and saved file
/// * `Err(TanaError)` - If download or validation fails
///
/// # Process
/// 1. Validates the URL format
/// 2. Downloads the image with size and timeout limits
/// 3. Validates the image format using magic bytes
/// 4. Extracts filename from URL or generates one
/// 5. Saves the file to the destination directory
/// 6. Returns the file path
pub fn download_image_from_url(url: &str, dest_dir: &str) -> Result<String> {
    // Validate URL format
    if !is_url(url) {
        return Err(TanaError::InvalidInput(
            "Invalid URL format. URL must start with http:// or https://".to_string(),
        ));
    }

    // Ensure destination directory exists
    ensure_images_directory(dest_dir)?;

    // Download the image
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build()
        .map_err(|e| TanaError::InvalidInput(format!("Failed to create HTTP client: {}", e)))?;

    let response = client.get(url).send().map_err(|e| {
        TanaError::InvalidInput(format!("Failed to download image from {}: {}", url, e))
    })?;

    // Check HTTP status
    if !response.status().is_success() {
        return Err(TanaError::InvalidInput(format!(
            "Failed to download image. HTTP status: {}",
            response.status()
        )));
    }

    // Check content length
    if let Some(content_length) = response.content_length()
        && content_length > MAX_IMAGE_SIZE
    {
        return Err(TanaError::InvalidInput(format!(
            "Image file is too large. Maximum size: {} MB, got: {} MB",
            MAX_IMAGE_SIZE / (1024 * 1024),
            content_length / (1024 * 1024)
        )));
    }

    // Read the response body
    let bytes = response
        .bytes()
        .map_err(|e| TanaError::InvalidInput(format!("Failed to read image data: {}", e)))?;

    let bytes = bytes.to_vec();

    // Check size limits
    if bytes.len() as u64 > MAX_IMAGE_SIZE {
        return Err(TanaError::InvalidInput(format!(
            "Image file is too large. Maximum size: {} MB, got: {} MB",
            MAX_IMAGE_SIZE / (1024 * 1024),
            bytes.len() / (1024 * 1024)
        )));
    }

    // Validate image format using magic bytes
    validate_image_magic_bytes(&bytes)?;

    // Extract filename from URL or generate one
    let filename = extract_filename_from_url(url)
        .unwrap_or_else(|| format!("image_{}.jpg", chrono::Local::now().timestamp()));

    let dest_path = expand_home(dest_dir);
    let dest_file = dest_path.join(&filename);

    // Write the file
    let mut file = fs::File::create(&dest_file).map_err(|e| {
        TanaError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to create image file: {}", e),
        ))
    })?;

    file.write_all(&bytes).map_err(|e| {
        TanaError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write image data: {}", e),
        ))
    })?;

    // Return just the filename (relative path)
    Ok(filename)
}

/// Process an image input that could be either a URL or a local file path
///
/// # Arguments
/// * `input` - The image input (URL or local file path)
/// * `dest_dir` - The destination directory to save/copy the image
///
/// # Returns
/// * `Ok(String)` - The path to the processed image file
/// * `Err(TanaError)` - If processing fails
///
/// # Process
/// - If input is a URL: downloads and saves the image locally
/// - If input is a local path: validates and copies to destination directory
/// - Returns the final path for storage in the database
pub fn process_image_input(input: &str, dest_dir: &str) -> Result<String> {
    // If it's already a finalized image filename, return it as-is
    if is_finalized_image(input) {
        return Ok(input.to_string());
    }

    // Otherwise, process it as a new image (download from URL or copy from file)
    if is_url(input) {
        download_image_from_url(input, dest_dir)
    } else {
        copy_image_file(input, dest_dir)
    }
}

/// Extract filename from a URL
///
/// # Arguments
/// * `url` - The URL to extract filename from
///
/// # Returns
/// * `Some(String)` - The filename if extraction was successful
/// * `None` - If filename could not be extracted
fn extract_filename_from_url(url: &str) -> Option<String> {
    // Remove query parameters
    let url_without_query = url.split('?').next()?;

    // Get the last path component
    let filename = url_without_query.split('/').next_back()?;

    // Check if it looks like a valid filename with an extension
    if !filename.is_empty() && filename.contains('.') {
        // Validate the extension
        let ext = filename.split('.').next_back()?;
        if SUPPORTED_FORMATS.contains(&ext.to_lowercase().as_str()) {
            return Some(filename.to_string());
        }
    }

    None
}

/// Extract file extension from a filename
///
/// # Arguments
/// * `filename` - The filename to extract extension from
///
/// # Returns
/// * `Option<String>` - The extension (without the dot) in lowercase, or None if no valid extension
fn get_file_extension(filename: &str) -> Option<String> {
    // Ensure the filename actually contains a dot
    if !filename.contains('.') {
        return None;
    }
    filename
        .split('.')
        .next_back()
        .map(|ext| ext.to_lowercase())
        .filter(|ext| !ext.is_empty())
}

/// Generate a rule-based filename for an image
///
/// # Arguments
/// * `media_type` - Type of media: "book", "movie", or "series"
/// * `id` - The database ID of the media record
/// * `extension` - The file extension (without the dot)
///
/// # Returns
/// * `String` - The formatted filename (e.g., "book_123_cover.jpg")
pub fn generate_filename(media_type: &str, id: i32, extension: &str) -> String {
    let purpose = match media_type {
        "book" => "cover",
        "movie" | "series" => "poster",
        _ => "image",
    };
    format!("{}_{}_{}.{}", media_type, id, purpose, extension)
}

/// Finalize an image file by renaming it from a temporary name to a rule-based name
///
/// # Arguments
/// * `images_dir` - The directory containing the image
/// * `temp_filename` - The current temporary filename
/// * `media_type` - Type of media: "book", "movie", or "series"
/// * `id` - The database ID of the media record
///
/// # Returns
/// * `Ok(String)` - The new rule-based filename
/// * `Err(TanaError)` - If the rename operation fails
pub fn finalize_image(
    images_dir: &str,
    temp_filename: &str,
    media_type: &str,
    id: i32,
) -> Result<String> {
    let images_path = expand_home(images_dir);
    let temp_path = images_path.join(temp_filename);

    // Extract the extension from the temp filename
    let extension = get_file_extension(temp_filename).ok_or_else(|| {
        TanaError::InvalidInput(format!(
            "Cannot determine file extension for {}",
            temp_filename
        ))
    })?;

    // Generate the final filename
    let final_filename = generate_filename(media_type, id, &extension);
    let final_path = images_path.join(&final_filename);

    // Rename the file
    fs::rename(&temp_path, &final_path).map_err(|e| {
        TanaError::Io(std::io::Error::new(
            e.kind(),
            format!(
                "Failed to finalize image from {} to {}: {}",
                temp_filename, final_filename, e
            ),
        ))
    })?;

    Ok(final_filename)
}

/// Expand home directory in path (~/)
fn expand_home(path: &str) -> PathBuf {
    if path.starts_with("~") {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(path.replacen("~", &home, 1))
        } else {
            PathBuf::from(path)
        }
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    fn get_unique_temp_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_base = std::env::temp_dir();
        temp_base.join(format!("tana_test_{}", nanos))
    }

    fn create_test_file(dir: &PathBuf, filename: &str) -> PathBuf {
        let _ = fs::create_dir_all(dir);
        let file_path = dir.join(filename);
        File::create(&file_path).expect("Failed to create test file");
        file_path
    }

    #[test]
    fn test_validate_image_path_valid_png() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "image.png");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_valid_jpg() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "photo.jpg");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_valid_jpeg() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "photo.jpeg");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_valid_webp() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "image.webp");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_valid_gif() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "animation.gif");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_valid_bmp() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "bitmap.bmp");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_nonexistent() {
        let result = validate_image_path("/nonexistent/path/image_xyz.png");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_validate_image_path_unsupported_format() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "document.txt");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_uppercase_extension() {
        let temp_dir = get_unique_temp_dir();
        let file = create_test_file(&temp_dir, "image.PNG");
        let result = validate_image_path(file.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_validate_image_path_directory() {
        let temp_dir = get_unique_temp_dir();
        let _ = fs::create_dir_all(&temp_dir);
        let result = validate_image_path(temp_dir.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a file"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ensure_images_directory_creates_new() {
        let temp_dir = get_unique_temp_dir();
        let new_dir = temp_dir.join("new_images");
        let result = ensure_images_directory(new_dir.to_str().unwrap());
        assert!(result.is_ok());
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ensure_images_directory_existing() {
        let temp_dir = get_unique_temp_dir();
        let _ = fs::create_dir_all(&temp_dir);
        let result = ensure_images_directory(temp_dir.to_str().unwrap());
        assert!(result.is_ok());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ensure_images_directory_nested() {
        let temp_dir = get_unique_temp_dir();
        let nested = temp_dir.join("a").join("b").join("c").join("images");
        let result = ensure_images_directory(nested.to_str().unwrap());
        assert!(result.is_ok());
        assert!(nested.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ensure_images_directory_file_conflict() {
        let temp_dir = get_unique_temp_dir();
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = create_test_file(&temp_dir, "conflict");
        let result = ensure_images_directory(file_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a directory"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_image_file_success() {
        let temp_dir = get_unique_temp_dir();
        let src_dir = temp_dir.join("src");
        let dest_dir = temp_dir.join("dest");
        let _ = fs::create_dir_all(&src_dir);

        let src_file = src_dir.join("test.png");
        {
            let mut f = File::create(&src_file).unwrap();
            f.write_all(b"test data").unwrap();
        }

        let result = copy_image_file(src_file.to_str().unwrap(), dest_dir.to_str().unwrap());

        assert!(result.is_ok());
        let filename = result.unwrap();
        let dest_file = dest_dir.join(&filename);
        assert!(dest_file.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_image_file_creates_directory() {
        let temp_dir = get_unique_temp_dir();
        let src_dir = temp_dir.join("src");
        let dest_dir = temp_dir.join("dest").join("nested");
        let _ = fs::create_dir_all(&src_dir);

        let src_file = create_test_file(&src_dir, "test.jpg");
        let result = copy_image_file(src_file.to_str().unwrap(), dest_dir.to_str().unwrap());

        assert!(result.is_ok());
        assert!(dest_dir.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_image_file_invalid_source() {
        let temp_dir = get_unique_temp_dir();
        let _ = fs::create_dir_all(&temp_dir);
        let result = copy_image_file("/nonexistent/image.png", temp_dir.to_str().unwrap());
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_copy_image_file_unsupported_format() {
        let temp_dir = get_unique_temp_dir();
        let src_dir = temp_dir.join("src");
        let dest_dir = temp_dir.join("dest");
        let _ = fs::create_dir_all(&src_dir);

        let src_file = create_test_file(&src_dir, "file.txt");
        let result = copy_image_file(src_file.to_str().unwrap(), dest_dir.to_str().unwrap());

        assert!(result.is_err());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_expand_home() {
        let expanded = expand_home("~/test.txt");
        assert!(expanded.to_string_lossy().contains("test.txt"));
        assert!(!expanded.to_string_lossy().contains("~"));

        let not_expanded = expand_home("/absolute/path.txt");
        assert_eq!(not_expanded.to_string_lossy(), "/absolute/path.txt");
    }

    #[test]
    fn test_supported_formats() {
        assert_eq!(SUPPORTED_FORMATS.len(), 6);
        assert!(SUPPORTED_FORMATS.contains(&"png"));
        assert!(SUPPORTED_FORMATS.contains(&"jpg"));
        assert!(SUPPORTED_FORMATS.contains(&"jpeg"));
        assert!(SUPPORTED_FORMATS.contains(&"webp"));
        assert!(SUPPORTED_FORMATS.contains(&"gif"));
        assert!(SUPPORTED_FORMATS.contains(&"bmp"));
    }

    #[test]
    fn test_process_image_input_with_local_file() {
        let temp_dir = get_unique_temp_dir();
        let src_dir = temp_dir.join("src");
        let dest_dir = temp_dir.join("dest");
        let _ = fs::create_dir_all(&src_dir);

        let src_file = create_test_file(&src_dir, "test.png");
        let result = process_image_input(src_file.to_str().unwrap(), dest_dir.to_str().unwrap());

        assert!(result.is_ok());
        let filename = result.unwrap();
        let dest_file = dest_dir.join(&filename);
        assert!(dest_file.exists());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_is_url() {
        assert!(is_url("https://example.com/image.jpg"));
        assert!(is_url("http://example.com/image.png"));
        assert!(!is_url("/local/path/image.jpg"));
        assert!(!is_url("~/image.jpg"));
    }

    #[test]
    fn test_validate_image_magic_bytes_png() {
        let png_magic = b"\x89PNG\r\n\x1a\n";
        let mut data = png_magic.to_vec();
        data.extend_from_slice(b"fake image data");
        assert!(validate_image_magic_bytes(&data).is_ok());
    }

    #[test]
    fn test_validate_image_magic_bytes_jpeg() {
        let jpeg_magic = b"\xFF\xD8\xFF";
        let mut data = jpeg_magic.to_vec();
        data.extend_from_slice(b"fake image data");
        assert!(validate_image_magic_bytes(&data).is_ok());
    }

    #[test]
    fn test_validate_image_magic_bytes_gif() {
        let gif_magic = b"GIF89a";
        let mut data = gif_magic.to_vec();
        data.extend_from_slice(b"fake image data");
        assert!(validate_image_magic_bytes(&data).is_ok());
    }

    #[test]
    fn test_validate_image_magic_bytes_invalid() {
        let invalid_data = b"This is not an image file";
        assert!(validate_image_magic_bytes(invalid_data).is_err());
    }

    #[test]
    fn test_validate_image_magic_bytes_empty() {
        let empty_data: &[u8] = &[];
        assert!(validate_image_magic_bytes(empty_data).is_err());
    }

    #[test]
    fn test_extract_filename_from_url_with_valid_extension() {
        let url = "https://example.com/path/to/image.jpg";
        assert_eq!(
            extract_filename_from_url(url),
            Some("image.jpg".to_string())
        );
    }

    #[test]
    fn test_extract_filename_from_url_with_query_params() {
        let url = "https://example.com/path/to/image.png?size=large&quality=high";
        assert_eq!(
            extract_filename_from_url(url),
            Some("image.png".to_string())
        );
    }

    #[test]
    fn test_extract_filename_from_url_png() {
        let url = "https://example.com/cover.png";
        assert_eq!(
            extract_filename_from_url(url),
            Some("cover.png".to_string())
        );
    }

    #[test]
    fn test_extract_filename_from_url_webp() {
        let url = "https://cdn.example.com/images/book_cover.webp";
        assert_eq!(
            extract_filename_from_url(url),
            Some("book_cover.webp".to_string())
        );
    }

    #[test]
    fn test_extract_filename_from_url_unsupported_extension() {
        let url = "https://example.com/document.pdf";
        assert_eq!(extract_filename_from_url(url), None);
    }

    #[test]
    fn test_extract_filename_from_url_no_extension() {
        let url = "https://example.com/image";
        assert_eq!(extract_filename_from_url(url), None);
    }

    #[test]
    fn test_generate_filename_book() {
        let filename = generate_filename("book", 42, "jpg");
        assert_eq!(filename, "book_42_cover.jpg");
    }

    #[test]
    fn test_generate_filename_movie() {
        let filename = generate_filename("movie", 123, "png");
        assert_eq!(filename, "movie_123_poster.png");
    }

    #[test]
    fn test_generate_filename_series() {
        let filename = generate_filename("series", 456, "webp");
        assert_eq!(filename, "series_456_poster.webp");
    }

    #[test]
    fn test_generate_filename_unknown_type() {
        let filename = generate_filename("unknown", 789, "gif");
        assert_eq!(filename, "unknown_789_image.gif");
    }

    #[test]
    fn test_finalize_image() {
        let temp_dir = get_unique_temp_dir();
        let images_dir = temp_dir.join("images");
        let _ = fs::create_dir_all(&images_dir);

        // Create a temporary file with timestamp-based name
        let temp_filename = "image_1775156955.jpg";
        let temp_path = images_dir.join(temp_filename);
        {
            let mut f = File::create(&temp_path).unwrap();
            f.write_all(b"fake image data").unwrap();
        }

        // Finalize the image
        let result = finalize_image(images_dir.to_str().unwrap(), temp_filename, "book", 42);

        assert!(result.is_ok());
        let final_filename = result.unwrap();
        assert_eq!(final_filename, "book_42_cover.jpg");

        // Check that the new file exists and the old one doesn't
        let final_path = images_dir.join(&final_filename);
        assert!(final_path.exists());
        assert!(!temp_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_finalize_image_preserves_extension() {
        let temp_dir = get_unique_temp_dir();
        let images_dir = temp_dir.join("images");
        let _ = fs::create_dir_all(&images_dir);

        // Create files with different extensions
        let test_cases = vec![
            ("image_123.png", "movie", 10, "movie_10_poster.png"),
            ("image_456.webp", "series", 20, "series_20_poster.webp"),
            ("image_789.gif", "book", 30, "book_30_cover.gif"),
        ];

        for (temp_name, media_type, id, expected_name) in test_cases {
            let temp_path = images_dir.join(temp_name);
            {
                let mut f = File::create(&temp_path).unwrap();
                f.write_all(b"fake image data").unwrap();
            }

            let result = finalize_image(images_dir.to_str().unwrap(), temp_name, media_type, id);

            assert!(result.is_ok());
            let final_filename = result.unwrap();
            assert_eq!(final_filename, expected_name);
            assert!(images_dir.join(&final_filename).exists());
            assert!(!temp_path.exists());
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_finalize_image_no_extension_fails() {
        let temp_dir = get_unique_temp_dir();
        let images_dir = temp_dir.join("images");
        let _ = fs::create_dir_all(&images_dir);

        // Create a file with no extension
        let temp_filename = "image_no_extension";
        let temp_path = images_dir.join(temp_filename);
        {
            let mut f = File::create(&temp_path).unwrap();
            f.write_all(b"fake image data").unwrap();
        }

        let result = finalize_image(images_dir.to_str().unwrap(), temp_filename, "book", 42);

        assert!(result.is_err());
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_finalize_image_nonexistent_file() {
        let temp_dir = get_unique_temp_dir();
        let images_dir = temp_dir.join("images");
        let _ = fs::create_dir_all(&images_dir);

        let result = finalize_image(images_dir.to_str().unwrap(), "nonexistent.jpg", "movie", 99);

        assert!(result.is_err());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}

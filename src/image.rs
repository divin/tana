//! Image file management module for Tana
//!
//! Handles validation, copying, and directory management for image files.
//! Supported image formats: PNG, JPG, JPEG, WebP, GIF, BMP

use crate::error::{Result, TanaError};
use std::fs;
use std::path::PathBuf;

/// Supported image file extensions
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

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

    Ok(dest_file.to_string_lossy().to_string())
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
        let dest_file = PathBuf::from(result.unwrap());
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
}

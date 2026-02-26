use crate::config::HeicConverterConfig;
use std::path::Path;
use std::process::Command;

pub struct HeicConverter {
    config: HeicConverterConfig,
}

impl HeicConverter {
    pub fn new(config: HeicConverterConfig) -> Self {
        Self { config }
    }

    pub fn convert(&self, input_path: &Path) -> anyhow::Result<Option<std::path::PathBuf>> {
        if !self.config.generate_jpeg {
            return Ok(None);
        }

        // Check if file is HEIC
        let extension = input_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        if extension.as_deref() != Some("heic") && extension.as_deref() != Some("heif") {
            return Ok(None);
        }

        let output_path = input_path.with_extension("jpg");

        match self.config.backend.as_str() {
            "image" => self.convert_with_image(input_path, &output_path),
            "libheif" => self.convert_with_libheif(input_path, &output_path),
            "sips" => self.convert_with_sips(input_path, &output_path),
            _ => {
                eprintln!(
                    "Unknown converter backend: {}, falling back to image",
                    self.config.backend
                );
                self.convert_with_image(input_path, &output_path)
            }
        }
    }

    fn convert_with_image(
        &self,
        _input: &Path,
        _output: &Path,
    ) -> anyhow::Result<Option<std::path::PathBuf>> {
        // Note: image crate has limited HEIC support
        // This is a placeholder - in production, use libheif-rs
        eprintln!(
            "Image crate HEIC support is limited. Consider using 'libheif' or 'sips' backend."
        );

        // For now, just return None to indicate no conversion
        // In a real implementation, you'd use libheif-rs crate
        Ok(None)
    }

    fn convert_with_libheif(
        &self,
        _input: &Path,
        _output: &Path,
    ) -> anyhow::Result<Option<std::path::PathBuf>> {
        // This would require libheif-sys or libheif-rs
        // Placeholder implementation
        eprintln!("libheif backend not yet implemented");
        Ok(None)
    }

    fn convert_with_sips(
        &self,
        input: &Path,
        output: &Path,
    ) -> anyhow::Result<Option<std::path::PathBuf>> {
        let quality = self.config.jpeg_quality;

        let status = Command::new("sips")
            .args([
                "-s",
                "format jpeg",
                "-s",
                &format!("formatOptions {}", quality),
                "-o",
                output.to_str().unwrap(),
                input.to_str().unwrap(),
            ])
            .status()?;

        if status.success() {
            Ok(Some(output.to_path_buf()))
        } else {
            Err(anyhow::anyhow!("sips conversion failed"))
        }
    }
}

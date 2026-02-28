use crate::config::Config;
use image::imageops::FilterType;
use image::GenericImageView;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};

pub struct ThumbnailGenerator;

impl ThumbnailGenerator {
    /// 获取缩略图存储目录
    pub fn thumbnail_dir(config: &Config) -> PathBuf {
        config.storage.base_path.join(".thumbnails")
    }

    /// 生成缩略图
    ///
    /// # Arguments
    /// * `img_path` - 原图路径
    /// * `config` - 服务器配置
    /// * `max_size` - 缩略图最大边长（默认 300）
    ///
    /// # Returns
    /// * (缩略图路径, 原图宽度, 原图高度)
    pub async fn generate(
        img_path: &Path,
        config: &Config,
        max_size: u32,
    ) -> Result<(PathBuf, i32, i32)> {
        // 创建缩略图目录
        let thumbnail_dir = Self::thumbnail_dir(config);
        tokio::fs::create_dir_all(&thumbnail_dir).await?;

        // 生成缩略图文件名（使用 UUID 避免冲突）
        let thumbnail_filename = format!("thumb_{}.jpg", uuid::Uuid::new_v4());
        let thumbnail_path = thumbnail_dir.join(&thumbnail_filename);

        // 在阻塞线程中执行图像处理
        let img_path = img_path.to_path_buf();
        let thumbnail_path_clone = thumbnail_path.clone();

        let (width, height) = tokio::task::spawn_blocking(move || -> Result<(i32, i32)> {
            // 打开原图
            let img = image::open(&img_path)
                .with_context(|| format!("Failed to open image: {:?}", img_path))?;

            // 获取原图尺寸
            let (orig_width, orig_height) = img.dimensions();

            // 计算缩放比例
            let ratio = if orig_width > orig_height {
                max_size as f32 / orig_width as f32
            } else {
                max_size as f32 / orig_height as f32
            };

            let new_width = (orig_width as f32 * ratio) as u32;
            let new_height = (orig_height as f32 * ratio) as u32;

            // 生成缩略图（使用 Lanczos3 算法保持质量）
            let thumbnail = img.resize(new_width, new_height, FilterType::Lanczos3);

            // 保存缩略图
            thumbnail.save(&thumbnail_path_clone)
                .with_context(|| format!("Failed to save thumbnail: {:?}", thumbnail_path_clone))?;

            Ok((orig_width as i32, orig_height as i32))
        }).await??;

        Ok((thumbnail_path, width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_thumbnail_dir() {
        let config = Config::default();
        let dir = ThumbnailGenerator::thumbnail_dir(&config);
        assert!(dir.to_string_lossy().contains(".thumbnails"));
    }
}

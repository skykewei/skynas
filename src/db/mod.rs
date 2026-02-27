use crate::models::*;
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS photos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                album TEXT NOT NULL DEFAULT '未分类',
                file_hash TEXT UNIQUE,
                size_bytes INTEGER NOT NULL,
                created_at TIMESTAMP,
                uploaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                local_path TEXT NOT NULL,
                has_jpeg_variant BOOLEAN DEFAULT FALSE
            );

            CREATE INDEX IF NOT EXISTS idx_photos_hash ON photos(file_hash);
            CREATE INDEX IF NOT EXISTS idx_photos_album ON photos(album);

            CREATE TABLE IF NOT EXISTS sync_operations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                completed_at TIMESTAMP,
                total_files INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                fail_count INTEGER DEFAULT 0,
                client_ip TEXT,
                user_agent TEXT
            );

            CREATE TABLE IF NOT EXISTS sync_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                photo_id INTEGER NOT NULL REFERENCES photos(id),
                operation_id INTEGER NOT NULL REFERENCES sync_operations(id),
                status TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'synced', 'failed')),
                synced_at TIMESTAMP,
                error_msg TEXT
            );

            CREATE TABLE IF NOT EXISTS upload_chunks (
                upload_id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                album TEXT NOT NULL,
                total_size INTEGER NOT NULL,
                chunk_index INTEGER NOT NULL,
                total_chunks INTEGER NOT NULL,
                received_bytes INTEGER DEFAULT 0,
                completed BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                temp_path TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_upload_chunks_completed ON upload_chunks(upload_id, completed);
            "#
        )?;

        // 尝试添加新列（如果不存在则忽略错误）
        let _ = self.conn.execute(
            "ALTER TABLE photos ADD COLUMN thumbnail_path TEXT",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE photos ADD COLUMN width INTEGER",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE photos ADD COLUMN height INTEGER",
            [],
        );

        // 新增 upload_tasks 表
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS upload_tasks (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                album TEXT NOT NULL,
                total_bytes INTEGER NOT NULL DEFAULT 0,
                received_bytes INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                cancelled INTEGER DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_upload_tasks_status ON upload_tasks(status);

            CREATE TABLE IF NOT EXISTS admin_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                jwt_secret TEXT NOT NULL,
                admin_password_hash TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            "#
        )?;

        Ok(())
    }

    // Photo operations
    pub fn insert_photo(&self, photo: &Photo) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO photos (filename, album, file_hash, size_bytes, created_at, local_path, has_jpeg_variant, thumbnail_path, width, height)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(file_hash) DO UPDATE SET
                 uploaded_at = excluded.uploaded_at
             RETURNING id",
            params![
                photo.filename,
                photo.album,
                photo.file_hash,
                photo.size_bytes,
                photo.created_at,
                photo.local_path,
                photo.has_jpeg_variant,
                photo.thumbnail_path,
                photo.width,
                photo.height,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    #[allow(dead_code)]
    pub fn find_photo_by_hash(&self, file_hash: &str) -> Result<Option<Photo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, album, file_hash, size_bytes, created_at, uploaded_at, local_path, has_jpeg_variant, thumbnail_path, width, height
             FROM photos WHERE file_hash = ?1"
        )?;
        let mut rows = stmt.query(params![file_hash])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Photo {
                id: row.get(0)?,
                filename: row.get(1)?,
                album: row.get(2)?,
                file_hash: row.get(3)?,
                size_bytes: row.get(4)?,
                created_at: row.get(5)?,
                uploaded_at: row.get(6)?,
                local_path: row.get(7)?,
                has_jpeg_variant: row.get(8)?,
                thumbnail_path: row.get(9).ok(),
                width: row.get(10).ok(),
                height: row.get(11).ok(),
            }))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn list_photos_by_album(&self, album: &str) -> Result<Vec<Photo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, album, file_hash, size_bytes, created_at, uploaded_at, local_path, has_jpeg_variant, thumbnail_path, width, height
             FROM photos WHERE album = ?1 ORDER BY uploaded_at DESC"
        )?;
        let rows = stmt.query_map(params![album], |row| {
            Ok(Photo {
                id: row.get(0)?,
                filename: row.get(1)?,
                album: row.get(2)?,
                file_hash: row.get(3)?,
                size_bytes: row.get(4)?,
                created_at: row.get(5)?,
                uploaded_at: row.get(6)?,
                local_path: row.get(7)?,
                has_jpeg_variant: row.get(8)?,
                thumbnail_path: row.get(9).ok(),
                width: row.get(10).ok(),
                height: row.get(11).ok(),
            })
        })?;

        let mut photos = Vec::new();
        for row in rows {
            photos.push(row?);
        }
        Ok(photos)
    }

    // Chunked upload operations
    pub fn create_upload_session(
        &self,
        upload_id: &str,
        filename: &str,
        album: &str,
        total_size: i64,
        total_chunks: i32,
        temp_path: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO upload_chunks (upload_id, filename, album, total_size, chunk_index, total_chunks, temp_path)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6)
             ON CONFLICT(upload_id) DO UPDATE SET
                 filename = excluded.filename,
                 album = excluded.album,
                 total_size = excluded.total_size,
                 total_chunks = excluded.total_chunks,
                 temp_path = excluded.temp_path",
            params![upload_id, filename, album, total_size, total_chunks, temp_path],
        )?;
        Ok(())
    }

    pub fn update_upload_progress(
        &self,
        upload_id: &str,
        chunk_index: i32,
        received_bytes: i64,
        completed: bool,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE upload_chunks SET chunk_index = ?1, received_bytes = ?2, completed = ?3
             WHERE upload_id = ?4",
            params![chunk_index, received_bytes, completed, upload_id],
        )?;
        Ok(())
    }

    pub fn get_upload_session(&self, upload_id: &str) -> Result<Option<UploadChunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT upload_id, filename, album, total_size, chunk_index, total_chunks,
                    received_bytes, completed, created_at, temp_path
             FROM upload_chunks WHERE upload_id = ?1",
        )?;
        let mut rows = stmt.query(params![upload_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(UploadChunk {
                upload_id: row.get(0)?,
                filename: row.get(1)?,
                album: row.get(2)?,
                total_size: row.get(3)?,
                chunk_index: row.get(4)?,
                total_chunks: row.get(5)?,
                received_bytes: row.get(6)?,
                completed: row.get(7)?,
                created_at: row.get(8)?,
                temp_path: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_upload_session(&self, upload_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM upload_chunks WHERE upload_id = ?1",
            params![upload_id],
        )?;
        Ok(())
    }

    // Cleanup old incomplete uploads
    #[allow(dead_code)]
    pub fn cleanup_old_uploads(&self, hours: i64) -> Result<usize> {
        let rows_affected = self.conn.execute(
            "DELETE FROM upload_chunks
             WHERE completed = FALSE
             AND created_at < datetime('now', ?1 || ' hours')",
            params![-hours],
        )?;
        Ok(rows_affected)
    }

    // Upload Task operations
    pub fn create_upload_task(&self, task: &UploadTask) -> Result<()> {
        self.conn.execute(
            "INSERT INTO upload_tasks (id, filename, album, total_bytes, received_bytes, status, created_at, updated_at, cancelled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                 received_bytes = excluded.received_bytes,
                 status = excluded.status,
                 updated_at = excluded.updated_at,
                 cancelled = excluded.cancelled",
            params![
                task.id, task.filename, task.album, task.total_bytes,
                task.received_bytes, task.status.as_str(), task.created_at,
                task.updated_at, task.cancelled as i32
            ],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_upload_task(&self, id: &str) -> Result<Option<UploadTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, album, total_bytes, received_bytes, status, created_at, updated_at, cancelled
             FROM upload_tasks WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(UploadTask {
                id: row.get(0)?,
                filename: row.get(1)?,
                album: row.get(2)?,
                total_bytes: row.get(3)?,
                received_bytes: row.get(4)?,
                status: match row.get::<_, String>(5)?.as_str() {
                    "uploading" => TaskStatus::Uploading,
                    "completed" => TaskStatus::Completed,
                    "cancelled" => TaskStatus::Cancelled,
                    "error" => TaskStatus::Error,
                    _ => TaskStatus::Pending,
                },
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                cancelled: row.get::<_, i32>(8)? != 0,
            }))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn list_active_uploads(&self) -> Result<Vec<UploadTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, filename, album, total_bytes, received_bytes, status, created_at, updated_at, cancelled
             FROM upload_tasks WHERE status IN ('pending', 'uploading') ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(UploadTask {
                id: row.get(0)?,
                filename: row.get(1)?,
                album: row.get(2)?,
                total_bytes: row.get(3)?,
                received_bytes: row.get(4)?,
                status: match row.get::<_, String>(5)?.as_str() {
                    "uploading" => TaskStatus::Uploading,
                    "completed" => TaskStatus::Completed,
                    "cancelled" => TaskStatus::Cancelled,
                    "error" => TaskStatus::Error,
                    _ => TaskStatus::Pending,
                },
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                cancelled: row.get::<_, i32>(8)? != 0,
            })
        })?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(row?);
        }
        Ok(tasks)
    }

    #[allow(dead_code)]
    pub fn cleanup_incomplete_uploads(&self) -> Result<usize> {
        let rows_affected = self.conn.execute(
            "DELETE FROM upload_tasks WHERE status IN ('pending', 'uploading', 'error')",
            [],
        )?;
        Ok(rows_affected)
    }

    // Admin Config operations
    #[allow(dead_code)]
    pub fn get_or_create_admin_config(&self, default_secret: &str) -> Result<AdminConfig> {
        let mut stmt = self.conn.prepare(
            "SELECT id, jwt_secret, admin_password_hash, created_at, updated_at FROM admin_config WHERE id = 1"
        )?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            Ok(AdminConfig {
                id: row.get(0)?,
                jwt_secret: row.get(1)?,
                admin_password_hash: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        } else {
            // 创建默认配置
            let default_hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST).unwrap_or_default();
            self.conn.execute(
                "INSERT INTO admin_config (id, jwt_secret, admin_password_hash) VALUES (1, ?1, ?2)",
                params![default_secret, default_hash],
            )?;

            Ok(AdminConfig {
                id: 1,
                jwt_secret: default_secret.to_string(),
                admin_password_hash: default_hash,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }
    }
}

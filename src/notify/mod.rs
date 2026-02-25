use notify_rust::Notification;

pub fn show_upload_complete(count: usize, album: &str) {
    let _ = Notification::new()
        .summary("照片同步完成")
        .body(&format!("成功同步 {} 张照片到「{}」相册", count, album))
        .appname("SkyNAS")
        .timeout(5000)
        .show();
}

pub fn show_sync_started(album: &str) {
    let _ = Notification::new()
        .summary("开始同步照片")
        .body(&format!("正在同步到「{}」相册", album))
        .appname("SkyNAS")
        .timeout(3000)
        .show();
}

pub fn show_cloud_sync_complete(success: bool) {
    if success {
        let _ = Notification::new()
            .summary("云同步完成")
            .body("照片已备份到云端/NAS")
            .appname("SkyNAS")
            .timeout(3000)
            .show();
    } else {
        let _ = Notification::new()
            .summary("云同步失败")
            .body("请检查网络连接或配置")
            .appname("SkyNAS")
            .timeout(5000)
            .show();
    }
}

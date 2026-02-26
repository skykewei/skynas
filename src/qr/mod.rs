use qrcode::QrCode;
use qrcode::render::unicode;

pub fn generate_qr_string(data: &str) -> anyhow::Result<String> {
    let code = QrCode::new(data)?;

    let string = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .module_dimensions(1, 1)
        .build();

    Ok(string)
}

pub fn print_server_info(host: &str, port: u16) {
    let url = format!("http://{}:{}", host, port);

    println!("\n{}", "=".repeat(50));
    println!("  SkyNAS Photo Sync Server");
    println!("{}", "=".repeat(50));

    match generate_qr_string(&url) {
        Ok(qr) => {
            println!("\nScan QR code with your iPhone:\n");
            for line in qr.lines() {
                println!("  {}", line);
            }
        }
        Err(e) => {
            eprintln!("Failed to generate QR code: {}", e);
        }
    }

    println!("\n  Or visit this URL:\n");
    println!("  {}", url);
    println!("\n{}", "=".repeat(50));
}

pub fn get_local_ip() -> Option<String> {
    match local_ip_address::local_ip() {
        Ok(ip) => Some(ip.to_string()),
        Err(_) => None,
    }
}

pub fn get_best_host(preferred: &str) -> String {
    if preferred != "0.0.0.0" && preferred != "127.0.0.1" && preferred != "localhost" {
        return preferred.to_string();
    }

    get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_generation() {
        let url = "http://192.168.1.100:8080";
        let qr = generate_qr_string(url);
        assert!(qr.is_ok());
        let qr_str = qr.unwrap();
        assert!(!qr_str.is_empty());
    }
}

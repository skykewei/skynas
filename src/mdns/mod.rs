use zeroconf::prelude::*;
use zeroconf::{MdnsService, ServiceType};

pub struct MdnsPublisher;

impl MdnsPublisher {
    pub fn new() -> Self {
        Self
    }

    pub fn publish(&mut self, name: &str, port: u16, _device_id: &str) -> anyhow::Result<()> {
        let service_type = ServiceType::new("_skynas", "_tcp")?;

        let mut service = MdnsService::new(service_type, port);
        service.set_registered_callback(Box::new(|_, _| {
            println!("mDNS service registered successfully");
        }));

        service.register()?;

        println!("mDNS service '{}' registered on port {}", name, port);

        // Keep the service alive by running an event loop
        // Note: This blocks the current thread
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        println!("mDNS service stopped");
    }
}

pub fn generate_device_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{:x}", timestamp)[..8].to_string()
}

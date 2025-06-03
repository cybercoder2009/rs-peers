use std::{
    net::SocketAddr, 
    str::FromStr
};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn parse_ip_port(addr: &str) -> Option<(String, u16)> {
    let addr = SocketAddr::from_str(addr).ok()?;
    let ip = addr.ip().to_string();
    let port = addr.port();
    Some((ip, port))
}

pub fn hhmmss() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let total_secs = now.as_secs();
    let hours = (total_secs / 3600) % 24;
    let minutes = (total_secs / 60) % 60;
    let seconds = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH) 
        .expect("Time went backwards")
        .as_secs()
}
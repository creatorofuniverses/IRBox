/// Liveness of a bound network interface, for the UI status indicator.
/// - "up": interface present and not administratively down
/// - "down": interface absent, or operstate == "down"
/// - "unknown": platform without a presence check (non-Linux/Android)
pub fn interface_status(name: &str) -> &'static str {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let dir = format!("/sys/class/net/{}", name);
        if !std::path::Path::new(&dir).exists() {
            return "down"; // not present (e.g. typo'd bind target, or tunnel not brought up)
        }
        // WireGuard/TUN devices report operstate "unknown" while fully up, so treat
        // anything that isn't an explicit "down" as up once the device exists.
        match std::fs::read_to_string(format!("{}/operstate", dir)) {
            Ok(s) if s.trim() == "down" => "down",
            _ => "up",
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = name;
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Loopback always exists on Linux/Android (operstate "unknown" → treated as up);
    // a clearly-bogus name is absent → down.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn loopback_is_up_and_missing_is_down() {
        assert_eq!(interface_status("lo"), "up");
        assert_eq!(interface_status("nonexistent-iface-zzz"), "down");
    }
}

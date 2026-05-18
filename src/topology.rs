use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::process::Command;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

/// Network node types for UI display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EndpointType {
    Router,   // The router itself
    Wired,    // Wired client
    Wireless, // Wi-Fi client
    Vpn,      // VPN client
    Remote,   // Remote server (Internet)
    #[default]
    Unknown, // Could not be determined
}

impl EndpointType {
    /// Returns an icon for the UI.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Router => "🖧 ",
            Self::Wired => "💻",
            Self::Wireless => "📶",
            Self::Vpn => "🛡️",
            Self::Remote => "🌐",
            Self::Unknown => "❓",
        }
    }
}

/// Background service for updating the network topology.
pub struct Topology {
    map: Arc<RwLock<HashMap<String, EndpointType>>>,
}

impl Topology {
    pub fn new() -> Self {
        let map = Arc::new(RwLock::new(HashMap::new()));
        let map_clone = Arc::clone(&map);

        // Start a background thread to avoid blocking the UI with command executions.
        thread::spawn(move || {
            loop {
                let new_map = Self::build_topology();
                if let Ok(mut w) = map_clone.write() {
                    *w = new_map;
                }
                thread::sleep(Duration::from_secs(5)); // Update every 5 seconds
            }
        });

        Self { map }
    }

    /// Gets the client type by IP address from the hot cache.
    pub fn get_type(&self, ip: &str) -> EndpointType {
        if let Ok(r) = self.map.read() {
            if let Some(&ep_type) = r.get(ip) {
                return ep_type;
            }
        }

        // If the IP is not in the local tables, check if it's a public IP.
        if let Ok(parsed_ip) = ip.parse::<IpAddr>() {
            if Self::is_global(parsed_ip) {
                return EndpointType::Remote;
            }
        }

        EndpointType::Unknown
    }

    fn build_topology() -> HashMap<String, EndpointType> {
        let mut map = HashMap::new();

        // Find the router's own IP addresses (Local).
        if let Ok(output) = Command::new("ip").args(["addr"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for token in stdout.split_whitespace() {
                if token.contains('/') {
                    let ip_str = token.split('/').next().unwrap_or("");
                    if ip_str.parse::<IpAddr>().is_ok() {
                        map.insert(ip_str.to_string(), EndpointType::Router);
                    }
                }
            }
        }

        // Poll the Wi-Fi driver for all associated clients.
        let wifi_macs = Self::get_wifi_macs();

        // Read the ARP table (Neighbor table) for IP -> MAC and IP -> Interface mappings.
        if let Ok(output) = Command::new("ip").args(["neigh", "show"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 && parts[1] == "dev" {
                    let ip = parts[0];
                    let dev = parts[2];

                    // Extract the MAC address (lladdr).
                    let mac = parts
                        .iter()
                        .position(|&x| x == "lladdr")
                        .and_then(|i| parts.get(i + 1))
                        .map(|s| s.to_lowercase());

                    // If the client is on a network bridge (br-lan, br0, etc.).
                    if dev.starts_with("br-") || dev == "bridge" || dev == "br0" {
                        if let Some(m) = &mac {
                            if wifi_macs.contains(m) {
                                map.insert(ip.to_string(), EndpointType::Wireless);
                            } else {
                                // If the MAC is not in the Wi-Fi clients list, it's a wired LAN client.
                                map.insert(ip.to_string(), EndpointType::Wired);
                            }
                        }
                    } else {
                        // For isolated interfaces (not part of a bridge).
                        map.insert(ip.to_string(), Self::classify_interface(dev));
                    }
                }
            }
        }

        map
    }

    /// Extracts MAC addresses of all wireless clients via `iw`.
    fn get_wifi_macs() -> HashSet<String> {
        let mut macs = HashSet::new();

        // Find all Wi-Fi interfaces (wlan0, wlan1, phy0-ap0, ra0, etc.).
        let mut wifi_ifaces = Vec::new();
        if let Ok(output) = Command::new("iw").args(["dev"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0] == "Interface" {
                    wifi_ifaces.push(parts[1].to_string());
                }
            }
        }

        // For each Wi-Fi interface, dump the list of connected stations.
        for iface in wifi_ifaces {
            if let Ok(output) = Command::new("iw")
                .args(["dev", &iface, "station", "dump"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    // Line format: "Station 34:ce:00:86:a0:c4 (on wlan0)".
                    if parts.len() >= 2 && parts[0] == "Station" {
                        macs.insert(parts[1].to_lowercase());
                    }
                }
            }
        }

        macs
    }

    fn classify_interface(dev: &str) -> EndpointType {
        if dev.starts_with("wl")
            || dev.starts_with("ra")
            || dev.starts_with("ath")
            || dev.starts_with("mt76")
            || dev.starts_with("phy")
        {
            EndpointType::Wireless
        } else if dev.starts_with("eth")
            || dev.starts_with("lan")
            || dev.starts_with("sw")
            || dev.starts_with("en")
        {
            EndpointType::Wired
        } else if dev.starts_with("wg")
            || dev.starts_with("tun")
            || dev.starts_with("tap")
            || dev.starts_with("tailscale")
        {
            EndpointType::Vpn
        } else {
            EndpointType::Unknown
        }
    }

    /// Checks whether the IP address is global (i.e., routable on the Internet).
    fn is_global(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                !ipv4.is_private()
                    && !ipv4.is_loopback()
                    && !ipv4.is_link_local()
                    && !ipv4.is_broadcast()
                    && !ipv4.is_documentation()
                    && ipv4 != std::net::Ipv4Addr::new(0, 0, 0, 0)
            }
            IpAddr::V6(ipv6) => {
                !ipv6.is_loopback() && ipv6 != std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)
            }
        }
    }
}

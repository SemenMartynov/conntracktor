use crate::topology::{EndpointType, Topology};
use ratatui::style::Color;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

/// Indicates whether a flow is processed by the CPU or hardware offloaded.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OffloadStatus {
    Software, // Software Flow Offload
    Hardware, // Hardware Offload (PPE)
    #[default]
    None, // CPU (Slow Path)
}

impl OffloadStatus {
    /// Returns the UI display color associated with the offload type.
    pub fn color(&self) -> Color {
        match self {
            OffloadStatus::Software => Color::Yellow,
            OffloadStatus::Hardware => Color::Green,
            OffloadStatus::None => Color::DarkGray,
        }
    }

    /// Returns the label representation for the UI.
    pub fn label(&self) -> &'static str {
        match self {
            OffloadStatus::Software => "SOFTWARE",
            OffloadStatus::Hardware => "HW (PPE)",
            OffloadStatus::None => "CPU",
        }
    }
}

/// Represents an active network connection entry.
#[derive(Debug, Clone)]
pub struct Connection {
    pub protocol: String,
    pub src_ip: String,
    pub src_type: EndpointType,
    pub dst_ip: String,
    pub dst_type: EndpointType,
    pub status: String,
    pub offload: OffloadStatus,
}

/// Statistics for connection tracking.
#[derive(Default, Debug)]
pub struct ConntrackStats {
    pub total: u32,
    pub max: u32,
    pub cpu: u32,
    pub software: u32,
    pub hardware: u32,
    /// Parsed list of active connections.
    pub connections: Vec<Connection>,
}

/// Reads connection tracking statistics from sysfs/procfs.
pub fn get_stats(topology: &Topology) -> io::Result<ConntrackStats> {
    // Read the maximum connection limit, falling back to a default if unavailable.
    let max_str = fs::read_to_string("/proc/sys/net/netfilter/nf_conntrack_max")
        .unwrap_or_else(|_| "16384".to_string());

    let max = max_str.trim().parse().unwrap_or(16384);

    let file = File::open("/proc/net/nf_conntrack");

    let mut total = 0;
    let mut cpu = 0;
    let mut software = 0;
    let mut hardware = 0;
    let mut connections = Vec::new();

    match file {
        Ok(f) => {
            let reader = BufReader::new(f);

            for line in reader.lines() {
                let line = line?;
                total += 1;

                if let Some(conn) = parse_conntrack_line(&line, topology) {
                    match conn.offload {
                        OffloadStatus::None => cpu += 1,
                        OffloadStatus::Software => software += 1,
                        OffloadStatus::Hardware => hardware += 1,
                    }
                    connections.push(conn);
                }
            }
        }
        Err(_) => {
            // Provide dummy data when /proc/net/nf_conntrack is unavailable (e.g., non-Linux environments).
            total = 4;
            cpu = 1;
            software = 1;
            hardware = 2;

            connections.push(Connection {
                protocol: "tcp".to_string(),
                src_ip: "192.168.1.15".to_string(),
                src_type: EndpointType::Wireless,
                dst_ip: "142.250.186.46".to_string(),
                dst_type: EndpointType::Remote,
                status: "ESTABLISHED".to_string(),
                offload: OffloadStatus::Hardware,
            });
            connections.push(Connection {
                protocol: "udp".to_string(),
                src_ip: "192.168.1.50".to_string(),
                src_type: EndpointType::Wireless,
                dst_ip: "104.16.124.96".to_string(),
                dst_type: EndpointType::Remote,
                status: "ASSURED".to_string(),
                offload: OffloadStatus::Hardware,
            });
            connections.push(Connection {
                protocol: "tcp".to_string(),
                src_ip: "192.168.1.100".to_string(),
                src_type: EndpointType::Wired,
                dst_ip: "8.8.8.8".to_string(),
                dst_type: EndpointType::Remote,
                status: "TIME_WAIT".to_string(),
                offload: OffloadStatus::Software,
            });
            connections.push(Connection {
                protocol: "udp".to_string(),
                src_ip: "192.168.1.1".to_string(),
                src_type: EndpointType::Wireless,
                dst_ip: "91.189.91.157".to_string(),
                dst_type: EndpointType::Remote,
                status: "UNREPLIED".to_string(),
                offload: OffloadStatus::None,
            });
        }
    }

    Ok(ConntrackStats {
        total,
        max,
        cpu,
        software,
        hardware,
        connections,
    })
}

/// Classifies the offload status of a conntrack line based on its status tags.
///
/// Hardware-offloaded flows in OpenWrt/MediaTek kernels are tagged with `[HW_OFFLOAD]`,
/// while OpenWrt's software flow offloading uses the `[OFFLOAD]` tag.
fn classify_offload(line: &str) -> OffloadStatus {
    if line.contains("[HW_OFFLOAD]") {
        // Hardware offloaded to the PPE.
        OffloadStatus::Hardware
    } else if line.contains("[OFFLOAD]") {
        // OpenWrt software flow offloading tag.
        OffloadStatus::Software
    } else {
        OffloadStatus::None
    }
}

/// Parses a single line from `/proc/net/nf_conntrack`.
fn parse_conntrack_line(line: &str, topology: &Topology) -> Option<Connection> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }

    let protocol = parts[2].to_string();

    // TCP includes state at index 5 (e.g., ESTABLISHED); for non-TCP, search for state flags.
    let status = if protocol == "tcp" {
        parts[5].to_string()
    } else {
        parts
            .iter()
            .find(|&&p| p == "UNREPLIED" || p == "ASSURED")
            .unwrap_or(&"")
            .to_string()
    };

    let src_ip = parts
        .iter()
        .find(|&&p| p.starts_with("src="))
        .map(|s| s.replace("src=", ""))
        .unwrap_or_default();
    let src_type = topology.get_type(&src_ip);

    let dst_ip = parts
        .iter()
        .find(|&&p| p.starts_with("dst="))
        .map(|s| s.replace("dst=", ""))
        .unwrap_or_default();
    let dst_type = topology.get_type(&dst_ip);

    Some(Connection {
        protocol,
        src_ip,
        src_type,
        dst_ip,
        dst_type,
        status,
        offload: classify_offload(line),
    })
}

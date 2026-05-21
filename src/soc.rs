use std::fs;

/// Represents the detected System on Chip (SoC) model of the device.
#[derive(Debug, PartialEq, Default)]
pub enum SocModel {
    Mt7988, // MediaTek Filogic 880 (Wi-Fi 7)
    Mt7986, // MediaTek Filogic 830 (Wi-Fi 6)
    Mt7981, // MediaTek Filogic 820 (Wi-Fi 6)
    Mt7621, // MediaTek Legacy (Wi-Fi 5 / MIPS)
    #[default]
    Unknown,
}

impl SocModel {
    /// Detects the SoC model by reading the Linux device tree `compatible` node.
    pub fn detect() -> Self {
        // The `compatible` node contains null-terminated strings. We read raw bytes
        // to safely handle null bytes without string parsing errors.
        if let Ok(bytes) = fs::read("/sys/firmware/devicetree/base/compatible") {
            let content = String::from_utf8_lossy(&bytes).to_lowercase();

            if content.contains("mt7988") {
                return Self::Mt7988;
            } else if content.contains("mt7986") {
                return Self::Mt7986;
            } else if content.contains("mt7981") {
                return Self::Mt7981;
            } else if content.contains("mt7621") {
                return Self::Mt7621;
            }
        }

        Self::Unknown
    }

    /// Returns a human-readable display name for the UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Mt7988 => "MT7988 (Filogic 880)",
            Self::Mt7986 => "MT7986 (Filogic 830)",
            Self::Mt7981 => "MT7981 (Filogic 820)",
            Self::Mt7621 => "MT7621 (MIPS)",
            Self::Unknown => "Unknown SoC",
        }
    }

    /// Returns the number of Packet Processing Engines (PPE) available on the SoC.
    pub fn ppe_count(&self) -> usize {
        match self {
            Self::Mt7988 => 3,
            Self::Mt7986 => 2,
            Self::Mt7981 => 1,
            Self::Mt7621 => 1,
            Self::Unknown => 0,
        }
    }

    /// Returns the number of Wireless Ethernet Dispatcher (WED) modules available on the SoC.
    pub fn wed_count(&self) -> usize {
        match self {
            Self::Mt7988 => 3,
            Self::Mt7986 => 2,
            Self::Mt7981 => 1,
            Self::Mt7621 => 0,
            Self::Unknown => 0,
        }
    }
}

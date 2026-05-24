use std::fs;

/// Represents the detected System on Chip (SoC) model of the device.
#[derive(Debug, PartialEq, Default, Clone, Copy)]
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
            let content = String::from_utf8_lossy(&bytes)
                .trim_matches('\0')
                .to_lowercase();

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

    /// Returns the CPU architecture and core count of the SoC.
    pub fn architecture(&self) -> &'static str {
        match self {
            Self::Mt7988 => "4× Cortex-A73",
            Self::Mt7986 => "4× Cortex-A53",
            Self::Mt7981 => "2× Cortex-A53",
            Self::Mt7621 => "2× MIPS1004Kc",
            Self::Unknown => "Unknown",
        }
    }

    /// Returns the maximum CPU core frequency in Hertz (Hz).
    /// Returns 0 if the SoC model is unknown.
    pub fn core_frequency_hz(&self) -> u64 {
        match self {
            Self::Mt7988 => 1_800_000_000, // 1.8 GHz
            Self::Mt7986 => 2_000_000_000, // 2.0 GHz
            Self::Mt7981 => 1_300_000_000, // 1.3 GHz
            Self::Mt7621 => 880_000_000,   // 880 MHz
            Self::Unknown => 0,
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

    /// Indicates whether the network subsystem supports Receive Side Scaling (RSS).
    pub fn has_rss(&self) -> bool {
        match self {
            // Modern Filogic platforms support advanced RSS on their PPE/switch.
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 => true,
            // Legacy MIPS lacks native multi-queue RSS in the same manner.
            Self::Mt7621 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC hardware supports IP/TCP/UDP Checksum Offload.
    pub fn has_checksum_offload(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 | Self::Mt7621 => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC supports TCP Segmentation Offload (TSO) in hardware.
    pub fn has_tso(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 | Self::Mt7621 => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the network interface controller (NIC) supports Multiple RX Queues.
    pub fn has_multi_rx_queues(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 => true,
            Self::Mt7621 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC is equipped with a dedicated Hardware Crypto Engine.
    pub fn has_crypto_engine(&self) -> bool {
        match self {
            // Filogic uses ARM cryptographic extensions or dedicated EIP modules,
            // MT7621 uses an internal EIP93 crypto accelerator.
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 | Self::Mt7621 => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC hardware provides acceleration for the AES cipher.
    pub fn has_aes_acceleration(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 | Self::Mt7621 => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC hardware provides acceleration for the SHA hashing algorithms.
    pub fn has_sha_acceleration(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 | Self::Mt7621 => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC contains an integrated True Random Number Generator (TRNG / HW RNG).
    pub fn has_trng(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 | Self::Mt7621 => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC supports Secure Boot mechanisms.
    pub fn has_secure_boot(&self) -> bool {
        match self {
            // ARM-based Filogic platforms support ATF (ARM Trusted Firmware) and Secure Boot.
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 => true,
            Self::Mt7621 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC supports ARM TrustZone or a Trusted Execution Environment (TEE).
    pub fn has_trustzone(&self) -> bool {
        match self {
            // Cortex-A processors support TrustZone (e.g., OP-TEE).
            Self::Mt7988 | Self::Mt7986 | Self::Mt7981 => true,
            // Legacy MIPS processors do not implement ARM TrustZone.
            Self::Mt7621 | Self::Unknown => false,
        }
    }
}

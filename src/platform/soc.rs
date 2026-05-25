use std::fs;

/// Represents the detected System on Chip (SoC) model of the device.
#[derive(Debug, PartialEq, Default, Clone, Copy)]
pub enum SocModel {
    Mt7988, // MediaTek Filogic 880 (Wi-Fi 7)
    Mt7987, // MediaTek Filogic 850 (Wi-Fi 7 entry)
    Mt7986, // MediaTek Filogic 830 (Wi-Fi 6/6E)
    Mt7981, // MediaTek Filogic 820 (Wi-Fi 6)
    Mt7622, // MediaTek Filogic 800 (Wi-Fi 6 / early ARM)
    Mt7629, // MediaTek MT7629 (Wi-Fi 5)
    Mt7621, // MediaTek Legacy (Wi-Fi 5 / MIPS)
    Mt7628, // MediaTek MT7628 (Wi-Fi 4/5 budget)
    Mt7620, // MediaTek MT7620 (Wi-Fi 4/5 budget)
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
            } else if content.contains("mt7987") {
                return Self::Mt7987;
            } else if content.contains("mt7986") {
                return Self::Mt7986;
            } else if content.contains("mt7981") {
                return Self::Mt7981;
            } else if content.contains("mt7622") {
                return Self::Mt7622;
            } else if content.contains("mt7629") {
                return Self::Mt7629;
            } else if content.contains("mt7621") {
                return Self::Mt7621;
            } else if content.contains("mt7628") {
                return Self::Mt7628;
            } else if content.contains("mt7620") {
                return Self::Mt7620;
            }
        }

        Self::Unknown
    }

    /// Returns a human-readable display name for the UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Mt7988 => "MT7988 (Filogic 880)",
            Self::Mt7987 => "MT7987 (Filogic 850)",
            Self::Mt7986 => "MT7986 (Filogic 830)",
            Self::Mt7981 => "MT7981 (Filogic 820)",
            Self::Mt7622 => "MT7622 (Filogic 800)",
            Self::Mt7629 => "MT7629",
            Self::Mt7621 => "MT7621 (MIPS)",
            Self::Mt7628 => "MT7628 (MIPS)",
            Self::Mt7620 => "MT7620 (MIPS)",
            Self::Unknown => "Unknown SoC",
        }
    }

    /// Returns the CPU architecture and core count of the SoC.
    pub fn architecture(&self) -> &'static str {
        match self {
            Self::Mt7988 => "4× Cortex-A73",
            Self::Mt7987 => "4× Cortex-A53",
            Self::Mt7986 => "4× Cortex-A53",
            Self::Mt7981 => "2× Cortex-A53",
            Self::Mt7622 => "2× Cortex-A53",
            Self::Mt7629 => "2× Cortex-A7",
            Self::Mt7621 => "2× MIPS1004Kc",
            Self::Mt7628 => "1× MIPS24KEc",
            Self::Mt7620 => "1× MIPS24KEc",
            Self::Unknown => "Unknown",
        }
    }

    /// Parses the CPU microarchitecture from `/proc/cpuinfo` output.
    pub(super) fn parse_cpu_arch_from_cpuinfo(cpuinfo: &str) -> Option<&'static str> {
        // Helper: strip optional 0x/0X prefix and parse as hex u16.
        let parse_hex_u16 = |value: &str| -> Option<u16> {
            let hex = value
                .strip_prefix("0x")
                .or_else(|| value.strip_prefix("0X"))
                .unwrap_or(value);
            u16::from_str_radix(hex, 16).ok()
        };

        let mut implementer: Option<u16> = None;
        let mut part: Option<u16> = None;
        let mut mips_model: Option<&'static str> = None;

        for line in cpuinfo.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();

            match key {
                // ARM identification fields
                k if k.eq_ignore_ascii_case("CPU implementer") => {
                    implementer = parse_hex_u16(value);
                }
                k if k.eq_ignore_ascii_case("CPU part") => {
                    part = parse_hex_u16(value);
                }

                // MIPS identification fields
                k if k.eq_ignore_ascii_case("cpu model")
                    || k.eq_ignore_ascii_case("system type") =>
                {
                    let lower = value.to_ascii_lowercase();
                    mips_model = if lower.contains("1004kc") {
                        Some("MIPS 1004Kc")
                    } else if lower.contains("24kec") {
                        Some("MIPS 24KEc")
                    } else if lower.contains("mips") {
                        Some("MIPS")
                    } else {
                        mips_model // keep previous value if any
                    };
                }

                _ => {}
            }
        }

        // Prefer ARM when the implementer is ARM Ltd (0x41).
        if implementer == Some(0x41) {
            if let Some(part_num) = part {
                return match part_num {
                    0xc07 => Some("Cortex-A7"),
                    0xd03 => Some("Cortex-A53"),
                    0xd04 => Some("Cortex-A35"),
                    0xd05 => Some("Cortex-A55"),
                    0xd08 => Some("Cortex-A72"),
                    0xd09 => Some("Cortex-A73"),
                    0xd0a => Some("Cortex-A75"),
                    0xd0b => Some("Cortex-A76"),
                    0xd0c => Some("Cortex-A77"),
                    0xd41 => Some("Cortex-A78"),
                    _ => None,
                };
            }
        }

        mips_model
    }

    /// Returns the maximum CPU core frequency in Hertz (Hz).
    /// Returns 0 if the SoC model is unknown.
    pub fn core_frequency_hz(&self) -> u64 {
        match self {
            Self::Mt7988 => 1_800_000_000, // 1.8 GHz
            Self::Mt7987 => 2_000_000_000, // 2.0 GHz
            Self::Mt7986 => 2_000_000_000, // 2.0 GHz (A variant; B is 1.6 GHz)
            Self::Mt7622 => 1_350_000_000, // 1.35 GHz
            Self::Mt7981 => 1_300_000_000, // 1.3 GHz (B variant); A can reach 1.8 GHz
            Self::Mt7629 => 1_250_000_000, // 1.25 GHz
            Self::Mt7621 => 880_000_000,   // 880 MHz
            Self::Mt7628 => 580_000_000,   // 580 MHz
            Self::Mt7620 => 600_000_000,   // 600 MHz (commonly listed as 580/600)
            Self::Unknown => 0,
        }
    }

    /// Returns the number of Packet Processing Engines (PPE) available on the SoC.
    pub fn ppe_count(&self) -> usize {
        match self {
            Self::Mt7988 => 3,
            Self::Mt7987 | Self::Mt7986 => 2,
            Self::Mt7981 => 2, // https://github.com/torvalds/linux/blob/3f008280327ba5ad132965abab0c7846283cef0c/drivers/net/ethernet/mediatek/mtk_eth_soc.h#L1117
            // Older chips have a single PPE / HNAT engine
            Self::Mt7622 | Self::Mt7629 | Self::Mt7621 | Self::Mt7628 | Self::Mt7620 => 1,
            Self::Unknown => 0,
        }
    }

    /// Returns the number of Wireless Ethernet Dispatcher (WED) modules available on the SoC.
    pub fn wed_count(&self) -> usize {
        match self {
            Self::Mt7988 | Self::Mt7987 | Self::Mt7986 | Self::Mt7622 => 2,
            Self::Mt7981 => 1, // https://github.com/torvalds/linux/blob/3f008280327ba5ad132965abab0c7846283cef0c/Documentation/devicetree/bindings/net/mediatek%2Cnet.yaml#L348
            // Legacy and budget chips do not have WED
            Self::Mt7629 | Self::Mt7621 | Self::Mt7628 | Self::Mt7620 => 0,
            Self::Unknown => 0,
        }
    }

    /// Indicates whether the network subsystem supports Receive Side Scaling (RSS).
    pub fn has_rss(&self) -> bool {
        match self {
            // Modern Filogic platforms support advanced multi-queue RSS
            Self::Mt7988 | Self::Mt7987 | Self::Mt7986 | Self::Mt7981 | Self::Mt7622 => true,
            // Older chips lack proper multi-queue RSS
            Self::Mt7629 | Self::Mt7621 | Self::Mt7628 | Self::Mt7620 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC hardware supports IP/TCP/UDP Checksum Offload.
    pub fn has_checksum_offload(&self) -> bool {
        match self {
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629
            | Self::Mt7621
            | Self::Mt7628
            | Self::Mt7620 => true, // Present on all MediaTek SoCs in this list
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC supports TCP Segmentation Offload (TSO) in hardware.
    pub fn has_tso(&self) -> bool {
        match self {
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629
            | Self::Mt7621
            | Self::Mt7628
            | Self::Mt7620 => true, // Present on all MediaTek SoCs in this list
            Self::Unknown => false,
        }
    }

    /// Indicates whether the network interface controller (NIC) supports Multiple RX Queues.
    pub fn has_multi_rx_queues(&self) -> bool {
        match self {
            Self::Mt7988 | Self::Mt7987 | Self::Mt7986 | Self::Mt7981 | Self::Mt7622 => true,
            Self::Mt7629 | Self::Mt7621 | Self::Mt7628 | Self::Mt7620 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC is equipped with a dedicated Hardware Crypto Engine.
    pub fn has_crypto_engine(&self) -> bool {
        match self {
            // Filogic/ARM platforms use ARM cryptographic extensions or dedicated EIP modules,
            // MT7621 uses an internal EIP93 crypto accelerator, and MT7620/MT7628 have basic HW crypto.
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629
            | Self::Mt7621 // EIP-93
            | Self::Mt7628 // basic AES engine
            | Self::Mt7620 // basic AES engine
            => true,
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC hardware provides acceleration for the AES cipher.
    pub fn has_aes_acceleration(&self) -> bool {
        match self {
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629
            | Self::Mt7621
            | Self::Mt7628
            | Self::Mt7620 => true, // All chips in the list have at least basic AES acceleration
            Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC hardware provides acceleration for the SHA hashing algorithms.
    pub fn has_sha_acceleration(&self) -> bool {
        match self {
            // Full SHA acceleration on Filogic and most ARM / MT7621
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629
            | Self::Mt7621 => true,
            // MT7628 / MT7620 mainly accelerate AES, SHA is usually software
            Self::Mt7628 | Self::Mt7620 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC contains an integrated True Random Number Generator (TRNG / HW RNG).
    pub fn has_trng(&self) -> bool {
        match self {
            // Present on modern Filogic and most ARM / MT7621
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629
            | Self::Mt7621 => true,
            // Older budget MIPS chips usually lack a proper HW TRNG
            Self::Mt7628 | Self::Mt7620 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC supports Secure Boot mechanisms.
    pub fn has_secure_boot(&self) -> bool {
        match self {
            // ARM platforms (Filogic + MT7622 + MT7629) support Secure Boot / ATF
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629 => true,
            // Classic MIPS chips do not have Secure Boot support
            Self::Mt7621 | Self::Mt7628 | Self::Mt7620 | Self::Unknown => false,
        }
    }

    /// Indicates whether the SoC supports ARM TrustZone or a Trusted Execution Environment (TEE).
    pub fn has_trustzone(&self) -> bool {
        match self {
            // All Cortex-A based SoCs support TrustZone (OP-TEE can be used)
            Self::Mt7988
            | Self::Mt7987
            | Self::Mt7986
            | Self::Mt7981
            | Self::Mt7622
            | Self::Mt7629 => true,
            // MIPS cores do not implement ARM TrustZone
            Self::Mt7621 | Self::Mt7628 | Self::Mt7620 | Self::Unknown => false,
        }
    }
}

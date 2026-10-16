use serde::Serialize;

pub const MINIMUM_DRIVER: (u32, u32) = (560, 76);
pub const MINIMUM_VRAM_MB: u64 = 2_048;
pub const RECOMMENDED_VRAM_MB: u64 = 8_192;
pub const MINIMUM_FREE_VRAM_MB: u64 = 1_536;
pub const MINIMUM_COMPUTE_CAPABILITY: (u32, u32) = (7, 5);
pub const BF16_COMPUTE_CAPABILITY: (u32, u32) = (8, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    Recommended,
    Minimum,
    PartiallyTested,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuCompatibility {
    pub tier: SupportTier,
    pub can_generate_with_cuda: bool,
    pub bf16_supported: bool,
    pub architecture: String,
    pub summary: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuFacts<'a> {
    pub available: bool,
    pub memory_total_mb: Option<u64>,
    pub memory_free_mb: Option<u64>,
    pub driver_version: Option<&'a str>,
    pub compute_capability: Option<&'a str>,
}

pub fn parse_version(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.trim().split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn at_least(value: Option<(u32, u32)>, minimum: (u32, u32)) -> bool {
    value.is_some_and(|value| value >= minimum)
}

fn architecture(compute: Option<(u32, u32)>) -> &'static str {
    match compute {
        Some((major, _)) if major >= 12 => "Blackwell or newer",
        Some((8, 9)) => "Ada Lovelace",
        Some((8, _)) => "Ampere",
        Some((7, 5)) => "Turing",
        Some((major, _)) if major >= 9 => "post-Ada NVIDIA architecture",
        Some(_) => "legacy NVIDIA architecture",
        None => "unknown NVIDIA architecture",
    }
}

pub fn assess(facts: GpuFacts<'_>) -> GpuCompatibility {
    if !facts.available {
        return GpuCompatibility {
            tier: SupportTier::Unsupported,
            can_generate_with_cuda: false,
            bf16_supported: false,
            architecture: "not detected".into(),
            summary: "No NVIDIA GPU was detected. CUDA generation is unavailable.".into(),
            recommendations: vec![
                "Install a supported NVIDIA GPU, or select CPU mode for slow compatibility testing."
                    .into(),
            ],
        };
    }

    let driver = facts.driver_version.and_then(parse_version);
    let compute = facts.compute_capability.and_then(parse_version);
    let architecture = architecture(compute).to_string();
    let bf16_supported = at_least(compute, BF16_COMPUTE_CAPABILITY);
    let mut recommendations = Vec::new();

    if !at_least(driver, MINIMUM_DRIVER) {
        recommendations.push(format!(
            "Update the NVIDIA driver to {}.{} or newer before using CUDA.",
            MINIMUM_DRIVER.0, MINIMUM_DRIVER.1
        ));
        return GpuCompatibility {
            tier: SupportTier::Unsupported,
            can_generate_with_cuda: false,
            bf16_supported,
            architecture,
            summary: match facts.driver_version {
                Some(version) => format!("NVIDIA driver {version} is below the supported range."),
                None => "The NVIDIA driver version could not be determined.".into(),
            },
            recommendations,
        };
    }

    if compute.is_some() && !at_least(compute, MINIMUM_COMPUTE_CAPABILITY) {
        recommendations.push("Use a Turing-generation NVIDIA GPU or newer.".into());
        return GpuCompatibility {
            tier: SupportTier::Unsupported,
            can_generate_with_cuda: false,
            bf16_supported: false,
            architecture,
            summary: "The GPU compute capability is below the tested CUDA baseline.".into(),
            recommendations,
        };
    }

    if facts
        .memory_total_mb
        .is_some_and(|memory| memory < MINIMUM_VRAM_MB)
    {
        recommendations.push(format!(
            "Use a GPU with at least {} MiB of dedicated VRAM.",
            MINIMUM_VRAM_MB
        ));
        return GpuCompatibility {
            tier: SupportTier::Unsupported,
            can_generate_with_cuda: false,
            bf16_supported,
            architecture,
            summary: "Dedicated VRAM is below Beatforge's minimum tested capacity.".into(),
            recommendations,
        };
    }

    let hardware_tier = match (compute, facts.memory_total_mb) {
        (Some(_), Some(memory)) if memory >= RECOMMENDED_VRAM_MB && bf16_supported => {
            SupportTier::Recommended
        }
        (Some(_), Some(_)) => SupportTier::Minimum,
        _ => SupportTier::PartiallyTested,
    };

    if facts
        .memory_free_mb
        .is_some_and(|memory| memory < MINIMUM_FREE_VRAM_MB)
    {
        recommendations.push(format!(
            "Close GPU-heavy applications until at least {} MiB of VRAM is free.",
            MINIMUM_FREE_VRAM_MB
        ));
        return GpuCompatibility {
            tier: hardware_tier,
            can_generate_with_cuda: false,
            bf16_supported,
            architecture,
            summary: "The GPU is supported, but there is not enough free VRAM to start safely."
                .into(),
            recommendations,
        };
    }

    if compute.is_none() || facts.memory_total_mb.is_none() {
        recommendations.push(
            "Hardware details are incomplete. Update the NVIDIA driver and rerun diagnostics."
                .into(),
        );
        return GpuCompatibility {
            tier: SupportTier::PartiallyTested,
            can_generate_with_cuda: true,
            bf16_supported,
            architecture,
            summary:
                "CUDA can be attempted, but this hardware combination is only partially identified."
                    .into(),
            recommendations,
        };
    }

    if !bf16_supported {
        recommendations.push(
            "Native BF16 is unavailable; Beatforge will use the slower FP32 fallback.".into(),
        );
    }

    if hardware_tier == SupportTier::Minimum {
        recommendations.push(
            "Generate fewer difficulties per job and avoid other GPU-heavy applications.".into(),
        );
        GpuCompatibility {
            tier: SupportTier::Minimum,
            can_generate_with_cuda: true,
            bf16_supported,
            architecture,
            summary: "This GPU meets the minimum tested CUDA configuration.".into(),
            recommendations,
        }
    } else {
        debug_assert_eq!(hardware_tier, SupportTier::Recommended);
        GpuCompatibility {
            tier: SupportTier::Recommended,
            can_generate_with_cuda: true,
            bf16_supported,
            architecture,
            summary: "This GPU meets the recommended Beatforge configuration.".into(),
            recommendations,
        }
    }
}

pub fn cuda_preflight(facts: GpuFacts<'_>) -> Result<GpuCompatibility, String> {
    let result = assess(facts);
    if result.can_generate_with_cuda {
        Ok(result)
    } else {
        let advice = result
            .recommendations
            .first()
            .map(String::as_str)
            .unwrap_or("Open Models and run AI environment diagnostics for recovery guidance.");
        Err(format!(
            "CUDA preflight failed: {} {advice}",
            result.summary
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        memory: u64,
        free: u64,
        driver: &'static str,
        compute: &'static str,
    ) -> GpuFacts<'static> {
        GpuFacts {
            available: true,
            memory_total_mb: Some(memory),
            memory_free_mb: Some(free),
            driver_version: Some(driver),
            compute_capability: Some(compute),
        }
    }

    #[test]
    fn classifies_recorded_recommended_and_minimum_devices() {
        let rtx_4070 = assess(facts(12_282, 9_867, "610.88", "8.9"));
        assert_eq!(rtx_4070.tier, SupportTier::Recommended);
        assert!(rtx_4070.bf16_supported);
        assert!(rtx_4070.can_generate_with_cuda);

        let mx_450 = assess(facts(2_048, 1_800, "573.91", "7.5"));
        assert_eq!(mx_450.tier, SupportTier::Minimum);
        assert!(!mx_450.bf16_supported);
        assert!(mx_450.can_generate_with_cuda);
    }

    #[test]
    fn blocks_old_driver_too_little_vram_and_busy_gpu() {
        assert!(cuda_preflight(facts(8_192, 7_000, "552.22", "8.6")).is_err());
        assert!(cuda_preflight(facts(1_024, 900, "610.88", "8.9")).is_err());
        let busy = assess(facts(12_282, 1_000, "610.88", "8.9"));
        assert_eq!(busy.tier, SupportTier::Recommended);
        assert!(!busy.can_generate_with_cuda);
        assert!(cuda_preflight(facts(12_282, 1_000, "610.88", "8.9")).is_err());
    }

    #[test]
    fn incomplete_but_current_probe_is_not_claimed_as_supported() {
        let result = assess(GpuFacts {
            available: true,
            memory_total_mb: None,
            memory_free_mb: None,
            driver_version: Some("610.88"),
            compute_capability: None,
        });
        assert_eq!(result.tier, SupportTier::PartiallyTested);
        assert!(result.can_generate_with_cuda);
    }
}

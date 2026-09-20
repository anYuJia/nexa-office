/// Hard resource ceilings for untrusted package input.
///
/// Values are product defaults, not promises that every valid file below a limit will
/// be accepted by every future editor. Callers may choose stricter limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageLimits {
    pub max_entries: u32,
    pub max_single_part_uncompressed: u64,
    pub max_total_uncompressed: u64,
    pub max_compression_ratio: u64,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            max_entries: 50_000,
            max_single_part_uncompressed: 256 * 1024 * 1024,
            max_total_uncompressed: 1024 * 1024 * 1024,
            max_compression_ratio: 200,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackageUsage {
    pub entries: u32,
    pub total_compressed: u64,
    pub total_uncompressed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitViolation {
    TooManyEntries,
    SinglePartTooLarge,
    TotalUncompressedTooLarge,
    SuspiciousCompressionRatio,
    IntegerOverflow,
}

impl PackageLimits {
    pub fn check_entry(
        self,
        usage: &mut PackageUsage,
        compressed_size: u64,
        uncompressed_size: u64,
    ) -> Result<(), LimitViolation> {
        if uncompressed_size > self.max_single_part_uncompressed {
            return Err(LimitViolation::SinglePartTooLarge);
        }

        if compressed_size > 0
            && uncompressed_size / compressed_size > self.max_compression_ratio
        {
            return Err(LimitViolation::SuspiciousCompressionRatio);
        }

        usage.entries = usage
            .entries
            .checked_add(1)
            .ok_or(LimitViolation::IntegerOverflow)?;
        usage.total_compressed = usage
            .total_compressed
            .checked_add(compressed_size)
            .ok_or(LimitViolation::IntegerOverflow)?;
        usage.total_uncompressed = usage
            .total_uncompressed
            .checked_add(uncompressed_size)
            .ok_or(LimitViolation::IntegerOverflow)?;

        if usage.entries > self.max_entries {
            return Err(LimitViolation::TooManyEntries);
        }
        if usage.total_uncompressed > self.max_total_uncompressed {
            return Err(LimitViolation::TotalUncompressedTooLarge);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_extreme_compression_ratio_before_accounting() {
        let limits = PackageLimits {
            max_compression_ratio: 10,
            ..PackageLimits::default()
        };
        let mut usage = PackageUsage::default();

        assert_eq!(
            limits.check_entry(&mut usage, 1, 11),
            Err(LimitViolation::SuspiciousCompressionRatio)
        );
        assert_eq!(usage, PackageUsage::default());
    }

    #[test]
    fn accumulates_package_usage() {
        let limits = PackageLimits::default();
        let mut usage = PackageUsage::default();

        limits.check_entry(&mut usage, 100, 200).unwrap();
        limits.check_entry(&mut usage, 200, 400).unwrap();

        assert_eq!(usage.entries, 2);
        assert_eq!(usage.total_compressed, 300);
        assert_eq!(usage.total_uncompressed, 600);
    }
}

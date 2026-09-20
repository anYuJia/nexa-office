#![no_main]

use libfuzzer_sys::fuzz_target;
use nexa_ooxml::{LazyZipPackage, PackageLimits};
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    let limits = PackageLimits {
        max_entries: 4096,
        max_single_part_uncompressed: 32 * 1024 * 1024,
        max_total_uncompressed: 64 * 1024 * 1024,
        max_compression_ratio: 100,
    };

    if let Ok(mut package) = LazyZipPackage::new(Cursor::new(data), limits) {
        let parts: Vec<_> = package
            .entries()
            .take(8)
            .map(|entry| entry.part_name().clone())
            .collect();

        for part in parts {
            let _ = package.read_part(&part);
        }

        let _ = package.read_content_types();
        let _ = package.read_relationships(None);
        let _ = package.office_package_info();
    }
});

#![no_main]

use libfuzzer_sys::fuzz_target;
use nexa_ooxml::{
    XmlLimits, parse_content_types_with_limits, parse_relationships_with_limits,
};

fuzz_target!(|data: &[u8]| {
    let limits = XmlLimits {
        max_input_bytes: 1024 * 1024,
        max_depth: 64,
        max_attributes_per_element: 64,
    };

    let _ = parse_content_types_with_limits(data, limits);
    let _ = parse_relationships_with_limits(None, data, limits);
});

#![forbid(unsafe_code)]

//! Shared OOXML / OPC primitives.
//!
//! Phase 2 starts with the package invariants that must remain true regardless of the
//! eventual ZIP and XML parser crates. I/O adapters are intentionally kept outside
//! these types so they can be fuzzed and tested without filesystem or UI concerns.

mod content_types;
mod limits;
mod package;
mod part_name;
mod relationships;

pub use content_types::{ContentTypeMap, ContentTypeRule};
pub use limits::{PackageLimits, PackageUsage};
pub use package::{Package, PackageError, Part, RelationshipSet};
pub use part_name::{PartName, PartNameError};
pub use relationships::{
    Relationship, RelationshipId, RelationshipTarget, TargetMode, relationship_part_name,
    resolve_internal_target, source_part_from_relationship_part,
};

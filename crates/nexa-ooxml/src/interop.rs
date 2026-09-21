use crate::{Package, PartName, Relationship, RelationshipTarget};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompatibilityFeature {
    Hyperlink,
    Comments,
    Notes,
    Chart,
    Drawing,
    Pivot,
    ExternalData,
    EmbeddedObject,
    ActiveX,
    Macro,
    SmartArt,
    CustomXml,
    Media,
    UnknownRelationship,
}

impl fmt::Display for CompatibilityFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hyperlink => "hyperlink",
            Self::Comments => "comments",
            Self::Notes => "notes",
            Self::Chart => "chart",
            Self::Drawing => "drawing",
            Self::Pivot => "pivot",
            Self::ExternalData => "external data",
            Self::EmbeddedObject => "embedded object",
            Self::ActiveX => "ActiveX",
            Self::Macro => "macro",
            Self::SmartArt => "SmartArt",
            Self::CustomXml => "custom XML",
            Self::Media => "media",
            Self::UnknownRelationship => "unknown relationship",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityRisk {
    PreservedOpaque,
    RewriteBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityFinding {
    pub source: Option<PartName>,
    pub feature: CompatibilityFeature,
    pub risk: CompatibilityRisk,
    pub relationship_type: String,
    pub target: String,
}

impl CompatibilityFinding {
    #[must_use]
    pub fn blocks_rewrite(&self) -> bool {
        self.risk == CompatibilityRisk::RewriteBlocked
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InteropReport {
    findings: Vec<CompatibilityFinding>,
}

impl InteropReport {
    #[must_use]
    pub fn findings(&self) -> &[CompatibilityFinding] {
        &self.findings
    }

    #[must_use]
    pub fn blocker_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.blocks_rewrite())
            .count()
    }

    #[must_use]
    pub fn can_rewrite_safely(&self) -> bool {
        self.blocker_count() == 0
    }

    #[must_use]
    pub fn summary(&self) -> String {
        let blockers = self.blocker_count();
        if blockers == 0 {
            format!("{} interoperability finding(s), no rewrite blockers", self.findings.len())
        } else {
            format!(
                "{} interoperability finding(s), {blockers} rewrite blocker(s)",
                self.findings.len()
            )
        }
    }

    fn push(&mut self, finding: CompatibilityFinding) {
        if !self.findings.contains(&finding) {
            self.findings.push(finding);
        }
    }
}

#[must_use]
pub fn audit_package_rewrite_risks(package: &Package) -> InteropReport {
    let mut report = InteropReport::default();

    for relationship in package.package_relationships().iter() {
        audit_relationship(None, relationship, &mut report);
    }
    for (source, relationships) in package.part_relationships() {
        for relationship in relationships.iter() {
            audit_relationship(Some(source), relationship, &mut report);
        }
    }

    for part in package.parts() {
        let content_type = part.content_type().to_ascii_lowercase();
        let path = part.name().as_str().to_ascii_lowercase();

        let feature = if content_type.contains("vba") || path.ends_with("/vbaproject.bin") {
            Some(CompatibilityFeature::Macro)
        } else if content_type.contains("activex") || path.contains("/activex/") {
            Some(CompatibilityFeature::ActiveX)
        } else if content_type.contains("oleobject")
            || path.contains("/embeddings/")
            || path.contains("/oleobject")
        {
            Some(CompatibilityFeature::EmbeddedObject)
        } else if content_type.contains("chart") || path.contains("/charts/") {
            Some(CompatibilityFeature::Chart)
        } else if content_type.contains("comments") || path.contains("/comments") {
            Some(CompatibilityFeature::Comments)
        } else if path.contains("/customxml/") {
            Some(CompatibilityFeature::CustomXml)
        } else {
            None
        };

        if let Some(feature) = feature {
            report.push(CompatibilityFinding {
                source: Some(part.name().clone()),
                feature,
                risk: if matches!(
                    feature,
                    CompatibilityFeature::Macro
                        | CompatibilityFeature::ActiveX
                        | CompatibilityFeature::EmbeddedObject
                ) {
                    CompatibilityRisk::RewriteBlocked
                } else {
                    CompatibilityRisk::PreservedOpaque
                },
                relationship_type: "content-type".into(),
                target: part.name().to_string(),
            });
        }
    }

    report
}

fn audit_relationship(
    source: Option<&PartName>,
    relationship: &Relationship,
    report: &mut InteropReport,
) {
    let relationship_type = relationship.relationship_type.to_ascii_lowercase();
    let source_path = source
        .map(PartName::as_str)
        .unwrap_or("/")
        .to_ascii_lowercase();

    let feature = classify_relationship(&relationship_type, &relationship.target);
    let Some(feature) = feature else {
        return;
    };

    let risk = if relationship_rewrite_is_lossy(&source_path, feature) {
        CompatibilityRisk::RewriteBlocked
    } else {
        CompatibilityRisk::PreservedOpaque
    };

    report.push(CompatibilityFinding {
        source: source.cloned(),
        feature,
        risk,
        relationship_type: relationship.relationship_type.clone(),
        target: target_text(&relationship.target),
    });
}

fn classify_relationship(
    relationship_type: &str,
    target: &RelationshipTarget,
) -> Option<CompatibilityFeature> {
    let suffix = relationship_type.rsplit('/').next().unwrap_or(relationship_type);

    match suffix {
        "hyperlink" => Some(CompatibilityFeature::Hyperlink),
        "comments" | "threadedcomment" | "person" => Some(CompatibilityFeature::Comments),
        "notesslide" | "notesmaster" => Some(CompatibilityFeature::Notes),
        "chart" | "chartsheet" => Some(CompatibilityFeature::Chart),
        "drawing" | "vmlDrawing" => Some(CompatibilityFeature::Drawing),
        "pivottable" | "pivotcachedefinition" | "pivotcacherecords" => {
            Some(CompatibilityFeature::Pivot)
        }
        "externallink" | "externalconnection" | "connections" => {
            Some(CompatibilityFeature::ExternalData)
        }
        "oleobject" | "package" => Some(CompatibilityFeature::EmbeddedObject),
        "control" | "activexcontrol" => Some(CompatibilityFeature::ActiveX),
        "vbaproject" | "vbadata" => Some(CompatibilityFeature::Macro),
        "diagramdata" | "diagramlayout" | "diagramquickstyle" | "diagramcolors" => {
            Some(CompatibilityFeature::SmartArt)
        }
        "customxml" | "customxmlprops" => Some(CompatibilityFeature::CustomXml),
        "audio" | "video" | "media" => Some(CompatibilityFeature::Media),
        _ if matches!(target, RelationshipTarget::External(_)) => {
            Some(CompatibilityFeature::UnknownRelationship)
        }
        _ => None,
    }
}

fn relationship_rewrite_is_lossy(source_path: &str, feature: CompatibilityFeature) -> bool {
    let rewritten_semantic_part = source_path == "/word/document.xml"
        || source_path == "/xl/workbook.xml"
        || source_path.starts_with("/xl/worksheets/")
        || source_path == "/ppt/presentation.xml"
        || source_path.starts_with("/ppt/slides/");

    if !rewritten_semantic_part {
        return matches!(
            feature,
            CompatibilityFeature::Macro
                | CompatibilityFeature::ActiveX
                | CompatibilityFeature::EmbeddedObject
        );
    }

    !matches!(
        feature,
        CompatibilityFeature::CustomXml
            | CompatibilityFeature::Notes
            | CompatibilityFeature::Media
    )
}

fn target_text(target: &RelationshipTarget) -> String {
    match target {
        RelationshipTarget::Internal(part) => part.to_string(),
        RelationshipTarget::External(value) => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ContentTypeMap, ContentTypeRule, Package, Relationship, RelationshipId, RelationshipTarget,
    };

    fn package() -> Package {
        let mut content_types = ContentTypeMap::default();
        content_types.insert(ContentTypeRule::Default {
            extension: "xml".into(),
            content_type: "application/xml".into(),
        });
        Package::new(content_types)
    }

    #[test]
    fn worksheet_hyperlink_blocks_semantic_rewrite() {
        let mut package = package();
        let sheet = PartName::new("/xl/worksheets/sheet1.xml").unwrap();
        package.insert_part(sheet.clone(), Vec::new()).unwrap();
        package
            .relationships_mut(sheet.clone())
            .insert(Relationship {
                id: RelationshipId::new("rId1"),
                relationship_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
                        .into(),
                target: RelationshipTarget::External("https://example.com".into()),
            })
            .unwrap();

        let report = audit_package_rewrite_risks(&package);

        assert_eq!(report.blocker_count(), 1);
        assert_eq!(
            report.findings()[0].feature,
            CompatibilityFeature::Hyperlink
        );
    }

    #[test]
    fn opaque_custom_xml_is_reported_without_blocking() {
        let mut package = package();
        let source = PartName::new("/customXml/item1.xml").unwrap();
        package.insert_part(source, b"<x/>".to_vec()).unwrap();

        let report = audit_package_rewrite_risks(&package);

        assert!(report.can_rewrite_safely());
        assert!(report.findings().iter().any(|finding| {
            finding.feature == CompatibilityFeature::CustomXml
                && finding.risk == CompatibilityRisk::PreservedOpaque
        }));
    }

    #[test]
    fn macros_are_always_blockers() {
        let mut content_types = ContentTypeMap::default();
        content_types.insert(ContentTypeRule::Override {
            part_name: PartName::new("/xl/vbaProject.bin").unwrap(),
            content_type: "application/vnd.ms-office.vbaProject".into(),
        });
        let mut package = Package::new(content_types);
        package
            .insert_part(
                PartName::new("/xl/vbaProject.bin").unwrap(),
                vec![0xde, 0xad],
            )
            .unwrap();

        let report = audit_package_rewrite_risks(&package);

        assert_eq!(report.blocker_count(), 1);
        assert_eq!(report.findings()[0].feature, CompatibilityFeature::Macro);
    }
}

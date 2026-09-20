use crate::{
    ContentTypeMap, ContentTypeRule, PackageError, PartName, PartNameError, Relationship,
    RelationshipId, RelationshipSet, RelationshipTarget, RelationshipTargetError,
    resolve_internal_target,
};
use quick_xml::{
    XmlVersion,
    events::{BytesStart, Event},
    reader::Reader,
};
use std::{error::Error, fmt};

const CONTENT_TYPES_ROOT: &str = "Types";
const RELATIONSHIPS_ROOT: &str = "Relationships";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmlLimits {
    pub max_input_bytes: usize,
    pub max_depth: usize,
    pub max_attributes_per_element: usize,
}

impl Default for XmlLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 4 * 1024 * 1024,
            max_depth: 32,
            max_attributes_per_element: 32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlParseError {
    InputTooLarge,
    DepthExceeded,
    TooManyAttributes,
    MissingRoot(&'static str),
    UnexpectedRoot {
        expected: &'static str,
        actual: String,
    },
    MissingAttribute(&'static str),
    UnsupportedDocType,
    InvalidPartName(PartNameError),
    InvalidRelationshipTarget(RelationshipTargetError),
    DuplicateRelationshipId(String),
    Malformed(String),
}

impl fmt::Display for XmlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputTooLarge => f.write_str("XML input exceeds configured size limit"),
            Self::DepthExceeded => f.write_str("XML nesting exceeds configured depth limit"),
            Self::TooManyAttributes => {
                f.write_str("XML element exceeds configured attribute-count limit")
            }
            Self::MissingRoot(expected) => write!(f, "missing XML root element: {expected}"),
            Self::UnexpectedRoot { expected, actual } => {
                write!(f, "unexpected XML root element '{actual}', expected '{expected}'")
            }
            Self::MissingAttribute(name) => write!(f, "missing required XML attribute: {name}"),
            Self::UnsupportedDocType => f.write_str("DOCTYPE is not allowed in OOXML metadata"),
            Self::InvalidPartName(error) => write!(f, "invalid OPC part name: {error}"),
            Self::InvalidRelationshipTarget(error) => {
                write!(f, "invalid OPC relationship target: {error}")
            }
            Self::DuplicateRelationshipId(id) => {
                write!(f, "duplicate OPC relationship identifier: {id}")
            }
            Self::Malformed(message) => write!(f, "malformed XML: {message}"),
        }
    }
}

impl Error for XmlParseError {}

impl From<PartNameError> for XmlParseError {
    fn from(value: PartNameError) -> Self {
        Self::InvalidPartName(value)
    }
}

impl From<RelationshipTargetError> for XmlParseError {
    fn from(value: RelationshipTargetError) -> Self {
        Self::InvalidRelationshipTarget(value)
    }
}

pub fn parse_content_types(input: &[u8]) -> Result<ContentTypeMap, XmlParseError> {
    parse_content_types_with_limits(input, XmlLimits::default())
}

pub fn parse_content_types_with_limits(
    input: &[u8],
    limits: XmlLimits,
) -> Result<ContentTypeMap, XmlParseError> {
    ensure_input_limit(input, limits)?;

    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);

    let mut output = ContentTypeMap::default();
    let mut buffer = Vec::new();
    let mut depth = 0usize;
    let mut root_seen = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                depth = checked_depth(depth, limits)?;
                handle_content_type_element(
                    &element,
                    &mut root_seen,
                    &mut output,
                    limits,
                )?;
            }
            Ok(Event::Empty(element)) => {
                handle_content_type_element(
                    &element,
                    &mut root_seen,
                    &mut output,
                    limits,
                )?;
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
            }
            Ok(Event::DocType(_)) => return Err(XmlParseError::UnsupportedDocType),
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(XmlParseError::Malformed(error.to_string())),
        }
        buffer.clear();
    }

    if !root_seen {
        return Err(XmlParseError::MissingRoot(CONTENT_TYPES_ROOT));
    }

    Ok(output)
}

pub fn parse_relationships(
    source: Option<&PartName>,
    input: &[u8],
) -> Result<RelationshipSet, XmlParseError> {
    parse_relationships_with_limits(source, input, XmlLimits::default())
}

pub fn parse_relationships_with_limits(
    source: Option<&PartName>,
    input: &[u8],
    limits: XmlLimits,
) -> Result<RelationshipSet, XmlParseError> {
    ensure_input_limit(input, limits)?;

    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);

    let mut output = RelationshipSet::default();
    let mut buffer = Vec::new();
    let mut depth = 0usize;
    let mut root_seen = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                depth = checked_depth(depth, limits)?;
                handle_relationship_element(
                    source,
                    &element,
                    &mut root_seen,
                    &mut output,
                    limits,
                )?;
            }
            Ok(Event::Empty(element)) => {
                handle_relationship_element(
                    source,
                    &element,
                    &mut root_seen,
                    &mut output,
                    limits,
                )?;
            }
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
            }
            Ok(Event::DocType(_)) => return Err(XmlParseError::UnsupportedDocType),
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(XmlParseError::Malformed(error.to_string())),
        }
        buffer.clear();
    }

    if !root_seen {
        return Err(XmlParseError::MissingRoot(RELATIONSHIPS_ROOT));
    }

    Ok(output)
}

fn handle_content_type_element(
    element: &BytesStart<'_>,
    root_seen: &mut bool,
    output: &mut ContentTypeMap,
    limits: XmlLimits,
) -> Result<(), XmlParseError> {
    let name = element.local_name();

    if !*root_seen {
        if name.as_ref() != CONTENT_TYPES_ROOT {
            return Err(XmlParseError::UnexpectedRoot {
                expected: CONTENT_TYPES_ROOT,
                actual: name.as_ref().to_owned(),
            });
        }
        *root_seen = true;
        validate_attribute_count(element, limits)?;
        return Ok(());
    }

    match name.as_ref() {
        "Default" => {
            let attributes = collect_attributes(element, limits)?;
            output.insert(ContentTypeRule::Default {
                extension: required_attribute(&attributes, "Extension")?.to_owned(),
                content_type: required_attribute(&attributes, "ContentType")?.to_owned(),
            });
        }
        "Override" => {
            let attributes = collect_attributes(element, limits)?;
            output.insert(ContentTypeRule::Override {
                part_name: PartName::new(
                    required_attribute(&attributes, "PartName")?.to_owned(),
                )?,
                content_type: required_attribute(&attributes, "ContentType")?.to_owned(),
            });
        }
        _ => {
            validate_attribute_count(element, limits)?;
        }
    }

    Ok(())
}

fn handle_relationship_element(
    source: Option<&PartName>,
    element: &BytesStart<'_>,
    root_seen: &mut bool,
    output: &mut RelationshipSet,
    limits: XmlLimits,
) -> Result<(), XmlParseError> {
    let name = element.local_name();

    if !*root_seen {
        if name.as_ref() != RELATIONSHIPS_ROOT {
            return Err(XmlParseError::UnexpectedRoot {
                expected: RELATIONSHIPS_ROOT,
                actual: name.as_ref().to_owned(),
            });
        }
        *root_seen = true;
        validate_attribute_count(element, limits)?;
        return Ok(());
    }

    if name.as_ref() != "Relationship" {
        validate_attribute_count(element, limits)?;
        return Ok(());
    }

    let attributes = collect_attributes(element, limits)?;
    let id = required_attribute(&attributes, "Id")?;
    let relationship_type = required_attribute(&attributes, "Type")?;
    let target = required_attribute(&attributes, "Target")?;
    let target_mode = optional_attribute(&attributes, "TargetMode");

    if id.is_empty() {
        return Err(XmlParseError::MissingAttribute("Id"));
    }

    let target = if target_mode == Some("External") {
        RelationshipTarget::External(target.to_owned())
    } else {
        RelationshipTarget::Internal(resolve_internal_target(source, target)?)
    };

    output
        .insert(Relationship {
            id: RelationshipId::new(id),
            relationship_type: relationship_type.to_owned(),
            target,
        })
        .map_err(|error| match error {
            PackageError::DuplicateRelationshipId(id) => {
                XmlParseError::DuplicateRelationshipId(id)
            }
            other => XmlParseError::Malformed(other.to_string()),
        })
}

fn collect_attributes(
    element: &BytesStart<'_>,
    limits: XmlLimits,
) -> Result<Vec<(String, String)>, XmlParseError> {
    let mut output = Vec::new();

    for (index, attribute) in element.attributes().enumerate() {
        if index >= limits.max_attributes_per_element {
            return Err(XmlParseError::TooManyAttributes);
        }

        let attribute = attribute.map_err(|error| XmlParseError::Malformed(error.to_string()))?;
        let key = attribute.key.local_name();
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| XmlParseError::Malformed(error.to_string()))?;

        output.push((key.as_ref().to_owned(), value.into_owned()));
    }

    Ok(output)
}

fn validate_attribute_count(
    element: &BytesStart<'_>,
    limits: XmlLimits,
) -> Result<(), XmlParseError> {
    for (index, attribute) in element.attributes().enumerate() {
        if index >= limits.max_attributes_per_element {
            return Err(XmlParseError::TooManyAttributes);
        }
        attribute.map_err(|error| XmlParseError::Malformed(error.to_string()))?;
    }
    Ok(())
}

fn required_attribute<'a>(
    attributes: &'a [(String, String)],
    name: &'static str,
) -> Result<&'a str, XmlParseError> {
    optional_attribute(attributes, name).ok_or(XmlParseError::MissingAttribute(name))
}

fn optional_attribute<'a>(
    attributes: &'a [(String, String)],
    name: &str,
) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn ensure_input_limit(input: &[u8], limits: XmlLimits) -> Result<(), XmlParseError> {
    if input.len() > limits.max_input_bytes {
        return Err(XmlParseError::InputTooLarge);
    }
    Ok(())
}

fn checked_depth(current: usize, limits: XmlLimits) -> Result<usize, XmlParseError> {
    let next = current
        .checked_add(1)
        .ok_or(XmlParseError::DepthExceeded)?;
    if next > limits.max_depth {
        return Err(XmlParseError::DepthExceeded);
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

    const RELATIONSHIPS: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="styles" Target="styles.xml"/>
  <Relationship Id="rId2" Type="hyperlink" Target="https://example.invalid/a?x=1&amp;y=2" TargetMode="External"/>
</Relationships>"#;

    #[test]
    fn parses_content_type_defaults_and_overrides() {
        let map = parse_content_types(CONTENT_TYPES).unwrap();

        assert_eq!(
            map.content_type_for(&PartName::new("/custom/item.xml").unwrap()),
            Some("application/xml")
        );
        assert_eq!(
            map.content_type_for(&PartName::new("/word/document.xml").unwrap()),
            Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            )
        );
    }

    #[test]
    fn parses_internal_and_external_relationships() {
        let source = PartName::new("/word/document.xml").unwrap();
        let relationships = parse_relationships(Some(&source), RELATIONSHIPS).unwrap();

        let styles = relationships.get(&RelationshipId::new("rId1")).unwrap();
        assert_eq!(
            styles.target,
            RelationshipTarget::Internal(PartName::new("/word/styles.xml").unwrap())
        );

        let hyperlink = relationships.get(&RelationshipId::new("rId2")).unwrap();
        assert_eq!(
            hyperlink.target,
            RelationshipTarget::External(
                "https://example.invalid/a?x=1&y=2".to_owned()
            )
        );
    }

    #[test]
    fn rejects_doctype() {
        let xml = br#"<!DOCTYPE Types><Types/>"#;
        assert_eq!(
            parse_content_types(xml),
            Err(XmlParseError::UnsupportedDocType)
        );
    }

    #[test]
    fn enforces_input_size_limit_before_parsing() {
        let limits = XmlLimits {
            max_input_bytes: 8,
            ..XmlLimits::default()
        };

        assert_eq!(
            parse_content_types_with_limits(CONTENT_TYPES, limits),
            Err(XmlParseError::InputTooLarge)
        );
    }

    #[test]
    fn enforces_depth_limit() {
        let xml = br#"<Types><a><b><c/></b></a></Types>"#;
        let limits = XmlLimits {
            max_depth: 2,
            ..XmlLimits::default()
        };

        assert_eq!(
            parse_content_types_with_limits(xml, limits),
            Err(XmlParseError::DepthExceeded)
        );
    }

    #[test]
    fn rejects_duplicate_relationship_ids() {
        let xml = br#"<Relationships>
          <Relationship Id="rId1" Type="a" Target="one.xml"/>
          <Relationship Id="rId1" Type="b" Target="two.xml"/>
        </Relationships>"#;

        assert_eq!(
            parse_relationships(None, xml),
            Err(XmlParseError::DuplicateRelationshipId("rId1".to_owned()))
        );
    }
}

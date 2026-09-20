use crate::PartName;
use std::collections::BTreeMap;

/// One rule from `[Content_Types].xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentTypeRule {
    Default {
        extension: String,
        content_type: String,
    },
    Override {
        part_name: PartName,
        content_type: String,
    },
}

/// Compact content-type index.
///
/// Overrides always win over extension defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentTypeMap {
    defaults: BTreeMap<String, String>,
    overrides: BTreeMap<PartName, String>,
}

impl ContentTypeMap {
    pub fn insert(&mut self, rule: ContentTypeRule) {
        match rule {
            ContentTypeRule::Default {
                extension,
                content_type,
            } => {
                self.defaults
                    .insert(normalize_extension(&extension), content_type);
            }
            ContentTypeRule::Override {
                part_name,
                content_type,
            } => {
                self.overrides.insert(part_name, content_type);
            }
        }
    }

    #[must_use]
    pub fn content_type_for(&self, part: &PartName) -> Option<&str> {
        if let Some(value) = self.overrides.get(part) {
            return Some(value);
        }

        part.extension()
            .and_then(|extension| self.defaults.get(&normalize_extension(extension)))
            .map(String::as_str)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.defaults.len() + self.overrides.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.defaults.is_empty() && self.overrides.is_empty()
    }
}

fn normalize_extension(extension: &str) -> String {
    extension.trim_start_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins_over_default() {
        let mut map = ContentTypeMap::default();
        map.insert(ContentTypeRule::Default {
            extension: "xml".into(),
            content_type: "application/xml".into(),
        });
        let document = PartName::new("/word/document.xml").unwrap();
        map.insert(ContentTypeRule::Override {
            part_name: document.clone(),
            content_type:
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
                    .into(),
        });

        assert_eq!(
            map.content_type_for(&document),
            Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            )
        );
        assert_eq!(
            map.content_type_for(&PartName::new("/custom/item.XML").unwrap()),
            Some("application/xml")
        );
    }
}

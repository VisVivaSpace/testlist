//! Types for testlist definition files (.testlist.ron).

use serde::{Deserialize, Deserializer, Serialize};

/// Metadata for a testlist definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub description: String,
    pub created: String,
    pub version: String,
}

/// A checklist item with an ID and text.
///
/// Supports backward-compatible deserialization from plain strings
/// (auto-generates IDs like "item-0", "item-1", etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChecklistItem {
    pub id: String,
    pub text: String,
}

/// Deserialize a `Vec<ChecklistItem>` from either:
/// - A `Vec<String>` (old format) — auto-generates IDs as `"{prefix}-{index}"`
/// - A `Vec<ChecklistItem>` (new format)
///
/// `prefix` is provided by the caller (e.g. "setup" or "verify").
pub fn deserialize_checklist_items<'de, D>(
    deserializer: D,
    prefix: &str,
) -> std::result::Result<Vec<ChecklistItem>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrItem {
        Plain(String),
        Item(ChecklistItem),
    }

    let items: Vec<StringOrItem> = Vec::deserialize(deserializer)?;
    Ok(items
        .into_iter()
        .enumerate()
        .map(|(i, item)| match item {
            StringOrItem::Plain(text) => ChecklistItem {
                id: format!("{}-{}", prefix, i),
                text,
            },
            StringOrItem::Item(item) => item,
        })
        .collect())
}

fn deserialize_setup<'de, D>(deserializer: D) -> std::result::Result<Vec<ChecklistItem>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_checklist_items(deserializer, "setup")
}

fn deserialize_verify<'de, D>(deserializer: D) -> std::result::Result<Vec<ChecklistItem>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_checklist_items(deserializer, "verify")
}

/// A single test item to verify.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Test {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(default, deserialize_with = "deserialize_setup")]
    pub setup: Vec<ChecklistItem>,
    pub action: String,
    #[serde(default, deserialize_with = "deserialize_verify")]
    pub verify: Vec<ChecklistItem>,
    pub suggested_command: Option<String>,
}

/// Root type for testlist definition files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testlist {
    pub meta: Meta,
    pub tests: Vec<Test>,
}

impl Testlist {
    /// Load a testlist from a RON file.
    pub fn load(path: &std::path::Path) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let testlist: Testlist = ron::from_str(&content)?;
        Ok(testlist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_testlist_old_format() {
        let ron_str = r#"
Testlist(
    meta: Meta(
        title: "Test Checklist",
        description: "A test checklist",
        created: "2025-01-24T10:00:00Z",
        version: "1",
    ),
    tests: [
        Test(
            id: "build",
            title: "Build successfully",
            description: "Verify the build completes",
            setup: [],
            action: "Run cargo build",
            verify: [
                "Build completes without errors",
            ],
            suggested_command: Some("cargo build"),
        ),
    ],
)
"#;
        let testlist: Testlist = ron::from_str(ron_str).unwrap();
        assert_eq!(testlist.meta.title, "Test Checklist");
        assert_eq!(testlist.tests.len(), 1);
        assert_eq!(testlist.tests[0].id, "build");
        assert_eq!(testlist.tests[0].verify.len(), 1);
        assert_eq!(testlist.tests[0].verify[0].id, "verify-0");
        assert_eq!(
            testlist.tests[0].verify[0].text,
            "Build completes without errors"
        );
        assert_eq!(
            testlist.tests[0].suggested_command,
            Some("cargo build".to_string())
        );
    }

    #[test]
    fn test_parse_testlist_new_format() {
        let ron_str = r#"
Testlist(
    meta: Meta(
        title: "Test",
        description: "Test",
        created: "2025-01-24",
        version: "1",
    ),
    tests: [
        Test(
            id: "t1",
            title: "Test 1",
            description: "Desc",
            setup: [
                ChecklistItem(id: "s1", text: "Step one"),
                ChecklistItem(id: "s2", text: "Step two"),
            ],
            action: "Do it",
            verify: [
                ChecklistItem(id: "v1", text: "Check one"),
            ],
            suggested_command: None,
        ),
    ],
)
"#;
        let testlist: Testlist = ron::from_str(ron_str).unwrap();
        assert_eq!(testlist.tests[0].setup.len(), 2);
        assert_eq!(testlist.tests[0].setup[0].id, "s1");
        assert_eq!(testlist.tests[0].setup[0].text, "Step one");
        assert_eq!(testlist.tests[0].setup[1].id, "s2");
        assert_eq!(testlist.tests[0].verify[0].id, "v1");
    }

    #[test]
    fn test_parse_testlist_mixed_backward_compat() {
        // Old-style plain strings should auto-generate IDs
        let ron_str = r#"
Testlist(
    meta: Meta(
        title: "Test",
        description: "Test",
        created: "2025-01-24",
        version: "1",
    ),
    tests: [
        Test(
            id: "t1",
            title: "Test 1",
            description: "Desc",
            setup: [
                "Step A",
                "Step B",
            ],
            action: "Do it",
            verify: [
                "Verify A",
                "Verify B",
                "Verify C",
            ],
            suggested_command: None,
        ),
    ],
)
"#;
        let testlist: Testlist = ron::from_str(ron_str).unwrap();
        assert_eq!(testlist.tests[0].setup[0].id, "setup-0");
        assert_eq!(testlist.tests[0].setup[0].text, "Step A");
        assert_eq!(testlist.tests[0].setup[1].id, "setup-1");
        assert_eq!(testlist.tests[0].verify[0].id, "verify-0");
        assert_eq!(testlist.tests[0].verify[2].id, "verify-2");
    }
}

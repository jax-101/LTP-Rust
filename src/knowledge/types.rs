use serde::{Deserialize, Serialize};

/// Naturaleza del knowledge item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    Measurement,
    Testimony,
    Hypothesis,
    Document,
    Observation,
    Derived,
}

/// Estado epistemico del item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatus {
    Unverified,
    Verified,
    Refuted,
    Superseded,
}

/// Nivel de confianza declarado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Tipo de relacion con una entidad del grafo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRelation {
    Supports,
    Contradicts,
    Contextualizes,
}

/// Origen del conocimiento (al menos uno de uri o excerpt debe estar presente).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

impl KnowledgeSource {
    /// Validates that at least one of uri or excerpt is present.
    pub fn is_valid(&self) -> bool {
        self.uri.is_some() || self.excerpt.is_some()
    }
}

/// Vinculo a una entidad del grafo causal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgeLink {
    pub target: String,
    pub relation: KnowledgeRelation,
}

/// Entidad de conocimiento del Knowledge Pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    #[serde(rename = "type")]
    pub knowledge_type: KnowledgeType,
    pub label: String,
    pub status: KnowledgeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    pub source: KnowledgeSource,
    pub captured: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<KnowledgeLink>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_source_valid_with_uri() {
        let source = KnowledgeSource {
            uri: Some("https://example.com".to_string()),
            excerpt: None,
        };
        assert!(source.is_valid());
    }

    #[test]
    fn knowledge_source_valid_with_excerpt() {
        let source = KnowledgeSource {
            uri: None,
            excerpt: Some("Some text".to_string()),
        };
        assert!(source.is_valid());
    }

    #[test]
    fn knowledge_source_invalid_when_empty() {
        let source = KnowledgeSource {
            uri: None,
            excerpt: None,
        };
        assert!(!source.is_valid());
    }

    #[test]
    fn round_trip_serialization() {
        let item = KnowledgeItem {
            id: "KN-001".to_string(),
            knowledge_type: KnowledgeType::Measurement,
            label: "Media entrega 18.3 dias".to_string(),
            status: KnowledgeStatus::Verified,
            confidence: Some(Confidence::High),
            source: KnowledgeSource {
                uri: Some("obsidian://vault/note.md".to_string()),
                excerpt: Some("Extracto".to_string()),
            },
            captured: "2026-08-17".to_string(),
            links: vec![KnowledgeLink {
                target: "UDE-003".to_string(),
                relation: KnowledgeRelation::Supports,
            }],
            tags: vec!["logistica".to_string()],
        };

        let json = serde_json::to_string_pretty(&item).unwrap();
        let deserialized: KnowledgeItem = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "KN-001");
        assert_eq!(deserialized.knowledge_type, KnowledgeType::Measurement);
        assert_eq!(deserialized.status, KnowledgeStatus::Verified);
        assert_eq!(deserialized.confidence, Some(Confidence::High));
        assert_eq!(deserialized.links.len(), 1);
        assert_eq!(deserialized.tags, vec!["logistica"]);
    }

    #[test]
    fn optional_fields_omitted_in_json() {
        let item = KnowledgeItem {
            id: "KN-002".to_string(),
            knowledge_type: KnowledgeType::Testimony,
            label: "Director dice mejoraron".to_string(),
            status: KnowledgeStatus::Unverified,
            confidence: None,
            source: KnowledgeSource {
                uri: Some("slack://C04/p123".to_string()),
                excerpt: None,
            },
            captured: "2026-08-17".to_string(),
            links: vec![],
            tags: vec![],
        };

        let json = serde_json::to_string_pretty(&item).unwrap();
        assert!(!json.contains("confidence"));
        assert!(!json.contains("links"));
        assert!(!json.contains("tags"));
        assert!(!json.contains("excerpt"));
    }
}

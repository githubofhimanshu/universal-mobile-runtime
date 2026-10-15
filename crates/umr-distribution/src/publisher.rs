use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A verified or registered application publisher in the UMR ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Publisher {
    pub id: String,
    pub name: String,
    pub public_key: String,
    pub contact_uri: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}

/// In-memory and persistent registry of application publishers.
#[derive(Debug, Clone)]
pub struct PublisherRegistry {
    publishers: HashMap<String, Publisher>,
}

impl Default for PublisherRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PublisherRegistry {
    /// Creates a new empty publisher registry.
    pub fn new() -> Self {
        Self {
            publishers: HashMap::new(),
        }
    }

    /// Creates a registry pre-seeded with canonical verified publishers.
    pub fn with_official_publishers() -> Self {
        let mut registry = Self::new();
        let now = Utc::now();

        registry.register(Publisher {
            id: "pub_umr_official".to_string(),
            name: "Universal Mobile Runtime Core Team".to_string(),
            public_key: "pubkey_umr_core_ed25519_master".to_string(),
            contact_uri: "https://github.com/universal-mobile-runtime".to_string(),
            verified: true,
            created_at: now,
        });

        registry.register(Publisher {
            id: "pub_sample_corp".to_string(),
            name: "Sample Corporation".to_string(),
            public_key: "pubkey_sample_corp_2026_ed25519".to_string(),
            contact_uri: "https://example.com/support".to_string(),
            verified: true,
            created_at: now,
        });

        registry.register(Publisher {
            id: "pub_open_mobile".to_string(),
            name: "Open Mobile Foundation".to_string(),
            public_key: "pubkey_open_mobile_key_prod".to_string(),
            contact_uri: "https://openmobile.org".to_string(),
            verified: true,
            created_at: now,
        });

        registry
    }

    /// Registers a new publisher.
    pub fn register(&mut self, publisher: Publisher) {
        self.publishers.insert(publisher.id.clone(), publisher);
    }

    /// Retrieves a publisher by ID.
    pub fn get(&self, id: &str) -> Option<&Publisher> {
        self.publishers.get(id)
    }

    /// Checks if a publisher exists and is verified.
    pub fn is_verified(&self, id: &str) -> bool {
        self.publishers.get(id).map_or(false, |p| p.verified)
    }

    /// Lists all publishers.
    pub fn list(&self) -> Vec<&Publisher> {
        self.publishers.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_publishers_seeded() {
        let registry = PublisherRegistry::with_official_publishers();
        let umr_pub = registry.get("pub_umr_official").expect("official publisher missing");
        assert!(umr_pub.verified);
        assert_eq!(umr_pub.name, "Universal Mobile Runtime Core Team");
        assert!(registry.is_verified("pub_sample_corp"));
        assert!(!registry.is_verified("pub_unknown_unverified"));
    }

    #[test]
    fn test_custom_publisher_registration() {
        let mut registry = PublisherRegistry::new();
        registry.register(Publisher {
            id: "pub_indie_dev".to_string(),
            name: "Indie Game Developer".to_string(),
            public_key: "key_indie_1234".to_string(),
            contact_uri: "https://indie.dev".to_string(),
            verified: false,
            created_at: Utc::now(),
        });

        let p = registry.get("pub_indie_dev").unwrap();
        assert_eq!(p.name, "Indie Game Developer");
        assert!(!p.verified);
    }
}


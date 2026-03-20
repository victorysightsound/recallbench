pub mod download;
pub mod longmemeval;
pub mod custom;

use std::collections::HashMap;

use anyhow::Result;

use crate::traits::BenchmarkDataset;

/// Registry mapping dataset names to loader functions.
pub struct DatasetRegistry {
    datasets: HashMap<String, DatasetInfo>,
}

/// Metadata about an available dataset.
#[derive(Debug, Clone)]
pub struct DatasetInfo {
    pub name: String,
    pub description: String,
    pub variants: Vec<String>,
}

impl DatasetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            datasets: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.datasets.insert(
            "longmemeval".to_string(),
            DatasetInfo {
                name: "longmemeval".to_string(),
                description: "LongMemEval (ICLR 2025) — 500 questions testing 5 memory abilities across 7 question types".to_string(),
                variants: vec!["oracle".to_string(), "small".to_string(), "medium".to_string()],
            },
        );
    }

    /// List all available datasets.
    pub fn list(&self) -> Vec<&DatasetInfo> {
        let mut datasets: Vec<_> = self.datasets.values().collect();
        datasets.sort_by(|a, b| a.name.cmp(&b.name));
        datasets
    }

    /// Get info for a specific dataset.
    pub fn get(&self, name: &str) -> Option<&DatasetInfo> {
        self.datasets.get(name)
    }

    /// Load a dataset by name and variant.
    pub async fn load(&self, name: &str, variant: &str, force_download: bool) -> Result<Box<dyn BenchmarkDataset>> {
        match name {
            "longmemeval" => {
                let dataset = longmemeval::LongMemEvalDataset::load(variant, force_download).await?;
                Ok(Box::new(dataset))
            }
            _ => anyhow::bail!("Unknown dataset: {name}. Run 'recallbench datasets' to see available datasets."),
        }
    }
}

impl Default for DatasetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_longmemeval() {
        let registry = DatasetRegistry::new();
        let info = registry.get("longmemeval").unwrap();
        assert_eq!(info.name, "longmemeval");
        assert!(info.variants.contains(&"oracle".to_string()));
    }

    #[test]
    fn registry_list() {
        let registry = DatasetRegistry::new();
        let datasets = registry.list();
        assert!(!datasets.is_empty());
    }

    #[test]
    fn registry_unknown_returns_none() {
        let registry = DatasetRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }
}

pub mod convomem;
pub mod custom;
pub mod download;
pub mod halumem;
pub mod locomo;
pub mod longmemeval;
pub mod mab;
pub mod membench;

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
        self.datasets.insert("longmemeval".to_string(), DatasetInfo {
            name: "longmemeval".to_string(),
            description: "LongMemEval (ICLR 2025) — 500 questions, 5 memory abilities, 7 types".to_string(),
            variants: vec!["oracle".to_string(), "small".to_string(), "medium".to_string()],
        });
        self.datasets.insert("locomo".to_string(), DatasetInfo {
            name: "locomo".to_string(),
            description: "LoCoMo (Snap Research) — long-context conversation memory".to_string(),
            variants: vec!["default".to_string()],
        });
        self.datasets.insert("convomem".to_string(), DatasetInfo {
            name: "convomem".to_string(),
            description: "ConvoMem (Salesforce) — conversational memory, 6 evidence categories".to_string(),
            variants: vec!["user_evidence".to_string(), "assistant_facts".to_string(), "changing".to_string(), "abstention".to_string(), "preference".to_string(), "implicit_connection".to_string()],
        });
        self.datasets.insert("membench".to_string(), DatasetInfo {
            name: "membench".to_string(),
            description: "MemBench (ACL 2025) — multi-aspect memory evaluation, multiple-choice QA".to_string(),
            variants: vec!["simple".to_string(), "aggregative".to_string(), "comparative".to_string(), "conditional".to_string(), "knowledge_update".to_string(), "highlevel".to_string()],
        });
        self.datasets.insert("memoryagentbench".to_string(), DatasetInfo {
            name: "memoryagentbench".to_string(),
            description: "MemoryAgentBench (ICLR 2026) — selective forgetting, fact consolidation".to_string(),
            variants: vec!["default".to_string()],
        });
        self.datasets.insert("halumem".to_string(), DatasetInfo {
            name: "halumem".to_string(),
            description: "HaluMem (MemTensor) — memory hallucination detection".to_string(),
            variants: vec!["medium".to_string(), "long".to_string()],
        });
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

    /// Resolve a dataset name, including category aliases.
    fn resolve_name<'a>(&self, name: &'a str) -> &'a str {
        match name {
            "recall" => "longmemeval",
            "longturn" => "locomo",
            "conversation" => "convomem",
            "multiaspect" => "membench",
            "forgetting" => "memoryagentbench",
            "hallucination" => "halumem",
            other => other,
        }
    }

    /// Load a dataset by name (or alias) and variant.
    pub async fn load(&self, name: &str, variant: &str, force_download: bool) -> Result<Box<dyn BenchmarkDataset>> {
        let resolved = self.resolve_name(name);
        match resolved {
            "longmemeval" => {
                let dataset = longmemeval::LongMemEvalDataset::load(variant, force_download).await?;
                Ok(Box::new(dataset))
            }
            "locomo" => {
                let dataset = locomo::LoCoMoDataset::load(force_download).await?;
                Ok(Box::new(dataset))
            }
            "convomem" => {
                let dataset = convomem::ConvoMemDataset::load(variant, force_download).await?;
                Ok(Box::new(dataset))
            }
            "membench" => {
                let category = if variant == "default" { "simple" } else { variant };
                let dataset = membench::MemBenchDataset::load(category, force_download).await?;
                Ok(Box::new(dataset))
            }
            "halumem" => {
                let dataset = halumem::HaluMemDataset::load(variant, force_download).await?;
                Ok(Box::new(dataset))
            }
            "memoryagentbench" => {
                anyhow::bail!(
                    "MemoryAgentBench uses Parquet format. Export to JSON first:\n\
                     pip install datasets\n\
                     python -c \"from datasets import load_dataset; import json; ds = load_dataset('ai-hyz/MemoryAgentBench'); [open(f'mab_{{s}}.json','w').write(json.dumps(list(ds[s]))) for s in ds]\"\n\
                     Then use: recallbench validate mab_Accurate_Retrieval.json"
                )
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
    fn registry_has_all_datasets() {
        let registry = DatasetRegistry::new();
        assert!(registry.get("longmemeval").is_some());
        assert!(registry.get("locomo").is_some());
        assert!(registry.get("convomem").is_some());
        assert!(registry.get("membench").is_some());
        assert!(registry.get("memoryagentbench").is_some());
        assert!(registry.get("halumem").is_some());
    }

    #[test]
    fn registry_list_sorted() {
        let registry = DatasetRegistry::new();
        let datasets = registry.list();
        assert_eq!(datasets.len(), 6);
        // Verify sorted
        let names: Vec<_> = datasets.iter().map(|d| d.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn registry_unknown_returns_none() {
        let registry = DatasetRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }
}

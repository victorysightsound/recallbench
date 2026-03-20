pub mod csv;
pub mod failure;
pub mod json;
pub mod markdown;
pub mod table;

/// Supported report output formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReportFormat {
    Table,
    Markdown,
    Json,
    Csv,
}

impl ReportFormat {
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Self::Table),
            "markdown" | "md" => Ok(Self::Markdown),
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            _ => anyhow::bail!("Unknown report format: {s}. Use table, markdown, json, or csv."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_formats() {
        assert_eq!(ReportFormat::from_str("table").unwrap(), ReportFormat::Table);
        assert_eq!(ReportFormat::from_str("markdown").unwrap(), ReportFormat::Markdown);
        assert_eq!(ReportFormat::from_str("md").unwrap(), ReportFormat::Markdown);
        assert_eq!(ReportFormat::from_str("json").unwrap(), ReportFormat::Json);
        assert_eq!(ReportFormat::from_str("csv").unwrap(), ReportFormat::Csv);
        assert!(ReportFormat::from_str("unknown").is_err());
    }
}

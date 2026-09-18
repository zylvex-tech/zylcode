use anyhow::Result;

use super::types::*;

/// File processor for document and code analysis
pub struct FileProcessor {
    stats: std::sync::RwLock<FileProcessorStats>,
}

#[derive(Debug, Default)]
struct FileProcessorStats {
    processed_count: u64,
    total_processing_time_ms: u64,
}

impl FileProcessor {
    /// Create a new file processor
    pub async fn new() -> Result<Self> {
        Ok(Self {
            stats: std::sync::RwLock::new(FileProcessorStats::default()),
        })
    }

    /// Process file
    pub async fn process_file(&self, file: File) -> Result<ProcessedFile> {
        let start = std::time::Instant::now();
        
        // Analyze file based on type
        let (summary, language, line_count, word_count) = match file.file_type {
            FileType::Code => self.analyze_code(&file).await?,
            FileType::Text => self.analyze_text(&file).await?,
            FileType::Document => self.analyze_document(&file).await?,
            _ => self.analyze_generic(&file).await?,
        };
        
        let entities = self.extract_entities(&file).await?;
        let confidence = self.calculate_confidence(&file).await?;
        let analysis = self.analyze_file(&file).await?;
        
        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().unwrap();
            stats.processed_count += 1;
            stats.total_processing_time_ms += duration;
        }

        Ok(ProcessedFile {
            summary,
            file_type: file.file_type,
            language,
            line_count,
            word_count,
            entities,
            confidence,
            analysis,
        })
    }

    /// Analyze code file
    async fn analyze_code(&self, file: &File) -> Result<(String, Option<String>, Option<usize>, Option<usize>)> {
        let content = String::from_utf8_lossy(&file.content);
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        let word_count = content.split_whitespace().count();
        
        // Detect language from file extension
        let language = self.detect_language(&file.name);
        
        // Generate summary
        let summary = format!(
            "Code file with {} lines and {} words. Language: {}",
            line_count,
            word_count,
            language.as_deref().unwrap_or("Unknown")
        );
        
        Ok((summary, language, Some(line_count), Some(word_count)))
    }

    /// Analyze text file
    async fn analyze_text(&self, file: &File) -> Result<(String, Option<String>, Option<usize>, Option<usize>)> {
        let content = String::from_utf8_lossy(&file.content);
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        let word_count = content.split_whitespace().count();
        
        let summary = format!(
            "Text file with {} lines and {} words",
            line_count,
            word_count
        );
        
        Ok((summary, None, Some(line_count), Some(word_count)))
    }

    /// Analyze document file
    async fn analyze_document(&self, file: &File) -> Result<(String, Option<String>, Option<usize>, Option<usize>)> {
        let content = String::from_utf8_lossy(&file.content);
        let word_count = content.split_whitespace().count();
        
        let summary = format!(
            "Document file with {} words. Format: {}",
            word_count,
            file.name.split('.').next_back().unwrap_or("unknown")
        );
        
        Ok((summary, None, None, Some(word_count)))
    }

    /// Analyze generic file
    async fn analyze_generic(&self, file: &File) -> Result<(String, Option<String>, Option<usize>, Option<usize>)> {
        let summary = format!(
            "File: {} ({} bytes)",
            file.name,
            file.size
        );
        
        Ok((summary, None, None, None))
    }

    /// Detect programming language from filename
    fn detect_language(&self, filename: &str) -> Option<String> {
        let extension = filename.split('.').next_back()?.to_lowercase();
        
        match extension.as_str() {
            "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript".to_string()),
            "ts" | "tsx" | "mts" | "cts" => Some("TypeScript".to_string()),
            "py" | "pyw" | "pyi" => Some("Python".to_string()),
            "rs" => Some("Rust".to_string()),
            "go" => Some("Go".to_string()),
            "java" => Some("Java".to_string()),
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hxx" => Some("C++".to_string()),
            "c" | "h" => Some("C".to_string()),
            "rb" => Some("Ruby".to_string()),
            "php" => Some("PHP".to_string()),
            "swift" => Some("Swift".to_string()),
            "kt" | "kts" => Some("Kotlin".to_string()),
            "scala" | "sc" => Some("Scala".to_string()),
            "clj" | "cljs" | "cljc" => Some("Clojure".to_string()),
            "hs" | "lhs" => Some("Haskell".to_string()),
            "ex" | "exs" => Some("Elixir".to_string()),
            "erl" | "hrl" => Some("Erlang".to_string()),
            "sql" => Some("SQL".to_string()),
            "html" | "htm" => Some("HTML".to_string()),
            "css" | "scss" | "sass" | "less" => Some("CSS".to_string()),
            "json" => Some("JSON".to_string()),
            "yaml" | "yml" => Some("YAML".to_string()),
            "toml" => Some("TOML".to_string()),
            "xml" => Some("XML".to_string()),
            "md" | "markdown" => Some("Markdown".to_string()),
            "sh" | "bash" | "zsh" => Some("Shell".to_string()),
            "ps1" | "psm1" | "psd1" => Some("PowerShell".to_string()),
            "r" | "R" => Some("R".to_string()),
            "dart" => Some("Dart".to_string()),
            "lua" => Some("Lua".to_string()),
            "pl" | "pm" => Some("Perl".to_string()),
            _ => None,
        }
    }

    /// Extract entities from file
    async fn extract_entities(&self, file: &File) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        
        let content = String::from_utf8_lossy(&file.content);
        
        // Extract file paths
        let path_regex = regex::Regex::new(r"(?:[\w\-\.]+/)+[\w\-\.]+").unwrap();
        for mat in path_regex.find_iter(&content) {
            entities.push(Entity {
                name: "file_path".to_string(),
                entity_type: EntityType::FilePath,
                value: mat.as_str().to_string(),
                confidence: 0.9,
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        // Extract URLs
        let url_regex = regex::Regex::new(r"https?://[^\s]+").unwrap();
        for mat in url_regex.find_iter(&content) {
            entities.push(Entity {
                name: "url".to_string(),
                entity_type: EntityType::URL,
                value: mat.as_str().to_string(),
                confidence: 0.95,
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        // Extract emails
        let email_regex = regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
        for mat in email_regex.find_iter(&content) {
            entities.push(Entity {
                name: "email".to_string(),
                entity_type: EntityType::Email,
                value: mat.as_str().to_string(),
                confidence: 0.95,
                start_pos: mat.start(),
                end_pos: mat.end(),
            });
        }
        
        Ok(entities)
    }

    /// Calculate confidence
    async fn calculate_confidence(&self, file: &File) -> Result<f64> {
        let mut confidence: f64 = 0.7; // Base confidence
        
        // Larger files = higher confidence
        if file.size > 1024 {
            confidence += 0.1;
        }
        
        // Known file types = higher confidence
        match file.file_type {
            FileType::Code | FileType::Text | FileType::Document => {
                confidence += 0.1;
            }
            _ => {}
        }
        
        Ok(confidence.min(1.0))
    }

    /// Analyze file for issues and metrics
    async fn analyze_file(&self, file: &File) -> Result<FileAnalysis> {
        let content = String::from_utf8_lossy(&file.content);
        
        // Calculate complexity (simple metric)
        let complexity = if content.contains("if") || content.contains("for") || content.contains("while") {
            0.7
        } else {
            0.3
        };
        
        // Calculate maintainability
        let maintainability = if content.lines().count() < 100 {
            0.8
        } else if content.lines().count() < 500 {
            0.6
        } else {
            0.4
        };
        
        // Extract dependencies
        let mut dependencies = Vec::new();
        let import_regex = regex::Regex::new(r#"(?:import|from|require)\s+['"]([^'"]+)['"]"#).unwrap();
        for cap in import_regex.captures_iter(&content) {
            if let Some(dep) = cap.get(1) {
                dependencies.push(dep.as_str().to_string());
            }
        }
        
        // Detect issues
        let mut issues = Vec::new();
        
        // Check for long lines
        for (i, line) in content.lines().enumerate() {
            if line.len() > 120 {
                issues.push(Issue {
                    severity: IssueSeverity::Warning,
                    message: format!("Line {} is too long ({} characters)", i + 1, line.len()),
                    line: Some(i + 1),
                    column: None,
                    suggestion: Some("Consider breaking this line into multiple lines".to_string()),
                });
            }
        }
        
        // Check for TODO/FIXME comments
        let todo_regex = regex::Regex::new(r"(?i)(TODO|FIXME|HACK|XXX)").unwrap();
        for (i, line) in content.lines().enumerate() {
            if todo_regex.is_match(line) {
                issues.push(Issue {
                    severity: IssueSeverity::Info,
                    message: format!("Found TODO/FIXME comment on line {}", i + 1),
                    line: Some(i + 1),
                    column: None,
                    suggestion: Some("Consider addressing this TODO/FIXME".to_string()),
                });
            }
        }
        
        Ok(FileAnalysis {
            complexity,
            maintainability,
            test_coverage: None,
            dependencies,
            issues,
        })
    }

    /// Get processing statistics
    pub async fn get_stats(&self) -> u64 {
        self.stats.read().unwrap().processed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_code_file_processing() {
        let processor = FileProcessor::new().await.unwrap();
        
        let file = File {
            name: "main.rs".to_string(),
            path: "/src/main.rs".to_string(),
            content: b"fn main() {\n    println!(\"Hello, world!\");\n}".to_vec(),
            file_type: FileType::Code,
            size: 100,
            metadata: HashMap::new(),
        };
        
        let result = processor.process_file(file).await.unwrap();
        assert_eq!(result.language, Some("Rust".to_string()));
        assert!(result.line_count.is_some());
        assert!(result.word_count.is_some());
    }

    #[tokio::test]
    async fn test_text_file_processing() {
        let processor = FileProcessor::new().await.unwrap();
        
        let file = File {
            name: "README.md".to_string(),
            path: "/README.md".to_string(),
            content: b"# Hello World\nThis is a test file.".to_vec(),
            file_type: FileType::Text,
            size: 100,
            metadata: HashMap::new(),
        };
        
        let result = processor.process_file(file).await.unwrap();
        assert!(result.summary.contains("Text file"));
    }
}
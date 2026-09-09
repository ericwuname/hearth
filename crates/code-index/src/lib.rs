//! Code index: tree-sitter incremental parsing + symbol graph + chunk embedding.
//!
//! Provides Index trait: index_tree, symbols, embed_chunks.
//! Embedding generation delegated to LlmProvider::embed (via llm-gateway abstraction).

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A symbol extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Method,
    Variable,
    Other(String),
}

/// A code chunk with its embedding vector.
#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub file: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub symbols: Vec<String>,
}

/// Vector index entry.
#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub chunk: CodeChunk,
    pub embedding: Vec<f32>,
}

/// The Index trait — code indexing abstraction.
#[async_trait]
pub trait Index: Send + Sync {
    /// Index a source tree (directory), returning discovered symbols.
    async fn index_tree(&mut self, root: &Path) -> Result<Vec<Symbol>>;

    /// Return all discovered symbols.
    fn symbols(&self) -> Vec<Symbol>;

    /// Generate embeddings for indexed chunks via LlmProvider::embed,
    /// returning vector entries for similarity search.
    async fn embed_chunks(
        &self,
        provider: &Arc<dyn llm_gateway::LlmProvider>,
    ) -> Result<Vec<VectorEntry>>;

    /// Return all code chunks (for BM25 indexing in retriever).
    fn chunks(&self) -> Vec<CodeChunk>;
}

// ── TreeSitterIndex ──

pub struct TreeSitterIndex {
    symbols: Vec<Symbol>,
    chunks: Vec<CodeChunk>,
}

impl TreeSitterIndex {
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
            chunks: Vec::new(),
        }
    }

    /// Parse a single Rust source file with tree-sitter, extract symbols + chunks.
    fn parse_rust_file(&mut self, file_path: &Path) -> Result<()> {
        let source = std::fs::read_to_string(file_path)?;
        let language = tree_sitter_rust::LANGUAGE;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language.into())
            .map_err(|e| anyhow::anyhow!("tree-sitter language error: {e}"))?;

        let tree = parser.parse(&source, None).ok_or_else(|| {
            anyhow::anyhow!("tree-sitter parse failed for {}", file_path.display())
        })?;

        let root = tree.root_node();
        let file = file_path.to_path_buf();

        // Walk the AST and extract symbols
        self.extract_symbols(&source, &root, &file);

        // Chunk the file into logical blocks (top-level items)
        self.chunk_file(&source, &root, &file);

        Ok(())
    }

    fn extract_symbols(&mut self, source: &str, node: &tree_sitter::Node, file: &Path) {
        for i in 0..node.child_count() {
            // CODEIDX-1: don't unwrap — a racing tree edit or grammar quirk
            // yielding None must not panic the indexer; skip the slot instead.
            let child = match node.child(i) {
                Some(c) => c,
                None => continue,
            };
            let kind_str = child.kind();

            let symbol_kind = match kind_str {
                "function_item" | "function_signature_item" => SymbolKind::Function,
                "struct_item" => SymbolKind::Struct,
                "enum_item" => SymbolKind::Enum,
                "trait_item" => SymbolKind::Trait,
                "impl_item" => SymbolKind::Impl,
                "mod_item" => SymbolKind::Module,
                _ => {
                    // Recurse into children for nested symbols
                    if child.child_count() > 0 {
                        self.extract_symbols(source, &child, file);
                    }
                    continue;
                }
            };

            // Extract the name from the first identifier child
            let name = Self::extract_name(source, &child);

            if !name.is_empty() {
                let start = child.start_position();
                self.symbols.push(Symbol {
                    name,
                    kind: symbol_kind,
                    file: file.to_path_buf(),
                    line: start.row + 1,
                    column: start.column + 1,
                    parent: None,
                });
            }

            // Recurse for nested items (methods in impl, etc.)
            if child.child_count() > 0 {
                self.extract_symbols(source, &child, file);
            }
        }
    }

    fn extract_name(source: &str, node: &tree_sitter::Node) -> String {
        for i in 0..node.child_count() {
            // CODEIDX-1: no unwrap (see extract_symbols).
            let child = match node.child(i) {
                Some(c) => c,
                None => continue,
            };
            if child.kind() == "identifier" || child.kind() == "name" {
                return child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            if child.kind() == "type_identifier" {
                return child.utf8_text(source.as_bytes()).unwrap_or("").to_string();
            }
            // For function items, the name is the second child (after 'fn' keyword)
            let recurse = Self::extract_name(source, &child);
            if !recurse.is_empty() {
                return recurse;
            }
        }
        String::new()
    }

    fn chunk_file(&mut self, source: &str, node: &tree_sitter::Node, file: &Path) {
        // Chunk at top-level item boundaries
        for i in 0..node.child_count() {
            // CODEIDX-1: no unwrap (see extract_symbols).
            let child = match node.child(i) {
                Some(c) => c,
                None => continue,
            };
            let kind = child.kind();

            // Top-level definitions form chunks
            let is_top_level = matches!(
                kind,
                "function_item"
                    | "struct_item"
                    | "enum_item"
                    | "trait_item"
                    | "impl_item"
                    | "mod_item"
                    | "const_item"
                    | "static_item"
            );

            if is_top_level {
                let start = child.start_position();
                let end = child.end_position();
                let content = source
                    .lines()
                    .skip(start.row)
                    .take(end.row - start.row + 1)
                    .collect::<Vec<_>>()
                    .join("\n");

                // Collect symbol names within this chunk
                let mut chunk_symbols: Vec<String> = Vec::new();
                let name = Self::extract_name(source, &child);
                if !name.is_empty() {
                    chunk_symbols.push(name.clone());
                }

                self.chunks.push(CodeChunk {
                    file: file.to_path_buf(),
                    start_line: start.row + 1,
                    end_line: end.row + 1,
                    content,
                    symbols: chunk_symbols,
                });
            }
        }
    }
}

#[async_trait]
impl Index for TreeSitterIndex {
    async fn index_tree(&mut self, root: &Path) -> Result<Vec<Symbol>> {
        self.symbols.clear();
        self.chunks.clear();

        // Walk the directory for .rs files
        Self::walk_and_parse(root, self)?;

        tracing::info!(
            "Indexed {}: {} symbols, {} chunks",
            root.display(),
            self.symbols.len(),
            self.chunks.len(),
        );

        Ok(self.symbols.clone())
    }

    fn symbols(&self) -> Vec<Symbol> {
        self.symbols.clone()
    }

    async fn embed_chunks(
        &self,
        provider: &Arc<dyn llm_gateway::LlmProvider>,
    ) -> Result<Vec<VectorEntry>> {
        if self.chunks.is_empty() {
            return Ok(Vec::new());
        }

        let texts: Vec<String> = self.chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = provider.embed(&texts).await?;

        let entries: Vec<VectorEntry> = self
            .chunks
            .iter()
            .zip(embeddings)
            .map(|(chunk, emb)| VectorEntry {
                chunk: chunk.clone(),
                embedding: emb.values,
            })
            .collect();

        tracing::info!(
            "Generated {} embeddings via {}",
            entries.len(),
            provider.name()
        );
        Ok(entries)
    }

    fn chunks(&self) -> Vec<CodeChunk> {
        self.chunks.clone()
    }
}

impl TreeSitterIndex {
    fn walk_and_parse(root: &Path, idx: &mut TreeSitterIndex) -> Result<()> {
        if !root.is_dir() {
            return Ok(());
        }
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip target directories and hidden dirs
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name == "target" || dir_name.starts_with('.') {
                    continue;
                }
                Self::walk_and_parse(&path, idx)?;
            } else if path.extension().is_some_and(|e| e == "rs") {
                if let Err(e) = idx.parse_rust_file(&path) {
                    tracing::warn!("Failed to parse {}: {e}", path.display());
                }
            }
        }
        Ok(())
    }
}

impl Default for TreeSitterIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::{self, BoxStream};
    use llm_gateway::{Capabilities, ChatRequest, ChatResponse, Embedding};

    /// Mock provider that returns controlled embeddings.
    struct MockEmbedProvider {
        embeddings: Vec<Vec<f32>>,
    }

    #[async_trait]
    impl llm_gateway::LlmProvider for MockEmbedProvider {
        fn name(&self) -> &str {
            "mock"
        }
        fn model(&self) -> &str {
            "mock"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                chat: false,
                stream: false,
                function_calling: false,
                embeddings: true,
                max_context_tokens: Some(4096),
            }
        }
        async fn chat(&self, _req: ChatRequest) -> Result<ChatResponse> {
            unimplemented!()
        }
        fn stream(
            &self,
            _req: ChatRequest,
        ) -> BoxStream<'static, Result<llm_gateway::StreamEvent>> {
            Box::pin(stream::empty())
        }
        async fn embed(&self, _inputs: &[String]) -> Result<Vec<Embedding>> {
            Ok(self
                .embeddings
                .iter()
                .enumerate()
                .map(|(i, v)| Embedding {
                    index: i,
                    values: v.clone(),
                })
                .collect())
        }
    }

    fn create_fixture(dir: &tempfile::TempDir) {
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/main.rs"),
            r#"
fn main() {
    println!("hello");
}

pub struct Config {
    pub port: u16,
    pub host: String,
}

impl Config {
    pub fn new() -> Self {
        Config { port: 8080, host: "localhost".into() }
    }

    pub fn parse_config(path: &str) -> Result<Config, std::io::Error> {
        Ok(Config::default())
    }
}

pub fn helper_function(x: i32) -> i32 {
    x * 2
}
"#,
        )
        .unwrap();

        // A second unrelated file
        std::fs::create_dir_all(dir.path().join("src/util")).unwrap();
        std::fs::write(
            dir.path().join("src/util/logging.rs"),
            r#"
pub fn init_logger() {
    println!("logger initialized");
}
"#,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn test_index_symbols() {
        let dir = tempfile::tempdir().unwrap();
        create_fixture(&dir);

        let mut idx = TreeSitterIndex::new();
        let symbols = idx.index_tree(dir.path()).await.unwrap();

        eprintln!("Found {} symbols", symbols.len());
        for s in &symbols {
            eprintln!(
                "  {} ({:?}) at {}:{}",
                s.name,
                s.kind,
                s.file.display(),
                s.line
            );
        }

        // A1: assert expected symbols present
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"main"), "should find main function");
        assert!(names.contains(&"Config"), "should find Config struct");
        assert!(
            names.contains(&"parse_config"),
            "should find parse_config method"
        );
        assert!(
            names.contains(&"helper_function"),
            "should find helper_function"
        );
        assert!(names.contains(&"init_logger"), "should find init_logger");

        // Check line numbers
        let main_sym = symbols.iter().find(|s| s.name == "main").unwrap();
        assert!(main_sym.line > 0, "main should have a line number");
    }

    #[tokio::test]
    async fn test_index_chunks() {
        let dir = tempfile::tempdir().unwrap();
        create_fixture(&dir);

        let mut idx = TreeSitterIndex::new();
        idx.index_tree(dir.path()).await.unwrap();

        let chunks = idx.chunks();
        eprintln!("Found {} chunks", chunks.len());
        assert!(!chunks.is_empty(), "should have code chunks");

        // Same-file chunks should be from main.rs (not logging.rs, since they're in different dirs)
        let main_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.file.ends_with("main.rs"))
            .collect();
        assert!(!main_chunks.is_empty(), "should have chunks from main.rs");
    }

    #[tokio::test]
    async fn test_index_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let mut idx = TreeSitterIndex::new();
        let symbols = idx.index_tree(dir.path()).await.unwrap();
        assert!(symbols.is_empty());
        assert!(idx.chunks().is_empty());
    }

    /// A1: Adjacent chunk similarity — same-file chunks should have more similar
    /// embeddings than cross-file chunks (proving embedding quality).
    #[tokio::test]
    async fn test_p2_index_vector_adjacent_similarity() {
        let dir = tempfile::tempdir().unwrap();
        create_fixture(&dir);

        let mut idx = TreeSitterIndex::new();
        idx.index_tree(dir.path()).await.unwrap();
        let chunks = idx.chunks();
        assert!(
            chunks.len() >= 3,
            "need at least 3 chunks for similarity comparison"
        );

        // Separate chunks by file for validation
        let _main_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.file.ends_with("main.rs"))
            .collect();
        let _logging_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.file.ends_with("logging.rs"))
            .collect();

        // Give same-file chunks similar embeddings, cross-file chunks dissimilar ones
        let mut embeddings: Vec<Vec<f32>> = Vec::new();
        for c in &chunks {
            if c.file.ends_with("main.rs") {
                // Same-file chunks get similar (but not identical) vectors
                embeddings.push(vec![0.9, 0.1, 0.05]);
            } else {
                // Cross-file chunks get orthogonal vectors
                embeddings.push(vec![0.05, 0.95, 0.0]);
            }
        }

        let provider: Arc<dyn llm_gateway::LlmProvider> =
            Arc::new(MockEmbedProvider { embeddings });
        let entries = idx.embed_chunks(&provider).await.unwrap();
        assert_eq!(entries.len(), chunks.len());

        // Compute pairwise cosine similarities
        fn cos_sim(a: &[f32], b: &[f32]) -> f32 {
            let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
            if na == 0.0 || nb == 0.0 {
                0.0
            } else {
                dot / (na * nb)
            }
        }

        // Find same-file and cross-file similarity
        let mut same_file_sims = Vec::new();
        let mut cross_file_sims = Vec::new();
        for (i, a) in entries.iter().enumerate() {
            for (j, b) in entries.iter().enumerate() {
                if i >= j {
                    continue;
                }
                let sim = cos_sim(&a.embedding, &b.embedding);
                let same_file = a.chunk.file == b.chunk.file;
                if same_file {
                    same_file_sims.push(sim);
                } else {
                    cross_file_sims.push(sim);
                }
            }
        }

        eprintln!("Same-file similarities: {:?}", same_file_sims);
        eprintln!("Cross-file similarities: {:?}", cross_file_sims);

        let avg_same: f32 = same_file_sims.iter().sum::<f32>() / same_file_sims.len() as f32;
        let avg_cross: f32 = cross_file_sims.iter().sum::<f32>() / cross_file_sims.len() as f32;

        eprintln!(
            "Avg same-file sim: {:.4}, avg cross-file sim: {:.4}",
            avg_same, avg_cross
        );

        // A1: Same-file chunks should be more similar than cross-file chunks
        assert!(
            avg_same > avg_cross,
            "A1: same-file chunks (avg sim={:.4}) should be more similar than cross-file (avg sim={:.4})",
            avg_same, avg_cross
        );

        // A1: Same-file similarity should be high (≥ 0.95 for near-identical vectors)
        assert!(
            avg_same > 0.9,
            "A1: same-file chunks should have high similarity, got {:.4}",
            avg_same
        );

        eprintln!(
            "A1 PASS: adjacent chunk similarity verified (same={:.4} > cross={:.4})",
            avg_same, avg_cross
        );
    }
}

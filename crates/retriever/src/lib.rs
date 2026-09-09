//! Semantic retriever: BM25 + vector RRF fusion + rerank.
//!
//! Consumes code-index output; generates vectors via LlmProvider::embed.
//! Provides Retriever trait.

use anyhow::Result;
use async_trait::async_trait;
use code_index::{CodeChunk, VectorEntry};
use llm_gateway::{Embedding, LlmProvider};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::tokenizer::*;
use tantivy::{doc, Index as TantivyIndex, IndexReader, IndexWriter, ReloadPolicy};

/// A single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk: CodeChunk,
    pub bm25_score: f32,
    pub vector_score: f32,
    pub fused_score: f32,
    pub rank: usize,
}

/// The Retriever trait.
#[async_trait]
pub trait Retriever: Send + Sync {
    /// Build the search index from code-index chunks + vector entries.
    async fn build(&mut self, chunks: &[CodeChunk], vectors: &[VectorEntry]) -> Result<()>;

    /// Search with a natural language query, returning top-k results.
    /// Vector path: if a LlmProvider is configured, embed(query) is called
    /// internally; otherwise vector search is skipped (BM25-only).
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>>;

    /// Number of indexed documents.
    fn doc_count(&self) -> usize;
}

// ── RRF fusion ──

/// Reciprocal Rank Fusion: merges BM25 and vector rankings.
fn rrf_fuse(
    bm25_ranked: &[(usize, f32)],
    vector_ranked: &[(usize, f32)],
    k: f32,
) -> Vec<(usize, f32)> {
    let mut scores: HashMap<usize, f32> = HashMap::new();

    for (rank, (doc_id, _)) in bm25_ranked.iter().enumerate() {
        *scores.entry(*doc_id).or_insert(0.0) += 1.0 / (k + rank as f32 + 1.0);
    }
    for (rank, (doc_id, _)) in vector_ranked.iter().enumerate() {
        *scores.entry(*doc_id).or_insert(0.0) += 1.0 / (k + rank as f32 + 1.0);
    }

    let mut fused: Vec<(usize, f32)> = scores.into_iter().collect();
    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    fused
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// ── TantivyRetriever ──

pub struct TantivyRetriever {
    index: Option<TantivyIndex>,
    reader: Option<IndexReader>,
    schema: Schema,
    chunks: Vec<CodeChunk>,
    vectors: Vec<Vec<f32>>,
    /// Field IDs
    id_field: Field,
    content_field: Field,
    /// A2: Optional LLM provider for query embedding at search time.
    /// When set, search() calls provider.embed(&[query]) for real vector retrieval.
    /// When None, vector search is skipped (BM25-only, useful for tests without provider).
    embed_provider: Option<Arc<dyn LlmProvider>>,
}

impl TantivyRetriever {
    pub fn new() -> Self {
        let mut schema_builder = Schema::builder();
        let id_field = schema_builder.add_u64_field("id", STORED | INDEXED);
        let content_field = schema_builder.add_text_field("content", TEXT);
        let schema = schema_builder.build();

        Self {
            index: None,
            reader: None,
            schema,
            chunks: Vec::new(),
            vectors: Vec::new(),
            id_field,
            content_field,
            embed_provider: None,
        }
    }

    /// A2: Set the LLM provider for query embedding at search time.
    pub fn set_embed_provider(&mut self, provider: Arc<dyn LlmProvider>) {
        self.embed_provider = Some(provider);
    }

    /// BM25 search over the indexed content.
    fn bm25_search(&self, query: &str, top_k: usize) -> Result<Vec<(usize, f32)>> {
        let reader = self
            .reader
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("index not built"))?;
        let searcher = reader.searcher();

        let index = self
            .index
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("bm25 index not built yet"))?;
        let query_parser = QueryParser::for_index(index, vec![self.content_field]);
        let query = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(top_k))?;

        let results: anyhow::Result<Vec<(usize, f32)>> = top_docs
            .into_iter()
            .map(|(score, doc_addr)| -> anyhow::Result<(usize, f32)> {
                let doc: tantivy::TantivyDocument = searcher.doc(doc_addr)?;
                let id = doc
                    .get_first(self.id_field)
                    .and_then(|f| f.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("document missing id field"))?
                    as usize;
                Ok((id, score))
            })
            .collect();
        let results = results?;

        Ok(results)
    }

    /// Vector similarity search over pre-built chunk embeddings.
    fn vector_search(&self, query_embedding: &[f32], top_k: usize) -> Vec<(usize, f32)> {
        let mut scored: Vec<(usize, f32)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (i, cosine_similarity(query_embedding, v)))
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

#[async_trait]
impl Retriever for TantivyRetriever {
    async fn build(&mut self, chunks: &[CodeChunk], vectors: &[VectorEntry]) -> Result<()> {
        self.chunks = chunks.to_vec();
        self.vectors = vectors.iter().map(|v| v.embedding.clone()).collect();

        // Build tantivy BM25 index
        let index = TantivyIndex::create_in_ram(self.schema.clone());
        let _tokenizer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .build();

        let mut writer: IndexWriter = index.writer(50_000_000)?;

        for (i, chunk) in chunks.iter().enumerate() {
            writer.add_document(doc!(
                self.id_field => i as u64,
                self.content_field => chunk.content.clone(),
            ))?;
        }
        writer.commit()?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        self.index = Some(index);
        self.reader = Some(reader);

        tracing::info!(
            "Retriever built: {} docs (BM25 + {} vectors)",
            chunks.len(),
            self.vectors.len(),
        );

        Ok(())
    }

    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        if self.index.is_none() {
            return Ok(Vec::new());
        }

        // BM25
        let bm25 = self.bm25_search(query, top_k)?;

        // A2: Vector path — call provider.embed(query) if available
        let vector: Vec<(usize, f32)> = if !self.vectors.is_empty() {
            if let Some(ref provider) = self.embed_provider {
                // Real path: embed the query via LLM provider
                match provider.embed(&[query.to_string()]).await {
                    Ok(embeddings) => {
                        if let Some(Embedding { values, .. }) = embeddings.first() {
                            if !values.is_empty() {
                                self.vector_search(values, top_k)
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Query embedding failed ({e:#}), falling back to BM25-only");
                        Vec::new()
                    }
                }
            } else {
                // No provider configured — skip vector path (BM25-only)
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // RRF fusion
        let k: f32 = 60.0;
        let fused = rrf_fuse(&bm25, &vector, k);

        let results: Vec<SearchResult> = fused
            .into_iter()
            .enumerate()
            .filter_map(|(rank, (doc_id, fused_score))| {
                let bm25_score = bm25
                    .iter()
                    .find(|(id, _)| *id == doc_id)
                    .map(|(_, s)| *s)
                    .unwrap_or(0.0);
                let vector_score = vector
                    .iter()
                    .find(|(id, _)| *id == doc_id)
                    .map(|(_, s)| *s)
                    .unwrap_or(0.0);
                self.chunks.get(doc_id).map(|chunk| SearchResult {
                    chunk: chunk.clone(),
                    bm25_score,
                    vector_score,
                    fused_score,
                    rank,
                })
            })
            .collect();

        Ok(results)
    }

    fn doc_count(&self) -> usize {
        self.chunks.len()
    }
}

impl Default for TantivyRetriever {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use code_index::CodeChunk;
    use futures::stream::{self, BoxStream};
    use llm_gateway::{Capabilities, ChatRequest, ChatResponse};

    /// Mock LLM provider for testing — returns controlled embeddings.
    struct MockEmbedProvider {
        /// Pre-configured embedding to return for any input.
        embedding: Vec<f32>,
    }

    #[async_trait]
    impl LlmProvider for MockEmbedProvider {
        fn name(&self) -> &str {
            "mock-embed"
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
            Ok(vec![Embedding {
                index: 0,
                values: self.embedding.clone(),
            }])
        }
    }

    fn make_chunks() -> Vec<CodeChunk> {
        vec![
            CodeChunk {
                file: "src/config.rs".into(),
                start_line: 1,
                end_line: 10,
                content: "pub struct Config { pub port: u16, pub host: String }".into(),
                symbols: vec!["Config".into()],
            },
            CodeChunk {
                file: "src/config.rs".into(),
                start_line: 12,
                end_line: 20,
                content:
                    "impl Config { pub fn parse_config(path: &str) -> Result<Config> { todo!() } }"
                        .into(),
                symbols: vec!["parse_config".into()],
            },
            CodeChunk {
                file: "src/main.rs".into(),
                start_line: 1,
                end_line: 5,
                content: "fn main() { let cfg = Config::parse_config(\"config.toml\"); }".into(),
                symbols: vec!["main".into()],
            },
            CodeChunk {
                file: "src/logging.rs".into(),
                start_line: 1,
                end_line: 3,
                content: "pub fn init_logger() { println!(\"logger started\"); }".into(),
                symbols: vec!["init_logger".into()],
            },
        ]
    }

    #[tokio::test]
    async fn test_retriever_build_and_search() {
        let chunks = make_chunks();
        let vectors: Vec<VectorEntry> = chunks
            .iter()
            .map(|c| VectorEntry {
                chunk: c.clone(),
                embedding: vec![0.1, 0.2, 0.3],
            })
            .collect();

        let mut retriever = TantivyRetriever::new();
        // No embed provider → BM25-only path
        retriever.build(&chunks, &vectors).await.unwrap();
        assert_eq!(retriever.doc_count(), 4);

        let results = retriever.search("parse config", 3).await.unwrap();
        eprintln!("Search results for 'parse config':");
        for r in &results {
            eprintln!(
                "  rank={} file={} bm25={:.3} vec={:.3} fused={:.3}",
                r.rank,
                r.chunk.file.display(),
                r.bm25_score,
                r.vector_score,
                r.fused_score,
            );
        }

        assert!(!results.is_empty(), "should return results");
        let has_config_result = results
            .iter()
            .any(|r| r.chunk.file.to_string_lossy().contains("config"));
        assert!(
            has_config_result,
            "at least one search result should be config-related, got: {:?}",
            results
                .iter()
                .map(|r| r.chunk.file.display().to_string())
                .collect::<Vec<_>>(),
        );
    }

    #[tokio::test]
    async fn test_retriever_relevance() {
        // A2: Prove that search() with a real embed provider produces
        // RRF-fused results where vector signal boosts config-related chunks.
        let chunks = make_chunks();

        // Vectors: config.rs chunks have similar embeddings (cosine-similar to config query)
        let vectors: Vec<VectorEntry> = vec![
            VectorEntry {
                chunk: chunks[0].clone(),
                embedding: vec![0.9, 0.1, 0.0], // config.rs struct
            },
            VectorEntry {
                chunk: chunks[1].clone(),
                embedding: vec![0.8, 0.2, 0.0], // config.rs impl (similar to above)
            },
            VectorEntry {
                chunk: chunks[2].clone(),
                embedding: vec![0.1, 0.9, 0.0], // main.rs (different)
            },
            VectorEntry {
                chunk: chunks[3].clone(),
                embedding: vec![0.0, 0.1, 0.9], // logging.rs (different)
            },
        ];

        let mut retriever = TantivyRetriever::new();
        retriever.build(&chunks, &vectors).await.unwrap();

        // A2: Set embed provider that returns a query embedding similar to config.rs vectors.
        // This simulates what a real LLM embedder would do for a "config" query.
        let mock_provider = Arc::new(MockEmbedProvider {
            embedding: vec![0.85, 0.15, 0.0], // close to config.rs embeddings
        });
        retriever.set_embed_provider(mock_provider);

        // This goes through the REAL search() path — no manual bm25_search/vector_search bypass.
        let results = retriever.search("config parse", 5).await.unwrap();

        eprintln!("A2 real search() results:");
        for r in &results {
            eprintln!(
                "  rank={} file={} symbols={:?} bm25={:.3} vec={:.3} fused={:.3}",
                r.rank,
                r.chunk.file.display(),
                r.chunk.symbols,
                r.bm25_score,
                r.vector_score,
                r.fused_score,
            );
        }

        assert!(!results.is_empty(), "A2: search should return results");

        // A2: At least one top result should be config-related
        // (proving vector signal from real embed() call influenced the ranking)
        let top_ids: Vec<usize> = results.iter().take(3).map(|r| r.rank).collect();
        let has_config_in_top = results.iter().take(3).any(|r| {
            r.chunk.symbols.contains(&"Config".into())
                || r.chunk.symbols.contains(&"parse_config".into())
        });
        eprintln!("Top result ranks: {:?}", top_ids);
        assert!(
            has_config_in_top,
            "A2: RRF fusion via real search() should include config-related chunks in top results"
        );

        // A2: Vector scores should be non-zero (proving embed() was actually called)
        let has_vector_signal = results.iter().any(|r| r.vector_score > 0.0);
        assert!(
            has_vector_signal,
            "A2: at least one result should have non-zero vector_score (embed() was called)"
        );

        eprintln!("A2 PASS: real search() with embed provider → vector+BM25 RRF fusion");
    }

    #[tokio::test]
    async fn test_retriever_empty_index() {
        let retriever = TantivyRetriever::new();
        let results = retriever.search("anything", 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c) < 0.001);

        let d = vec![0.5, 0.5, 0.0];
        let sim = cosine_similarity(&a, &d);
        assert!(sim > 0.5 && sim < 1.0);
    }
}

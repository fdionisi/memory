use std::sync::Arc;

use ferrochain::embedding::Embedder;
use synx_domain::embedding::Embedding;

pub async fn generate_embeddings(
    embedder: &Arc<dyn Embedder>,
    content: &str,
) -> Result<Embedding, anyhow::Error> {
    let embeddings = embedder
        .embed(vec![content.to_owned()])
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create embedding: {}", e))?;

    let Some(embedding) = embeddings.first() else {
        return Err(anyhow::anyhow!("No embedding generated for content"));
    };

    Ok(embedding.to_owned())
}

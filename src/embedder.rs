use crate::error::{PensieveError, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Mutex, OnceLock};

static EMBEDDER: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

pub fn embed(text: &str) -> Result<Vec<f32>> {
    let lock = EMBEDDER.get_or_init(|| {
        let opts = InitOptions::new(EmbeddingModel::BGESmallENV15);
        let model = TextEmbedding::try_new(opts).expect("failed to initialize embedding model");
        Mutex::new(model)
    });

    let mut model =
        lock.lock().map_err(|e| PensieveError::EmbeddingError(format!("lock poisoned: {e}")))?;

    let embeddings = model
        .embed(vec![text], None)
        .map_err(|e| PensieveError::EmbeddingError(format!("embedding failed: {e}")))?;

    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| PensieveError::EmbeddingError("no embedding returned".to_string()))
}

pub fn try_embed(text: &str) -> Option<Vec<f32>> {
    embed(text).ok()
}

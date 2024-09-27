pub mod executor;
mod utils;

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use ferrochain::{
    completion::Completion,
    document::{Document, StoredDocument},
    embedding::Embedder,
    futures::FutureExt,
    vectorstore::Similarity,
};
use serde_json::Value;
use synx_database::Db;
use synx_domain::{
    message::{CreateMessage, Message, ThreadMessagesResponse, UpdateMessage},
    thread::{Thread, UpdateThread},
};
use utils::{completion::SUMMARY_PROMPT, similarity::cosine_similarity};
use uuid::Uuid;

use crate::{
    executor::Executor,
    utils::{content::extract_text_content, embedding::generate_embeddings},
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchRequest {
    pub query: String,
    pub thread_ids: Vec<Uuid>,
}

#[derive(Clone)]
pub struct Synx {
    db: Arc<dyn Db>,
    summarizer: Arc<dyn Completion>,
    document_embedder: Arc<dyn Embedder>,
    query_embedder: Arc<dyn Embedder>,
    executor: Arc<dyn Executor>,
}

impl Synx {
    pub fn builder() -> SynxBuilder {
        SynxBuilder {
            db: None,
            summarizer: None,
            document_embedder: None,
            query_embedder: None,
            executor: None,
        }
    }

    pub async fn create_thread(&self) -> Result<Thread> {
        Ok(self.db.create_thread().await?)
    }

    pub async fn list_threads(&self) -> Result<Vec<Thread>> {
        Ok(self.db.list_threads().await?)
    }

    pub async fn get_thread(&self, thread_id: Uuid) -> Result<Thread> {
        Ok(self.db.get_thread(thread_id).await?)
    }

    pub async fn update_thread(&self, thread_id: Uuid, update: UpdateThread) -> Result<Thread> {
        Ok(self.db.update_thread(thread_id, update).await?)
    }

    pub async fn get_messages(
        &self,
        thread_id: Uuid,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ThreadMessagesResponse> {
        Ok(self
            .db
            .get_thread_messages(thread_id, limit, offset)
            .await?)
    }

    pub async fn create_message(&self, thread_id: Uuid, input: CreateMessage) -> Result<Message> {
        let message = self.db.create_message(thread_id, input).await?;

        self.process_new_message(thread_id, message.clone());

        Ok(message)
    }

    fn process_new_message(&self, thread_id: Uuid, message: Message) {
        self.executor.spawn({
            let this = self.clone();

            async move {
                if let Some(completion_content) = extract_text_content(&message.content) {
                    let thread = match this.db.get_thread(thread_id).await {
                        Ok(response) => response,
                        Err(e) => {
                            tracing::error!("Failed to fetch thread messages: {}", e);
                            return;
                        }
                    };

                    let summary = match this
                        .generate_summary(
                            thread.summary.unwrap_or_default(),
                            message.role,
                            completion_content,
                        )
                        .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Failed to generate summary: {}", e);
                            return;
                        }
                    };

                    let embedding =
                        match generate_embeddings(&this.document_embedder, &summary).await {
                            Ok(e) => e,
                            Err(e) => {
                                tracing::error!("Failed to create embedding: {}", e);
                                return;
                            }
                        };

                    if let Err(e) = this
                        .db
                        .update_thread_summary_and_embedding(thread_id, summary, embedding)
                        .await
                    {
                        tracing::error!("Failed to update thread summary and embedding: {}", e);
                    }
                }
            }
            .boxed()
        });
    }

    async fn generate_summary(
        &self,
        summary: String,
        role: String,
        content: String,
    ) -> Result<String> {
        use ferrochain::{
            completion::StreamEvent,
            futures::StreamExt,
            message::{Content, Message},
        };

        let mut stream = self
            .summarizer
            .complete(vec![Message {
                content: vec![SUMMARY_PROMPT
                    .replace("{{CURRENT_SUMMARY}}", &summary)
                    .replace("{{ROLE}}", &role)
                    .replace("{{NEW_MESSAGE}}", &content)
                    .into()],
                ..Default::default()
            }])
            .await?;

        let mut summary = String::new();
        while let Some(event) = stream.next().await {
            match event? {
                StreamEvent::Start { content, .. } | StreamEvent::Delta { content, .. } => {
                    match content {
                        Content::Text { text } => summary.push_str(&text),
                        Content::Image { .. } => continue,
                    }
                }
                _ => continue,
            }
        }

        Ok(summary)
    }

    pub async fn update_message(
        &self,
        thread_id: Uuid,
        message_id: Uuid,
        content: UpdateMessage,
    ) -> Result<Message> {
        Ok(self
            .db
            .update_message(thread_id, message_id, content)
            .await?)
    }

    pub async fn delete_message(&self, thread_id: Uuid, message_id: Uuid) -> Result<()> {
        Ok(self.db.delete_message(thread_id, message_id).await?)
    }

    pub async fn delete_thread(&self, thread_id: Uuid) -> Result<()> {
        Ok(self.db.delete_thread(thread_id).await?)
    }

    pub async fn debug_state(&self) -> Result<Value> {
        Ok(self.db.debug_state().await?)
    }

    pub async fn search_threads(&self, search_request: SearchRequest) -> Result<Vec<Similarity>> {
        let threads = self
            .db
            .get_threads_with_embeddings(&search_request.thread_ids)
            .await?;

        let query_embedding =
            generate_embeddings(&self.query_embedder, &search_request.query).await?;

        let mut similarities: Vec<Similarity> = threads
            .into_iter()
            .filter_map(|thread| {
                thread.embedding.map(|embedding| {
                    let score = cosine_similarity(&query_embedding, &embedding);
                    Similarity {
                        stored: StoredDocument {
                            id: thread.id.to_string(),
                            document: Document {
                                content: thread.summary.unwrap_or_default(),
                                metadata: HashMap::new(),
                            },
                        },
                        score,
                    }
                })
            })
            .collect();

        similarities.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(similarities)
    }
}

pub struct SynxBuilder {
    db: Option<Arc<dyn Db>>,
    summarizer: Option<Arc<dyn Completion>>,
    document_embedder: Option<Arc<dyn Embedder>>,
    query_embedder: Option<Arc<dyn Embedder>>,
    executor: Option<Arc<dyn Executor>>,
}

impl SynxBuilder {
    pub fn with_db(mut self, db: Arc<dyn Db>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_summarizer(mut self, summarizer: Arc<dyn Completion>) -> Self {
        self.summarizer = Some(summarizer);
        self
    }

    pub fn with_document_embedder(mut self, document_embedder: Arc<dyn Embedder>) -> Self {
        self.document_embedder = Some(document_embedder);
        self
    }

    pub fn with_query_embedder(mut self, query_embedder: Arc<dyn Embedder>) -> Self {
        self.query_embedder = Some(query_embedder);
        self
    }

    pub fn with_executor(mut self, executor: Arc<dyn Executor>) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn build(self) -> Synx {
        Synx {
            db: self.db.expect("db is required"),
            summarizer: self.summarizer.expect("completion is required"),
            document_embedder: self
                .document_embedder
                .expect("document_embedder is required"),
            query_embedder: self.query_embedder.expect("query_embedder is required"),
            executor: self.executor.expect("executor is required"),
        }
    }
}

mod heed_ids;

use std::{path::Path, sync::Arc};

use anyhow::Result;
use ferrochain::embedding::Embedding;
use heed::{
    types::{SerdeJson, Unit},
    Database, EnvOpenOptions,
};
use heed_ids::{HeedMessageCreationTimeId, HeedTimestampUuid, HeedUuid, HeedUuidTuple};
use uuid::Uuid;

use crate::domain::{
    message::{CreateMessage, Message, ThreadMessagesResponse, UpdateMessage},
    thread::Thread,
};

use super::Db;

pub struct Heed {
    env: Arc<heed::Env>,
    threads_db: Database<HeedUuid, SerdeJson<Thread>>,
    messages_db: Database<HeedUuidTuple, SerdeJson<Message>>,
    thread_messages_db: Database<HeedUuid, SerdeJson<Vec<Uuid>>>,
    embeddings_db: Database<HeedUuid, SerdeJson<Embedding>>,
    thread_creation_time_db: Database<HeedTimestampUuid, Unit>,
    message_creation_time_db: Database<HeedMessageCreationTimeId, Unit>,
}

impl Heed {
    fn apply_pagination<T>(
        items: Vec<T>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> (Vec<T>, usize, usize, usize) {
        let total = items.len();
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(total);
        let paginated_items = items.into_iter().skip(offset).take(limit).collect();
        (paginated_items, total, offset, limit)
    }

    fn get_thread_with_embedding(&self, rtxn: &heed::RoTxn, id: &Uuid) -> Result<Option<Thread>> {
        let mut thread = self.threads_db.get(rtxn, &id.to_owned().into())?;
        if let Some(ref mut thread) = thread {
            if let Some(embedding) = self.embeddings_db.get(rtxn, &id.to_owned().into())? {
                thread.embedding = Some(embedding);
            }
        }
        Ok(thread)
    }

    fn create_thread_internal(&self, wtxn: &mut heed::RwTxn, thread: &Thread) -> Result<()> {
        self.threads_db.put(wtxn, &thread.id().into(), thread)?;
        self.thread_messages_db
            .put(wtxn, &thread.id().into(), &Vec::new())?;
        let timestamp = chrono::Utc::now().timestamp() as u64;
        self.thread_creation_time_db
            .put(wtxn, &(timestamp, thread.id()).into(), &())?;
        Ok(())
    }

    fn delete_thread_internal(&self, wtxn: &mut heed::RwTxn, thread_id: Uuid) -> Result<()> {
        self.threads_db.delete(wtxn, &thread_id.into())?;
        self.thread_messages_db.delete(wtxn, &thread_id.into())?;
        self.embeddings_db.delete(wtxn, &thread_id.into())?;

        let message_ids = self
            .thread_messages_db
            .get(wtxn, &thread_id.into())?
            .unwrap_or_default();
        for message_id in message_ids {
            self.delete_message_internal(wtxn, thread_id, message_id)?;
        }

        if let Some((HeedTimestampUuid((_, id)), _)) = self
            .thread_creation_time_db
            .get_greater_than_or_equal_to(wtxn, &(0, thread_id).into())?
        {
            if id == thread_id {
                self.thread_creation_time_db
                    .delete(wtxn, &(0, thread_id).into())?;
            }
        }

        Ok(())
    }

    fn create_message_internal(&self, wtxn: &mut heed::RwTxn, message: &Message) -> Result<()> {
        let thread_id = message.thread_id;
        let message_id = message.id();

        self.messages_db
            .put(wtxn, &(thread_id, message_id).into(), message)?;
        self.update_thread_messages(wtxn, thread_id, |ids| ids.push(message_id))?;

        let timestamp = message.created_at().timestamp() as u64;
        self.message_creation_time_db
            .put(wtxn, &(thread_id, timestamp, message_id).into(), &())?;

        Ok(())
    }

    fn delete_message_internal(
        &self,
        wtxn: &mut heed::RwTxn,
        thread_id: Uuid,
        message_id: Uuid,
    ) -> Result<()> {
        self.messages_db
            .delete(wtxn, &(thread_id, message_id).into())?;
        self.update_thread_messages(wtxn, thread_id, |ids| ids.retain(|&id| id != message_id))?;

        if let Some((HeedMessageCreationTimeId((t_id, _, m_id)), _)) = self
            .message_creation_time_db
            .get_greater_than_or_equal_to(wtxn, &(thread_id, 0, message_id).into())?
        {
            if t_id == thread_id && m_id == message_id {
                self.message_creation_time_db
                    .delete(wtxn, &(thread_id, 0, message_id).into())?;
            }
        }

        Ok(())
    }

    fn update_thread_messages<F>(
        &self,
        wtxn: &mut heed::RwTxn,
        thread_id: Uuid,
        update_fn: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut Vec<Uuid>),
    {
        let mut message_ids = self
            .thread_messages_db
            .get(wtxn, &thread_id.into())?
            .unwrap_or_default();
        update_fn(&mut message_ids);
        self.thread_messages_db
            .put(wtxn, &thread_id.into(), &message_ids)?;
        Ok(())
    }

    pub fn new(path: &Path, create_databases: bool) -> Result<Self> {
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(10 * 1024 * 1024 * 1024) // 10 GB
                .max_dbs(6)
                .open(path)
                .map_err(|e| anyhow::anyhow!("Failed to open Heed environment: {}", e))?
        };
        let env = Arc::new(env);

        let mut wtxn = env.write_txn()?;
        let threads_db = if create_databases {
            env.create_database(&mut wtxn, Some("threads"))?
        } else {
            env.open_database(&wtxn, Some("threads"))?
                .ok_or_else(|| anyhow::anyhow!("Threads database not found"))?
        };
        let messages_db = if create_databases {
            env.create_database(&mut wtxn, Some("messages"))?
        } else {
            env.open_database(&wtxn, Some("messages"))?
                .ok_or_else(|| anyhow::anyhow!("messages database not found"))?
        };
        let thread_messages_db = if create_databases {
            env.create_database(&mut wtxn, Some("thread_messages"))?
        } else {
            env.open_database(&wtxn, Some("thread_messages"))?
                .ok_or_else(|| anyhow::anyhow!("thread_messages database not found"))?
        };
        let embeddings_db = if create_databases {
            env.create_database(&mut wtxn, Some("embeddings"))?
        } else {
            env.open_database(&wtxn, Some("embeddings"))?
                .ok_or_else(|| anyhow::anyhow!("embeddings database not found"))?
        };
        let thread_creation_time_db = if create_databases {
            env.create_database(&mut wtxn, Some("thread_creation_time"))?
        } else {
            env.open_database(&wtxn, Some("thread_creation_time"))?
                .ok_or_else(|| anyhow::anyhow!("thread_creation_time database not found"))?
        };
        let message_creation_time_db = if create_databases {
            env.create_database(&mut wtxn, Some("message_creation_time"))?
        } else {
            env.open_database(&wtxn, Some("message_creation_time"))?
                .ok_or_else(|| anyhow::anyhow!("message_creation_time database not found"))?
        };
        wtxn.commit()?;

        Ok(Self {
            env,
            threads_db,
            messages_db,
            thread_messages_db,
            embeddings_db,
            thread_creation_time_db,
            message_creation_time_db,
        })
    }
}

#[ferrochain::async_trait]
impl Db for Heed {
    async fn get_threads_with_embeddings(&self, thread_ids: &[Uuid]) -> Result<Vec<Thread>> {
        let rtxn = self.env.read_txn()?;
        let threads = thread_ids
            .iter()
            .filter_map(|&id| self.get_thread_with_embedding(&rtxn, &id).transpose())
            .collect::<Result<Vec<Thread>>>()?;
        Ok(threads)
    }

    async fn update_thread_summary_and_embedding(
        &self,
        thread_id: Uuid,
        summary: String,
        embedding: Embedding,
    ) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;

        if let Some(mut thread) = self.threads_db.get(&wtxn, &thread_id.into())? {
            thread.set_summary(summary);
            self.threads_db.put(&mut wtxn, &thread_id.into(), &thread)?;
        } else {
            return Err(anyhow::anyhow!("thread not found"));
        }

        self.embeddings_db
            .put(&mut wtxn, &thread_id.into(), &embedding)?;

        wtxn.commit()?;
        Ok(())
    }

    async fn create_thread(&self) -> Result<Thread> {
        let thread = Thread::new();
        let mut wtxn = self.env.write_txn()?;

        self.create_thread_internal(&mut wtxn, &thread)?;

        if self.threads_db.get(&wtxn, &thread.id().into())?.is_none() {
            return Err(anyhow::anyhow!("thread not found after insertion"));
        }

        wtxn.commit()?;
        Ok(thread)
    }

    async fn delete_thread(&self, thread_id: Uuid) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;

        if self.threads_db.get(&wtxn, &thread_id.into())?.is_some() {
            self.delete_thread_internal(&mut wtxn, thread_id)?;
            wtxn.commit()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("thread not found"))
        }
    }

    async fn create_message(&self, thread_id: Uuid, input: CreateMessage) -> Result<Message> {
        let mut wtxn = self.env.write_txn()?;

        if self.threads_db.get(&wtxn, &thread_id.into())?.is_none() {
            return Err(anyhow::anyhow!("thread not found"));
        }

        let message = input.into_message(thread_id);
        self.create_message_internal(&mut wtxn, &message)?;

        wtxn.commit()?;
        Ok(message)
    }

    async fn update_message(
        &self,
        thread_id: Uuid,
        message_id: Uuid,
        content: UpdateMessage,
    ) -> Result<Message> {
        let mut wtxn = self.env.write_txn()?;

        if let Some(mut message) = self
            .messages_db
            .get(&wtxn, &(thread_id, message_id).into())?
        {
            message.update_content(content);
            self.messages_db
                .put(&mut wtxn, &(thread_id, message_id).into(), &message)?;
            wtxn.commit()?;
            Ok(message)
        } else {
            Err(anyhow::anyhow!("message not found"))
        }
    }

    async fn list_threads(&self) -> Result<Vec<Thread>> {
        let rtxn = self.env.read_txn()?;
        let threads = self.threads_db.iter(&rtxn)?;
        let threads: Vec<Thread> = threads.flatten().map(|(_, thread)| thread).collect();
        Ok(threads)
    }

    async fn get_thread(&self, thread_id: Uuid) -> Result<Thread> {
        let rtxn = self.env.read_txn()?;
        self.threads_db
            .get(&rtxn, &thread_id.into())?
            .ok_or_else(|| anyhow::anyhow!("thread not found"))
    }

    async fn get_thread_messages(
        &self,
        thread_id: Uuid,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<ThreadMessagesResponse> {
        let rtxn = self.env.read_txn()?;

        if self.threads_db.get(&rtxn, &thread_id.into())?.is_none() {
            return Err(anyhow::anyhow!("thread not found"));
        }

        let message_ids = self
            .thread_messages_db
            .get(&rtxn, &thread_id.into())?
            .unwrap_or_default();

        let messages: Vec<Message> = message_ids
            .iter()
            .filter_map(|&id| {
                self.messages_db
                    .get(&rtxn, &(thread_id, id).into())
                    .unwrap()
            })
            .collect();

        let (paginated_messages, total, offset, limit) =
            Self::apply_pagination(messages, limit, offset);

        Ok(ThreadMessagesResponse {
            messages: paginated_messages,
            total,
            offset,
            limit,
        })
    }

    async fn debug_state(&self) -> Result<serde_json::Value> {
        let rtxn = self.env.read_txn()?;

        let threads: Vec<(Uuid, Thread)> = self
            .threads_db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, thread)| (k.0, thread))
            .collect();
        let messages: Vec<((Uuid, Uuid), Message)> = self
            .messages_db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, message)| ((k.0 .0, k.0 .1), message))
            .collect();
        let thread_messages: Vec<(Uuid, Vec<Uuid>)> = self
            .thread_messages_db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, v)| (k.0, v))
            .collect();
        let embeddings: Vec<(Uuid, Embedding)> = self
            .embeddings_db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, v)| (k.0, v))
            .collect();
        let thread_creation_times: Vec<(u64, Uuid)> = self
            .thread_creation_time_db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, _)| (k.0 .0, k.0 .1))
            .collect();
        let message_creation_times: Vec<(Uuid, u64, Uuid)> = self
            .message_creation_time_db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, _)| (k.0 .0, k.0 .1, k.0 .2))
            .collect();

        Ok(serde_json::json!({
            "threads": threads,
            "messages": messages,
            "thread_messages": thread_messages,
            "embeddings": embeddings,
            "thread_creation_times": thread_creation_times,
            "message_creation_times": message_creation_times
        }))
    }

    async fn delete_message(&self, thread_id: Uuid, message_id: Uuid) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;

        if self
            .messages_db
            .get(&wtxn, &(thread_id, message_id).into())?
            .is_some()
        {
            self.delete_message_internal(&mut wtxn, thread_id, message_id)?;
            wtxn.commit()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("message not found"))
        }
    }
}

mod api;
mod api_state;

use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

use anyhow::Result;
use api_state::ApiState;
use clap::{Parser, Subcommand};
use ferrochain_anthropic_completion::{AnthropicCompletion, Model};
use ferrochain_voyageai_embedder::{EmbeddingInputType, EmbeddingModel, VoyageAiEmbedder};
use synx::{executor::Executor, Synx};
use synx_heed_database::{heed::EnvOpenOptions, SynxHeedDatabase};
use synx_in_memory_database::SynxInMemory;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct TokioExecutor;

impl Executor for TokioExecutor {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>) {
        tokio::spawn(future);
    }
}

#[derive(Parser)]
struct Cli {
    #[clap(long, default_value = "0.0.0.0")]
    host: String,
    #[clap(long, default_value = "3000")]
    port: u16,
    #[clap(subcommand)]
    database: Database,
}

#[derive(Default, Subcommand)]
enum Database {
    Heed {
        #[clap(long)]
        path: PathBuf,
    },
    #[default]
    InMemory,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,memory=debug,tower_http=debug",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    let synx = Synx::builder()
        .with_db({
            match cli.database {
                Database::Heed { path } => {
                    let env = unsafe {
                        EnvOpenOptions::new()
                            .map_size(10 * 1024 * 1024 * 1024) // 10 GB
                            .max_dbs(6)
                            .open(path)?
                    };

                    Arc::new(SynxHeedDatabase::new(Arc::new(env), true)?)
                }
                Database::InMemory => Arc::new(SynxInMemory::new()),
            }
        })
        .with_document_embedder(Arc::new(
            VoyageAiEmbedder::builder()
                .model(EmbeddingModel::Voyage3)
                .input_type(EmbeddingInputType::Document)
                .build()
                .expect("Failed to create VoyageAiEmbedder"),
        ))
        .with_query_embedder(Arc::new(
            VoyageAiEmbedder::builder()
                .model(EmbeddingModel::Voyage3)
                .input_type(EmbeddingInputType::Query)
                .build()
                .expect("Failed to create VoyageAiEmbedder"),
        ))
        .with_completion(Arc::new(
            AnthropicCompletion::builder()
                .build()
                .expect("Failed to create AnthropicCompletion"),
        ))
        .with_completion_model(Model::ClaudeThreeDotFiveSonnet)
        .with_executor(Arc::new(TokioExecutor))
        .build();

    let state = ApiState { synx };

    let listener = TcpListener::bind((cli.host, cli.port)).await?;
    tracing::debug!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        api::routes::router(state).layer(TraceLayer::new_for_http()),
    )
    .await?;

    Ok(())
}

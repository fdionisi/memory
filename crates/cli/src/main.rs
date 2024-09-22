use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::{Parser, Subcommand};
use ferrochain_anthropic_completion::{AnthropicCompletion, Model};
use ferrochain_voyageai_embedder::{EmbeddingInputType, EmbeddingModel, VoyageAiEmbedder};
use synx::Synx;
use synx_heed_database::{heed::EnvOpenOptions, SynxHeedDatabase};
use synx_in_memory_database::SynxInMemory;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    Synx::builder()
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
        .build()
        .listen(SocketAddr::new(cli.host.parse()?, cli.port))
        .await
}

pub mod address_book;
pub mod alert;
pub mod block_scanner;
pub mod dex;
pub mod helpers;
pub mod mempool;
pub mod uni;

use std::sync::Arc;

use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::*;

use crate::dex::Dex;
use crate::helpers::setup_signer;

use tracing::{Level, event};

pub struct Config {
    #[allow(dead_code)]
    pub http: Arc<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>,
    #[allow(dead_code)]
    pub wss: Arc<Provider<Ws>>,
    pub dex_factory: H160,
    pub dex_router: H160,
}

impl Config {
    pub async fn new() -> Self {
        let network = std::env::var("NETWORK_RPC").expect("missing NETWORK_RPC");
        let provider: Provider<Http> = Provider::<Http>::try_from(network).unwrap();
        let middleware = Arc::new(setup_signer(provider.clone()).await);

        let ws_network = std::env::var("NETWORK_WSS").expect("missing NETWORK_WSS");
        let ws_provider: Provider<Ws> = Provider::<Ws>::connect(ws_network).await.unwrap();

        let dex_factory:H160 = std::env::var("DEX_FACTORY").expect("missing DEX_FACTORY").parse().unwrap();
        let dex_router: H160 = std::env::var("DEX_ROUTER").expect("missing DEX_ROUTER").parse().unwrap();
        
        Self {
            http: middleware,
            wss: Arc::new(ws_provider),
            dex_factory,
            dex_router,
        }
    }

    pub async fn create_dex(&self, factory: Address, router: Address) -> Dex {
        Dex::new(self.http.clone(), factory, router)
    }
}

/// Run the strategy here.
pub async fn run() {
    let config = Config::new().await;

    let file_appender = tracing_appender::rolling::never(".", "log.txt");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO) // Set the maximum log level
        .compact() // Disable the log crate's own formatting
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_writer(non_blocking) // Write logs to a file
        .with_ansi(false) // Disable ANSI escape codes
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    event!(Level::INFO, "Starting Sandbot ...");    

    // Example of how to interact with a contract.
    let dex = config.create_dex(config.dex_factory, config.dex_router).await;
    dex.get_pairs().await;

    // Thread for checking what block we're on.
    tokio::spawn(async move {
        block_scanner::loop_blocks(Arc::clone(&config.http)).await;
    });

    // Main loop to monitor the mempool.
    mempool::loop_mempool(Arc::clone(&config.wss)).await;
}

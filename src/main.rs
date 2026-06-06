use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use ethers::prelude::*;
use rand::RngCore;
use std::fs::File;
use std::io::{Write, Read};
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use prometheus::{Registry, Counter, Gauge, register_counter_with_registry, register_gauge_with_registry};
use lazy_static::lazy_static;
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;
use axum::{Router, routing::get};

// --- GLOBAL SIGNALS & SELECTORS ---
const ERC20_TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];
const UNISWAP_V2_SWAP_SELECTOR: [u8; 4] = [0xd7, 0x8a, 0xd8, 0x0d];
const MINIMUM_STABLECOIN_THRESHOLD: u64 = 100_000_000;
const REVERT_SELECTOR: &str = "0x08c379a0";

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref MEMPOOL_TX_PROCESSED: Counter = register_counter_with_registry!(
        "titan_mempool_tx_processed_total",
        "Total mempool transactions processed",
        &REGISTRY
    ).unwrap();
    pub static ref HONEYPOTS_INTERCEPTED: Counter = register_counter_with_registry!(
        "titan_honeypots_intercepted_total",
        "Total honeypot contracts intercepted",
        &REGISTRY
    ).unwrap();
    pub static ref SUCCESSFUL_INTERCEPTIONS: Counter = register_counter_with_registry!(
        "titan_successful_interceptions_total",
        "Total successful transaction interceptions",
        &REGISTRY
    ).unwrap();
    pub static ref ACCUMULATED_PROFIT: Gauge = register_gauge_with_registry!(
        "titan_accumulated_profit_usd",
        "Accumulated profit in USD",
        &REGISTRY
    ).unwrap();
}

// --- SECURE PERSISTENT LEDGER ---
pub struct LedgerEngine {
    pool: Pool<Postgres>,
}

impl LedgerEngine {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS interception_ledger (
                id UUID PRIMARY KEY,
                timestamp TIMESTAMPTZ NOT NULL,
                target_tx_hash VARCHAR(66) NOT NULL,
                token_address VARCHAR(42) NOT NULL,
                extracted_amount_usd NUMERIC(18, 4) NOT NULL,
                gas_spent_gwei NUMERIC(12, 2) NOT NULL,
                execution_status VARCHAR(20) NOT NULL
            );",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    pub async fn record_interception(
        &self,
        tx_hash: &str,
        token: &str,
        amount: f64,
        gas: f64,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO interception_ledger (id, timestamp, target_tx_hash, token_address, extracted_amount_usd, gas_spent_gwei, execution_status)
             VALUES ($1, NOW(), $2, $3, $4, $5, $6)",
        )
        .bind(uuid::Uuid::new_v4())
        .bind(tx_hash)
        .bind(token)
        .bind(amount)
        .bind(gas)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// --- ENCRYPTED OFFLINE ARCHIVER ---
pub struct SecureArchiver;

impl SecureArchiver {
    pub fn create_encrypted_backup(
        raw_ledger_json: &str,
        secret_passphrase: &[u8; 32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let zip_path = "titan_ledger_backup.zip";
        let file = File::create(zip_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("ledger_dump.json", options)?;
        zip.write_all(raw_ledger_json.as_bytes())?;
        zip.finish()?;

        let mut unencrypted_bytes = Vec::new();
        File::open(zip_path)?.read_to_end(&mut unencrypted_bytes)?;

        let key = Key::<Aes256Gcm>::from_slice(secret_passphrase);
        let cipher = Aes256Gcm::new(key);
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, unencrypted_bytes.as_ref())?;

        let mut final_archive = File::create("titan_ledger_encrypted.vault")?;
        final_archive.write_all(&nonce_bytes)?;
        final_archive.write_all(&ciphertext)?;

        std::fs::remove_file(zip_path)?;

        info!("✓ Encrypted vault archive generated securely: titan_ledger_encrypted.vault");
        Ok(())
    }
}

// --- MAIN RUNTIME ORCHESTRATOR ---
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    info!("🚀 Booting Multi-Agent Workspace Nodes...");

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/titan".to_string());

    let _ledger = Arc::new(LedgerEngine::connect(&db_url).await?);
    info!("✓ Secure PostgreSQL Persistence Ledger verified and connected");

    // Setup Axum web server
    let app = Router::new()
        .route("/", get(|| async { "Titan Agent System Running" }))
        .route("/health", get(|| async { "OK" }))
        .route("/metrics", get(|| async { format_metrics() }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("🌐 Titan Gateway running on 0.0.0.0:8080");

    axum::serve(listener, app).await?;

    Ok(())
}

fn format_metrics() -> String {
    use std::io::Write;
    let mut buffer = Vec::new();
    let encoder = prometheus::TextEncoder::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

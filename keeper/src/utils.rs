use std::str::FromStr;

use crate::PostgresArgs;
use anchor_client::{
    solana_sdk::{pubkey::Pubkey, signer::Signer},
    Client, Cluster, Program,
};
use anyhow::*;

use log::error;
use sqlx::{postgres::PgConnectOptions, ConnectOptions, Pool, Postgres};

/// Build a valid postgres connection string
fn build_pg_connection_str(args: PostgresArgs) -> String {
    format!(
        "postgres://{}:{}@{}/{}",
        args.postgres_user, args.postgres_password, args.postgres_socket_address, args.postgres_db
    )
}

/// Create postgres database pool connection
pub async fn create_pg_pool(args: PostgresArgs) -> Result<Pool<Postgres>> {
    let pg_conn = build_pg_connection_str(args);

    // println!("{}", pg_conn);

    let mut options = PgConnectOptions::from_str(&pg_conn)?;
    options.disable_statement_logging();

    let pool = sqlx::PgPool::connect_with(options).await?;

    Ok(pool)
}

/// Standardize logging for error with prefix ERROR
pub fn log_error(message: &str, error: Error) {
    error!("ERROR: {}. Details: {}", message, error.to_string());
}

/// Create an anchor program instance
pub fn create_program<C: Clone + std::ops::Deref<Target = impl Signer>>(
    http_provider: String,
    wss_provider: String,
    program_id: Pubkey,
    payer: C,
) -> Result<Program<C>> {
    let cluster = Cluster::Custom(http_provider, wss_provider);
    let client = Client::new(cluster, payer);
    let program = client.program(program_id)?;

    Ok(program)
}

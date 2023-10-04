// pub mod client_pool;
pub mod anchor_adapter;
pub mod core;
pub mod database;
pub mod router;
pub mod state;
pub mod sync_gauge;
pub mod utils;
#[macro_use]
pub mod macros;

use crate::core::Core;
use crate::state::init_state;
use crate::utils::create_pg_pool;
use clap::Parser;
use hyper::Server;
use log::info;
use router::router;
use routerify::RouterService;
use sqlx::migrate::Migrator;
use state::init_epoch_infos;
use std::result::Result::Ok;
use std::sync::Arc;
use tokio::time::interval;
use tokio::time::Duration;
const MONITOR_GAUGE_FACTORY: u64 = 60 * 1; // 1 minutes

const MONITOR_GAUGE: u64 = 60 * 1; // 1 minutes

static MIGRATOR: Migrator = sqlx::migrate!();

#[derive(Parser, Debug)]
pub struct PostgresArgs {
    /// User for postgres database. For example: meteora
    #[clap(long)]
    postgres_user: String,
    /// Password for postgres database. For example: meteora1234
    #[clap(long)]
    postgres_password: String,
    /// Postgres database name. For example: clmm_keeper
    #[clap(long)]
    postgres_db: String,
    /// Postgres database socket address. For example: 127.0.0.1:8888
    #[clap(long)]
    postgres_socket_address: String,
}

#[derive(Parser, Debug)]
pub struct Args {
    /// Base address for gauge factory
    #[clap(long)]
    base: String,
    /// Socket address the keeper to bind to. For example: 0.0.0.0:5566
    #[clap(long)]
    socket_address: String,
    #[clap(flatten)]
    postgres_args: PostgresArgs,
    /// Solana RPC provider. For example: https://api.mainnet-beta.solana.com
    #[clap(long)]
    provider: String,
    /// Keypair, used to do permissionless actions like trigger next epoch
    #[clap(long, default_value_t = String::from(shellexpand::tilde("~/.config/solana/id.json")))]
    keypair_url: String,
    /// should trigger
    #[clap(long, default_value_t = 0)]
    should_crank: u64,
}

#[tokio::main(worker_threads = 20)] // TODO figure out why it is blocking in linux
async fn main() {
    let Args {
        base,
        socket_address,
        postgres_args,
        provider,
        keypair_url,
        should_crank,
    } = Args::parse();

    let pg_pool = create_pg_pool(postgres_args).await.unwrap();
    MIGRATOR.run(&pg_pool).await.unwrap();

    let core = Core {
        pg_pool,
        base,
        provider,
        state: init_state(),
        epochs: init_epoch_infos(),
        keypair_url,
    };

    // init some state
    core.init().await.unwrap();

    let core: Arc<Core> = Arc::new(core);
    let mut handles = vec![];

    {
        // cache gauge factory
        let core = core.clone();
        let handle = tokio::spawn(async move {
            let duration = MONITOR_GAUGE_FACTORY;
            let mut interval = interval(Duration::from_secs(duration));
            loop {
                interval.tick().await;
                info!("process_monitor gauge factory");
                core.process_monitor_gauge_factory().await;
            }
        });
        handles.push(handle);
    }

    {
        // cache gauge
        let core = core.clone();
        let handle = tokio::spawn(async move {
            let duration = MONITOR_GAUGE;
            let mut interval = interval(Duration::from_secs(duration));
            loop {
                interval.tick().await;
                info!("process_monitor gauge");
                match core.process_monitor_gauge().await {
                    Ok(_) => {}
                    Err(err) => println!("process_monitor_gauge err {}", err),
                }
            }
        });
        handles.push(handle);
    }

    {
        // crawl epoch up
        let core = core.clone();
        let handle = tokio::spawn(async move {
            let duration = 10 * 1; // 1 min
            let mut interval = interval(Duration::from_secs(duration));
            loop {
                interval.tick().await;
                info!("process_crawl_epoch_up");
                match core.process_crawl_epoch_up().await {
                    Ok(_) => {}
                    Err(err) => println!("process_crawl_epoch_up err {}", err),
                }
            }
        });
        handles.push(handle);
    }

    {
        // crawl epoch down
        let core = core.clone();
        let handle = tokio::spawn(async move {
            let duration = 10 * 1; // 1 min
            let mut interval = interval(Duration::from_secs(duration));
            loop {
                interval.tick().await;
                info!("process_crawl_epoch_down");
                match core.process_crawl_epoch_down().await {
                    Ok(_) => {}
                    Err(err) => println!("process_crawl_epoch_down err {}", err),
                }
            }
        });
        handles.push(handle);
    }

    {
        // crawl bribe
        let core = core.clone();
        let handle = tokio::spawn(async move {
            let duration = 10 * 1; // 1 min
            let mut interval = interval(Duration::from_secs(duration));
            loop {
                interval.tick().await;
                info!("process_crawl_bribe");
                match core.process_crawl_bribe().await {
                    Ok(_) => {}
                    Err(err) => println!("process_crawl_bribe err {}", err),
                }
            }
        });
        handles.push(handle);
    }

    {
        // crawl bribe
        let core = core.clone();
        let handle = tokio::spawn(async move {
            let duration = 30 * 1; // 1 min
            let mut interval = interval(Duration::from_secs(duration));
            loop {
                interval.tick().await;
                info!("process_cache_latest_epoches");
                match core.process_cache_latest_epoches().await {
                    Ok(_) => {}
                    Err(err) => println!("process_cache_latest_epoches err {}", err),
                }
            }
        });
        handles.push(handle);
    }

    if should_crank == 1 {
        {
            // sync gauge
            let core: Arc<Core> = core.clone();
            let handle = tokio::spawn(async move {
                let duration = 10 * 1; // 1 min
                let mut interval = interval(Duration::from_secs(duration));
                loop {
                    interval.tick().await;
                    info!("process_sync_gauge");
                    match core.process_sync_gauge().await {
                        Ok(_) => {}
                        Err(err) => println!("process_sync_gauge err {}", err),
                    }
                }
            });
            handles.push(handle);
        }
    }

    let router = router(core);

    let service = RouterService::new(router).unwrap();

    let addr = ([0, 0, 0, 0], 8080).into();

    let server = Server::bind(&addr).serve(service);

    server.await.unwrap();

    for handle in handles {
        handle.await.unwrap();
    }
}

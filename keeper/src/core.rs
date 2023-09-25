// use gauge::GaugeFactory;
use crate::state::{BribeInfo, DaoState, GaugeFactoryState, GaugeInfo, GaugeState};
use crate::unwrap_ok_or;

use crate::anchor_adapter::AClock;
use crate::database::*;
use crate::sync_gauge::*;
use crate::utils::create_program;
use anchor_client::solana_client::nonblocking::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_filter::RpcFilterType;
use anchor_client::solana_sdk::signature::read_keypair_file;
use anchor_client::solana_sdk::signer::keypair::Keypair;
use anchor_client::Program;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar;
use anchor_lang::AccountDeserialize;
use anyhow::Result;
use sqlx::Pool;
use sqlx::Postgres;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;
pub struct Core {
    pub pg_pool: Pool<Postgres>,
    pub base: String,
    pub provider: String,
    pub keypair_url: String,
    pub state: Arc<Mutex<DaoState>>,
}

impl Core {
    pub fn get_gauge_factory_addr(&self) -> Pubkey {
        let base_pk = Pubkey::from_str(&self.base).unwrap();
        let (gauge_factory, _bump) = Pubkey::find_program_address(
            &[b"GaugeFactory".as_ref(), base_pk.as_ref()],
            &gauge::id(),
        );
        gauge_factory
    }

    pub async fn init(&self) -> Result<()> {
        let program: Program<Arc<Keypair>> = create_program(
            self.provider.to_string(),
            self.provider.to_string(),
            gauge::ID,
            Arc::new(Keypair::new()),
        )?;
        let gauge_factory = self.get_gauge_factory_addr();
        let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory).await?;
        let gauges: Vec<(Pubkey, gauge::Gauge)> = program
            .accounts::<gauge::Gauge>(vec![RpcFilterType::DataSize(
                (8 + std::mem::size_of::<gauge::Gauge>()) as u64,
            )])
            .await?;

        // filter gauge
        let gauges = gauges
            .into_iter()
            .filter(|&x| x.1.gauge_factory == gauge_factory)
            .collect::<Vec<(Pubkey, gauge::Gauge)>>();

        {
            let mut state = self.state.lock().unwrap();
            state.save_gauge(&gauges);
            state.save_gauge_factory(&gauge_factory_state, self.base.clone(), &gauge_factory);
        };

        let crawl_epoch_up = get_voting_epoch_up(&self.pg_pool).await;
        match crawl_epoch_up {
            Ok(_value) => {}
            Err(err) => match err {
                sqlx::Error::RowNotFound => {
                    init_crawl_config(
                        &self.pg_pool,
                        gauge_factory_state.current_voting_epoch.into(),
                    )
                    .await?
                }
                _ => {
                    return Err(anyhow::Error::msg("cannot get crawl config"));
                }
            },
        };

        Ok(())
    }
    pub async fn process_monitor_gauge_factory(&self) {
        let program: Program<Arc<Keypair>> = unwrap_ok_or!(
            create_program(
                self.provider.to_string(),
                self.provider.to_string(),
                gauge::ID,
                Arc::new(Keypair::new()),
            ),
            "Cannot get program client"
        );
        let gauge_factory = self.get_gauge_factory_addr();
        let gauge_factory_state: gauge::GaugeFactory = unwrap_ok_or!(
            program.account(gauge_factory).await,
            "cannot get gauge state"
        );

        let mut state = self.state.lock().unwrap();
        state.save_gauge_factory(&gauge_factory_state, self.base.clone(), &gauge_factory);
    }

    pub async fn process_monitor_gauge(&self) {
        let program: Program<Arc<Keypair>> = unwrap_ok_or!(
            create_program(
                self.provider.to_string(),
                self.provider.to_string(),
                gauge::ID,
                Arc::new(Keypair::new()),
            ),
            "Cannot get program client"
        );

        let gauge_factory = self.get_gauge_factory_addr();

        let gauges: Vec<(Pubkey, gauge::Gauge)> = unwrap_ok_or!(
            program
                .accounts::<gauge::Gauge>(vec![RpcFilterType::DataSize(
                    (8 + std::mem::size_of::<gauge::Gauge>()) as u64,
                )])
                .await,
            "Cannot get gauges"
        );

        // filter gauge
        let gauges = gauges
            .into_iter()
            .filter(|&x| x.1.gauge_factory == gauge_factory)
            .collect::<Vec<(Pubkey, gauge::Gauge)>>();

        let mut state = self.state.lock().unwrap();
        state.save_gauge(&gauges);
    }

    pub async fn process_crawl_epoch_up(&self) -> Result<()> {
        let current_voting_epoch = {
            let state: std::sync::MutexGuard<'_, DaoState> = self.state.lock().unwrap();
            if !state.is_gauge_factory_initialized() {
                return Ok(());
            }
            state.gauge_factory.current_voting_epoch
        };
        let crawl_epoch_up = get_voting_epoch_up(&self.pg_pool).await?;
        let crawl_epoch_up: u32 = crawl_epoch_up.try_into()?;

        let should_craw_epoch = if current_voting_epoch > crawl_epoch_up {
            crawl_epoch_up
        } else {
            current_voting_epoch
        };

        let gauges = {
            let state = self.state.lock().unwrap();
            let gauges = state.get_gauges();
            let gauge_pubkeys: Vec<Pubkey> = gauges
                .iter()
                .map(|gauge| Pubkey::from_str(&gauge.pubkey).unwrap())
                .collect();
            gauge_pubkeys
        };

        let epoch_pubkeys: Vec<Pubkey> = gauges
            .iter()
            .map(|&gauge| {
                let (epoch_gauge, _bump) = Pubkey::find_program_address(
                    &[
                        b"EpochGauge".as_ref(),
                        gauge.as_ref(),
                        should_craw_epoch.to_le_bytes().as_ref(),
                    ],
                    &gauge::id(),
                );
                epoch_gauge
            })
            .collect();

        let rpc_client = RpcClient::new(self.provider.clone());

        let epoch_gauges = rpc_client.get_multiple_accounts(&epoch_pubkeys).await?;

        let epoch_gauges: Vec<gauge::EpochGauge> = epoch_gauges
            .into_iter()
            .filter(|x| !x.is_none())
            .map(|x| gauge::EpochGauge::try_deserialize(&mut x.unwrap().data.as_ref()).unwrap())
            .collect();

        save_epoch_gauges_up(
            &self.pg_pool,
            &epoch_gauges,
            (should_craw_epoch + 1).into(),
            should_craw_epoch < current_voting_epoch,
        )
        .await?;

        Ok(())
    }

    pub async fn process_crawl_epoch_down(&self) -> Result<()> {
        let crawl_epoch_down = get_voting_epoch_down(&self.pg_pool).await?;
        if crawl_epoch_down < 0 {
            return Ok(());
        }
        let crawl_epoch_down = u32::try_from(crawl_epoch_down)?;

        let gauges = {
            let state = self.state.lock().unwrap();
            let gauges = state.get_gauges();
            let gauge_pubkeys: Vec<Pubkey> = gauges
                .iter()
                .map(|gauge| Pubkey::from_str(&gauge.pubkey).unwrap())
                .collect();
            gauge_pubkeys
        };

        let epoch_pubkeys: Vec<Pubkey> = gauges
            .iter()
            .map(|&gauge| {
                let (epoch_gauge, _bump) = Pubkey::find_program_address(
                    &[
                        b"EpochGauge".as_ref(),
                        gauge.as_ref(),
                        crawl_epoch_down.to_le_bytes().as_ref(),
                    ],
                    &gauge::id(),
                );
                epoch_gauge
            })
            .collect();

        let rpc_client = RpcClient::new(self.provider.clone());

        let epoch_gauges = rpc_client.get_multiple_accounts(&epoch_pubkeys).await?;

        let epoch_gauges: Vec<gauge::EpochGauge> = epoch_gauges
            .into_iter()
            .filter(|x| !x.is_none())
            .map(|x| gauge::EpochGauge::try_deserialize(&mut x.unwrap().data.as_ref()).unwrap())
            .collect();

        let crawl_epoch_down: i64 = crawl_epoch_down.into();
        save_epoch_gauges_down(&self.pg_pool, &epoch_gauges, crawl_epoch_down - 1).await?;

        Ok(())
    }

    pub async fn process_crawl_bribe(&self) -> Result<()> {
        let bribe_index = get_max_bribe_index(&self.pg_pool).await?;

        let current_bribe_index = {
            let state = self.state.lock().unwrap();
            (state.gauge_factory.bribe_index - 1).into()
        };

        if bribe_index >= current_bribe_index {
            return Ok(());
        }
        let next_bribe_index: u32 = (bribe_index + 1).try_into()?;

        let (bribe, _bump) = Pubkey::find_program_address(
            &[
                b"Bribe".as_ref(),
                self.get_gauge_factory_addr().as_ref(),
                next_bribe_index.to_le_bytes().as_ref(),
            ],
            &gauge::id(),
        );

        let program: Program<Arc<Keypair>> = create_program(
            self.provider.to_string(),
            self.provider.to_string(),
            gauge::ID,
            Arc::new(Keypair::new()),
        )?;

        let bribe_state: gauge::Bribe = program.account(bribe).await?;

        save_bribe(&self.pg_pool, bribe, &bribe_state).await?;

        Ok(())
    }

    pub async fn process_sync_gauge(&self) -> Result<()> {
        //payer
        // let payer = read_keypair_file(self.keypair_url.clone()).expect("Requires a keypair file");
        let program: Program<Arc<Keypair>> = create_program(
            self.provider.to_string(),
            self.provider.to_string(),
            gauge::ID,
            Arc::new(read_keypair_file(self.keypair_url.clone()).expect("Requires a keypair file")),
        )?;

        // let current_node_time =
        let clock: AClock = program.account(sysvar::clock::id()).await?;
        let current_node_time = u64::try_from(clock.unix_timestamp)?;
        let should_trigger_next_epoch = {
            let state = self.state.lock().unwrap();
            state.should_trigger_next_epoch(current_node_time)
        };

        let gauge_factory = self.get_gauge_factory_addr();
        // trigger next epoch
        if should_trigger_next_epoch {
            trigger_next_epoch(&program, &self.keypair_url, gauge_factory).await?;
        }

        // check whether old gauge are sync
        let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory).await?;
        let gauges = {
            let state = self.state.lock().unwrap();
            let gauges = state.get_gauges();
            let gauge_pubkeys: Vec<Pubkey> = gauges
                .into_iter()
                .filter(|x| !x.is_disabled)
                .map(|x| Pubkey::from_str(&x.pubkey).unwrap())
                .collect();
            gauge_pubkeys
        };
        for gauge_pk in gauges.iter() {
            let (epoch_gauge, _bump) = Pubkey::find_program_address(
                &[
                    b"EpochGauge".as_ref(),
                    gauge_pk.as_ref(),
                    gauge_factory_state.rewards_epoch()?.to_le_bytes().as_ref(),
                ],
                &gauge::id(),
            );
            match program.rpc().get_account(&epoch_gauge) {
                Ok(account) => {
                    let epoch_gauge_state =
                        gauge::EpochGauge::try_deserialize(&mut account.data.as_ref())?;
                    let gauge_state: gauge::Gauge = program.account(*gauge_pk).await?;
                    let quarry_state: quarry::Quarry = program.account(gauge_state.quarry).await?;
                    if quarry_state.rewards_share != epoch_gauge_state.total_power {
                        println!(
                            "sync gauge {} epoch {} quarry_rewards_share {} epoch_total_power {}",
                            gauge_pk,
                            gauge_factory_state.rewards_epoch()?,
                            quarry_state.rewards_share,
                            epoch_gauge_state.total_power
                        );
                        sync_gauge(
                            &program,
                            &self.keypair_url,
                            gauge_factory,
                            *gauge_pk,
                            gauge_factory_state.rewards_epoch()?,
                            false,
                        )
                        .await?;
                    }
                }
                Err(err) => {
                    // println!("{}", err);
                }
            }
        }

        // check whether to new epoch gauge is created
        for gauge_pk in gauges.iter() {
            let (epoch_gauge, _bump) = Pubkey::find_program_address(
                &[
                    b"EpochGauge".as_ref(),
                    gauge_pk.as_ref(),
                    gauge_factory_state
                        .current_voting_epoch
                        .to_le_bytes()
                        .as_ref(),
                ],
                &gauge::id(),
            );
            match program.rpc().get_account(&epoch_gauge) {
                Ok(_account) => {}
                Err(err) => {
                    println!("create epoch gauge {}", gauge_pk);
                    create_epoch_gauge(&program, &self.keypair_url, gauge_factory, *gauge_pk)
                        .await?;
                }
            }
        }

        Ok(())
    }

    pub fn get_gauge_factory(&self) -> GaugeFactoryState {
        let state: std::sync::MutexGuard<'_, DaoState> = self.state.lock().unwrap();
        return state.gauge_factory.clone();
    }

    pub fn get_gauges(&self) -> Vec<GaugeState> {
        let state: std::sync::MutexGuard<'_, DaoState> = self.state.lock().unwrap();
        return state.get_gauges();
    }

    pub async fn get_epoch_info(&self, epoch: u64) -> Result<Vec<GaugeInfo>> {
        let epoch: i64 = epoch.try_into()?;
        let epoch_gauges = get_epoch_gauges(&self.pg_pool, epoch).await?;
        let bribes = get_bribes(&self.pg_pool, epoch).await?;
        let gauges = self.get_gauges();

        let mut gauge_infos = vec![];
        for epoch_gauge in epoch_gauges.iter() {
            let gauge = DaoState::get_gauge(&gauges, epoch_gauge.gauge.clone())?;

            let bribes: Vec<BribeInfo> = bribes
                .clone()
                .into_iter()
                .filter(|x| {
                    x.gauge.clone() == epoch_gauge.gauge.clone()
                        && x.reward_each_epoch.parse::<u64>().is_ok()
                })
                .map(|x| BribeInfo {
                    pubkey: x.address,
                    token_mint: x.token_mint,
                    bribe_index: x.bribe_index as u32,
                    reward_each_epoch: x.reward_each_epoch.parse::<u64>().unwrap(),
                })
                .collect();

            gauge_infos.push(GaugeInfo {
                gauge_pk: epoch_gauge.gauge.clone(),
                voting_power: epoch_gauge.total_power.parse::<u64>()?,
                token_a_mint: gauge.token_a_mint,
                token_b_mint: gauge.token_b_mint,
                token_a_fee: epoch_gauge.token_a_fee.parse::<u64>()?,
                token_b_fee: epoch_gauge.token_b_fee.parse::<u64>()?,
                bribes,
            })
        }

        Ok(gauge_infos)
    }
}

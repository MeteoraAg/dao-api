use anchor_client::solana_sdk::pubkey;
// use gauge::GaugeFactory;
use anchor_client::solana_sdk::signature::Signable;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::blake3::Hash;
use anyhow::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct GaugeFactoryState {
    pub pubkey: String,
    pub base: String,
    pub rewarder: String,
    pub locker: String,
    pub foreman: String,
    pub epoch_duration_seconds: u32,
    pub current_voting_epoch: u32,
    pub next_epoch_starts_at: u64,
    pub bribe_index: u32,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct GaugeState {
    pub pubkey: String,
    pub quarry: String,
    pub amm_pool: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_fee_key: String,
    pub token_b_fee_key: String,
    pub is_disabled: bool,
    pub cummulative_token_a_fee: u128,
    pub cummulative_token_b_fee: u128,
    pub cummulative_claimed_token_a_fee: u128,
    pub cummulative_claimed_token_b_fee: u128,
    pub amm_type: u64,
}

pub struct DaoState {
    pub gauge_factory: GaugeFactoryState,
    pub gauges: HashMap<Pubkey, GaugeState>,
    pub quarries: HashMap<Pubkey, quarry::Quarry>,

    pub pools: HashMap<Pubkey, PoolDynamicInfo>,
    pub quarry_infos: HashMap<Pubkey, QuarryDynamicInfo>,
}

pub fn init_state() -> Arc<Mutex<DaoState>> {
    let e = DaoState {
        gauge_factory: GaugeFactoryState::default(),
        gauges: HashMap::new(),
        quarries: HashMap::new(),
        pools: HashMap::new(),
        quarry_infos: HashMap::new(),
    };
    Arc::new(Mutex::new(e))
}

impl DaoState {
    pub fn should_trigger_next_epoch(&self, current_node_time: u64) -> bool {
        current_node_time >= self.gauge_factory.next_epoch_starts_at
    }
    pub fn is_gauge_factory_initialized(&self) -> bool {
        self.gauge_factory.pubkey != String::default()
    }
    pub fn save_gauge_factory(
        &mut self,
        gauge_factory: &gauge::GaugeFactory,
        base: String,
        pubkey: &Pubkey,
    ) {
        self.gauge_factory.base = base;
        self.gauge_factory.pubkey = pubkey.to_string();
        self.gauge_factory.rewarder = gauge_factory.rewarder.to_string();
        self.gauge_factory.locker = gauge_factory.locker.to_string();
        self.gauge_factory.foreman = gauge_factory.foreman.to_string();
        self.gauge_factory.epoch_duration_seconds = gauge_factory.epoch_duration_seconds;
        self.gauge_factory.current_voting_epoch = gauge_factory.current_voting_epoch;
        self.gauge_factory.next_epoch_starts_at = gauge_factory.next_epoch_starts_at;
        self.gauge_factory.bribe_index = gauge_factory.bribe_index;
    }

    pub fn save_gauges_and_quarries(
        &mut self,
        gauges: &Vec<(Pubkey, gauge::Gauge)>,
        quarries: &Vec<(Pubkey, quarry::Quarry)>,
    ) {
        for (pubkey, gauge) in gauges.iter() {
            let gauge_state = GaugeState {
                pubkey: pubkey.to_string(),
                quarry: gauge.quarry.to_string(),
                amm_pool: gauge.amm_pool.to_string(),
                token_a_mint: gauge.token_a_mint.to_string(),
                token_b_mint: gauge.token_b_mint.to_string(),
                token_a_fee_key: gauge.token_a_fee_key.to_string(),
                token_b_fee_key: gauge.token_b_fee_key.to_string(),
                is_disabled: gauge.is_disabled,
                cummulative_token_a_fee: gauge.cummulative_token_a_fee,
                cummulative_token_b_fee: gauge.cummulative_token_b_fee,
                cummulative_claimed_token_a_fee: gauge.cummulative_claimed_token_a_fee,
                cummulative_claimed_token_b_fee: gauge.cummulative_claimed_token_b_fee,
                amm_type: gauge.amm_type,
            };

            self.gauges.insert(*pubkey, gauge_state);
        }

        for (pubkey, quarry) in quarries.iter() {
            self.quarries.insert(*pubkey, *quarry);
        }
    }
    pub fn get_gauges(&self) -> Vec<GaugeState> {
        let mut gauges = vec![];
        for (_pubkey, gauge) in self.gauges.iter() {
            gauges.push(gauge.clone());
        }
        gauges
    }

    pub fn get_gauge(gauges: &Vec<GaugeState>, pubkey: String) -> Result<GaugeState> {
        for gauge in gauges.iter() {
            if gauge.pubkey == pubkey {
                return Ok(gauge.clone());
            }
        }
        return Err(Error::msg("cannot find gauge"));
    }

    pub fn get_pools(&self) -> HashMap<Pubkey, PoolDynamicInfo> {
        self.pools.clone()
    }
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct EpochGaugeInfoWrapper {
    pub epoch: u64,
    pub gauges: Vec<GaugeInfo>,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct GaugeInfo {
    pub gauge_pk: String,
    pub pool_pk: String,
    pub quarry_pk: String,
    pub voting_power: u64,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_fee: u64,
    pub token_b_fee: u64,
    pub bribes: Vec<BribeInfo>,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct BribeInfo {
    pub pubkey: String,
    pub token_mint: String,
    pub bribe_index: u32,
    pub reward_each_epoch: u64,
}

pub struct EpochInfos {
    pub epochs: HashMap<u64, Vec<GaugeInfo>>,
    pub max_cached: u64,
}

pub fn init_epoch_infos() -> Arc<Mutex<EpochInfos>> {
    let e = EpochInfos {
        epochs: HashMap::new(),
        max_cached: 3,
    };
    Arc::new(Mutex::new(e))
}

impl EpochInfos {
    pub fn clear_old_epochs(&mut self, latest_epoch: u64) {
        let mut epochs = HashMap::new();
        let start_cached_epoch = if latest_epoch > self.max_cached {
            latest_epoch - self.max_cached
        } else {
            0
        };
        for epoch in start_cached_epoch..=latest_epoch {
            match self.epochs.get(&epoch) {
                Some(value) => {
                    epochs.insert(epoch, value.clone());
                }
                None => continue,
            }
        }
        self.epochs = epochs;
    }
    pub fn save_epoch(&mut self, epoch: u64, epoch_info: Vec<GaugeInfo>) {
        self.epochs.insert(epoch, epoch_info);
    }
    pub fn get_epoch_info(&self, epoch: u64) -> Result<Vec<GaugeInfo>> {
        let epoch_info = self
            .epochs
            .get(&epoch)
            .ok_or(anyhow::Error::msg("Cannot find epoch"))?;
        Ok(epoch_info.clone())
    }
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PoolDynamicInfo {
    pub pubkey: Pubkey,
    pub tvl: String,        // in usd
    pub quarry_tvl: String, // in usd
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct QuarryDynamicInfo {
    pub pubkey: Pubkey,
    pub tvl: String,        // in usd
    pub quarry_tvl: String, // in usd
    pub apy: String,
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pubkey: String,
    pub amm_type: u64,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub tvl: String,        // in usd
    pub quarry_tvl: String, // in usd
}

#[derive(Default, PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct QuarryInfo {
    pub pubkey: String,
    pub quarry_tvl: String, // in usd
    pub apy: String,
    // those info are from state:
    /// Total number of tokens deposited into the quarry.
    pub total_tokens_deposited: u64,
    /// Number of [Miner]s.
    pub num_miners: u64,
    /// Timestamp when quarry rewards cease
    pub famine_ts: i64,
    /// Amm pool this quarry is designated to
    pub amm_pool: String,
    /// Amm type, can be Meteora or LbClmm
    pub amm_type: u64,
    /// Amount of rewards distributed to the quarry per year.
    pub annual_rewards_rate: u64,
    /// Rewards shared allocated to this quarry
    pub rewards_share: u64,
}

use std::convert::TryInto;

// use anyhow::Result;
use anchor_lang::prelude::Pubkey;
use sqlx::Pool;
use sqlx::Postgres;
use sqlx::QueryBuilder;

#[derive(Debug)]
pub struct CrawlConfig {
    pub voting_epoch_up: i64,
    pub voting_epoch_down: i64,
}

pub async fn get_voting_epoch_up(pg_pool: &Pool<Postgres>) -> Result<i64, sqlx::Error> {
    let config: CrawlConfig = sqlx::query_as!(CrawlConfig, r#"SELECT * FROM crawl_config"#)
        .fetch_one(pg_pool)
        .await?;

    Ok(config.voting_epoch_up)
}

pub async fn init_crawl_config(
    pg_pool: &Pool<Postgres>,
    current_voting_epoch: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO crawl_config (voting_epoch_up, voting_epoch_down) VALUES ($1, $2)
        "#,
        current_voting_epoch,
        current_voting_epoch - 1,
    )
    .execute(pg_pool)
    .await?;
    Ok(())
}

pub async fn get_voting_epoch_down(pg_pool: &Pool<Postgres>) -> Result<i64, sqlx::Error> {
    let config: CrawlConfig = sqlx::query_as!(CrawlConfig, r#"SELECT * FROM crawl_config"#)
        .fetch_one(pg_pool)
        .await?;

    Ok(config.voting_epoch_down)
}

pub async fn save_epoch_gauges_up(
    pg_pool: &Pool<Postgres>,
    epoch_gauges: &Vec<gauge::EpochGauge>,
    voting_epoch_up: i64,
    should_save_voting_epoch: bool,
) -> anyhow::Result<()> {
    let mut tx = pg_pool.begin().await?;

    if should_save_voting_epoch {
        sqlx::query!(
            "UPDATE crawl_config SET voting_epoch_up = $1",
            voting_epoch_up,
        )
        .execute(&mut tx)
        .await?;
    }
    for epoch_gauge in epoch_gauges.iter() {
        let (pubkey, _bump) = Pubkey::find_program_address(
            &[
                b"EpochGauge".as_ref(),
                epoch_gauge.gauge.as_ref(),
                epoch_gauge.voting_epoch.to_le_bytes().as_ref(),
            ],
            &gauge::id(),
        );
        sqlx::query!(
            r#"
                insert into epoch_gauge(address, gauge, voting_epoch, total_power, token_a_fee, token_b_fee) values($1, $2, $3, $4, $5, $6)
                ON CONFLICT (address) 
                DO
                    UPDATE SET total_power = $7
            "#,
            pubkey.to_string(),
            epoch_gauge.gauge.to_string(),
            epoch_gauge.voting_epoch as i64, // TODO change type
            epoch_gauge.total_power as i64,// TODO change type
            epoch_gauge.token_a_fee as i64,// TODO change type
            epoch_gauge.token_b_fee as i64,// TODO change type
            epoch_gauge.total_power as i64,// TODO change type
        )
        .execute(&mut tx)
        .await?;
    }

    // insertions become visible to other connections only after this point
    tx.commit().await?;

    Ok(())
}

pub async fn save_epoch_gauges_down(
    pg_pool: &Pool<Postgres>,
    epoch_gauges: &Vec<gauge::EpochGauge>,
    voting_epoch_down: i64,
) -> anyhow::Result<()> {
    let mut tx = pg_pool.begin().await?;

    sqlx::query!(
        "UPDATE crawl_config SET voting_epoch_down = $1",
        voting_epoch_down,
    )
    .execute(&mut tx)
    .await?;
    for epoch_gauge in epoch_gauges.iter() {
        let (pubkey, _bump) = Pubkey::find_program_address(
            &[
                b"EpochGauge".as_ref(),
                epoch_gauge.gauge.as_ref(),
                epoch_gauge.voting_epoch.to_le_bytes().as_ref(),
            ],
            &gauge::id(),
        );
        sqlx::query!(
            r#"
                insert into epoch_gauge(address, gauge, voting_epoch, total_power, token_a_fee, token_b_fee) values($1, $2, $3, $4, $5, $6)
                ON CONFLICT (address) 
                DO
                    UPDATE SET total_power = $7
            "#,
            pubkey.to_string(),
            epoch_gauge.gauge.to_string(),
            epoch_gauge.voting_epoch as i64, // TODO change type
            epoch_gauge.total_power as i64,// TODO change type
            epoch_gauge.token_a_fee as i64,// TODO change type
            epoch_gauge.token_b_fee as i64,// TODO change type
            epoch_gauge.total_power as i64,// TODO change type
        )
        .execute(&mut tx)
        .await?;
    }

    // insertions become visible to other connections only after this point
    tx.commit().await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Bribe {
    pub address: String,
    pub gauge: String,
    pub token_mint: String,
    pub reward_each_epoch: i64,
    pub briber: String,
    pub token_account_vault: String,
    pub bribe_rewards_epoch_start: i64,
    pub bribe_rewards_epoch_end: i64,
    pub bribe_index: i64,
}

pub async fn get_max_bribe_index(pg_pool: &Pool<Postgres>) -> anyhow::Result<i64> {
    match sqlx::query!(r#"SELECT bribe_index, MAX(bribe_index) FROM bribe GROUP BY bribe_index"#)
        .fetch_optional(pg_pool)
        .await
    {
        Ok(value) => {
            if value.is_none() {
                return Ok(-1);
            }
            value
                .and_then(|res| Some(res.bribe_index))
                .ok_or(anyhow::Error::msg("cannot get max bribe index"))
        }
        Err(err) => match err {
            sqlx::Error::RowNotFound => Ok(-1),
            err => {
                println!("{}", err);
                Err(anyhow::Error::msg("cannot get max bribe index"))
            }
        },
    }
}

pub async fn save_bribe(
    pg_pool: &Pool<Postgres>,
    pubkey: Pubkey,
    bribe: &gauge::Bribe,
) -> anyhow::Result<()> {
    let bribe_rewards_epoch_start: i64 = bribe.bribe_rewards_epoch_start.into();
    let bribe_rewards_epoch_end: i64 = bribe.bribe_rewards_epoch_end.into();
    let bribe_index: i64 = bribe.bribe_index.into();
    let reward_each_epoch: i64 = bribe.reward_each_epoch.try_into()?;
    sqlx::query!(
        r#"
            INSERT INTO bribe (address, gauge, token_mint, reward_each_epoch, briber, token_account_vault, bribe_rewards_epoch_start, bribe_rewards_epoch_end, bribe_index) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)            
        "#,
        pubkey.to_string(),
        bribe.gauge.to_string(),
        bribe.token_mint.to_string(),
        reward_each_epoch,
        bribe.briber.to_string(),
        bribe.token_account_vault.to_string(),
        bribe_rewards_epoch_start,
        bribe_rewards_epoch_end,
        bribe_index,
    )
    .execute(pg_pool)
    .await?;
    Ok(())
}

#[derive(Debug)]
pub struct EpochGauge {
    pub address: String,
    pub gauge: String,
    pub total_power: i64,
    pub token_a_fee: i64,
    pub token_b_fee: i64,
    pub voting_epoch: i64,
}

pub async fn get_epoch_gauges(
    pg_pool: &Pool<Postgres>,
    epoch: i64,
) -> anyhow::Result<Vec<EpochGauge>> {
    let epoch_gauges: Vec<EpochGauge> = sqlx::query_as!(
        EpochGauge,
        r#"SELECT * FROM epoch_gauge WHERE voting_epoch = $1"#,
        epoch
    )
    .fetch_all(pg_pool)
    .await?;
    Ok(epoch_gauges)
}

pub async fn get_bribes(pg_pool: &Pool<Postgres>, epoch: i64) -> anyhow::Result<Vec<Bribe>> {
    let bribes: Vec<Bribe> = sqlx::query_as!(
        Bribe,
        r#"SELECT * FROM bribe WHERE bribe_rewards_epoch_start <= $1 and bribe_rewards_epoch_end >= $2"#,
        epoch,
        epoch,
    )
    .fetch_all(pg_pool)
    .await?;
    Ok(bribes)
}

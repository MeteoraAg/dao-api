use anchor_client::solana_sdk::signature::read_keypair_file;
use anchor_client::solana_sdk::signature::Signature;
use anchor_client::solana_sdk::signer::keypair::Keypair;
use anchor_client::solana_sdk::signer::Signer;
use anchor_client::solana_sdk::transaction::Transaction;
use anchor_client::Program;
use anchor_client::RequestBuilder;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::system_program;
use anyhow::{Ok, Result};
use std::sync::Arc;

pub async fn trigger_next_epoch(
    program: &Program<Arc<Keypair>>,
    keypair_url: &str,
    gauge_factory: Pubkey,
) -> Result<()> {
    let builder = program
        .request()
        .accounts(gauge::accounts::TriggerNextEpoch { gauge_factory })
        .args(gauge::instruction::TriggerNextEpoch {});

    let signature = send_tx(keypair_url, program, &builder)?;

    println!("trigger_next_epoch Signature {:?}", signature);
    Ok(())
}

fn send_tx<C: Clone + std::ops::Deref<Target = impl Signer>>(
    keypair_url: &str,
    program: &Program<Arc<Keypair>>,
    builder: &RequestBuilder<C>,
) -> Result<Signature> {
    let payer = read_keypair_file(keypair_url.clone()).expect("Requires a keypair file");
    let rpc_client = program.rpc();
    let ixs = builder.instructions()?;

    let latest_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &builder.instructions()?,
        Some(&payer.pubkey()),
        &[&payer],
        latest_blockhash,
    );

    let signature = rpc_client.send_and_confirm_transaction(&tx)?;
    Ok(signature)
}

fn simulation_tx<C: Clone + std::ops::Deref<Target = impl Signer>>(
    keypair_url: &str,
    program: &Program<Arc<Keypair>>,
    builder: &RequestBuilder<C>,
) -> Result<()> {
    let payer = read_keypair_file(keypair_url.clone()).expect("Requires a keypair file");
    let rpc_client = program.rpc();
    let ixs = builder.instructions()?;

    let latest_blockhash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &builder.instructions()?,
        Some(&payer.pubkey()),
        &[&payer],
        latest_blockhash,
    );

    let simulation = rpc_client.simulate_transaction(&tx)?;
    println!("{:?}", simulation);
    Ok(())
}

pub async fn sync_gauge(
    program: &Program<Arc<Keypair>>,
    keypair_url: &str,
    gauge_factory: Pubkey,
    gauge_pk: Pubkey,
    voting_epoch: u32,
    is_simulation: bool,
) -> Result<()> {
    let (epoch_gauge, _bump) = Pubkey::find_program_address(
        &[
            b"EpochGauge".as_ref(),
            gauge_pk.as_ref(),
            voting_epoch.to_le_bytes().as_ref(),
        ],
        &gauge::id(),
    );

    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory).await?;
    let gauge_state: gauge::Gauge = program.account(gauge_pk).await?;

    let builder = program
        .request()
        .accounts(gauge::accounts::SyncGauge {
            gauge_factory,
            gauge: gauge_pk,
            epoch_gauge,
            quarry: gauge_state.quarry,
            rewarder: gauge_factory_state.rewarder,
            quarry_program: quarry::id(),
        })
        .args(gauge::instruction::SyncGauge {});

    if is_simulation {
        simulation_tx(keypair_url, program, &builder)?;
    } else {
        let signature = send_tx(keypair_url, program, &builder)?;
        println!("sync_gauge {} Signature {:?}", gauge_pk, signature);
    }

    Ok(())
}

pub async fn create_epoch_gauge(
    program: &Program<Arc<Keypair>>,
    keypair_url: &str,
    gauge_factory: Pubkey,
    gauge_pk: Pubkey,
) -> Result<()> {
    let gauge_factory_state: gauge::GaugeFactory = program.account(gauge_factory).await?;
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

    let gauge_state: gauge::Gauge = program.account(gauge_pk).await?;

    let builder = program
        .request()
        .accounts(gauge::accounts::CreateEpochGauge {
            gauge_factory,
            gauge: gauge_pk,
            epoch_gauge,
            amm_pool: gauge_state.amm_pool,
            token_a_fee: gauge_state.token_a_fee_key,
            token_b_fee: gauge_state.token_b_fee_key,
            payer: program.payer(),
            system_program: system_program::id(),
        })
        .args(gauge::instruction::CreateEpochGauge {});

    let signature = send_tx(keypair_url, program, &builder)?;

    // let signature = builder.send().await?;
    println!(
        "create_epoch_gauge gauge: {} epoch: {} Signature: {:?}",
        gauge_pk, gauge_factory_state.current_voting_epoch, signature
    );
    Ok(())
}

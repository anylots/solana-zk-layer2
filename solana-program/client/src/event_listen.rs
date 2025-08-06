use anchor_client::{Client, ClientError, Cluster};
use anchor_lang::prelude::*;
use anyhow::Result;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::PROGRAM_ID;

#[derive(Debug, Clone)]
#[event]
pub struct DepositEvent {
    pub sender: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct EventData {
    pub event: DepositEvent,
    pub slot: u64,
}

/// Create an event listener and return the receiver
pub async fn create_listener(
    rpc_url: String,
    ws_url: String,
) -> Result<mpsc::UnboundedReceiver<EventData>, ClientError> {
    let (tx, rx) = mpsc::unbounded_channel();

    let program_id: Pubkey = PROGRAM_ID
        .parse()
        .map_err(|e| ClientError::LogParseError(format!("Invalid program ID: {}", e)))?;

    let payer = solana_sdk::signature::Keypair::new();
    let client = Client::new_with_options(
        Cluster::Custom(rpc_url, ws_url),
        Arc::new(payer),
        CommitmentConfig::processed(),
    );
    let program = client.program(program_id).unwrap();

    log::info!("Starting Event listener");
    let _unsubscriber = program.on::<DepositEvent>(move |ctx, event| {
        let event_data = EventData {
            event: event.clone(),
            slot: ctx.slot,
        };
        if tx.send(event_data).is_err() {
            log::info!("Receiver is turned off");
        }
    })?;
    

    Ok(rx)
}

// use example: cargo test test_create_listener -- --nocapture
#[tokio::test]
async fn test_create_listener() -> Result<()> {
    let mut receiver = create_listener(
        "http://127.0.0.1:8899".to_string(),
        "ws://127.0.0.1:8900".to_string(),
    )
    .await?;

    while let Some(event_data) = receiver.recv().await {
        println!(
            "Received event: {} lamports from {}",
            event_data.event.amount, event_data.event.sender
        );
        // Do anything
    }

    Ok(())
}

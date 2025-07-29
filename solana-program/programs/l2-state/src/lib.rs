#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;

mod error;
mod verifier;
mod state;

declare_id!("9RrUP9zNimDPVeoP47zJAAMnWahf7geUuWgcv3XMCzGq");

#[program]
pub mod l2_state {
    use crate::verifier::{prove_batch, Groth16Proof};

    use super::*;

    // Init storage pda.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let batch_storage = &mut ctx.accounts.batch_storage;
        batch_storage.authority = ctx.accounts.authority.key();
        batch_storage.batches = Vec::new();
        
        let latest_batch_index = &mut ctx.accounts.latest_batch_index;
        latest_batch_index.authority = ctx.accounts.authority.key();
        latest_batch_index.latest_index = 0;
        
        msg!("Batch storage and latest batch index initialized");
        Ok(())
    }

    // Commit batch, use solana network as DA.
    pub fn commit_batch(ctx: Context<CommitBatch>, batch_info: BatchInfo) -> Result<[u8; 32]> {
        msg!("Committing batch number: {}", batch_info.batch_index);
        msg!("Number of blocks in batch: {}", batch_info.blocks.len());

        let batch_hash = hash_nested_vector(&batch_info.blocks);

        // Create BatchData to store
        let batch_data = BatchData {
            batch_index: batch_info.batch_index,
            batch_hash,
            prev_state_root: batch_info.prev_state_root,
            post_state_root: batch_info.post_state_root,
        };

        let batch_storage = &mut ctx.accounts.batch_storage;

        // Check if batch already exists and update, otherwise append
        if let Some(existing_batch) = batch_storage
            .batches
            .iter_mut()
            .find(|b| b.batch_index == batch_info.batch_index)
        {
            *existing_batch = batch_data;
        } else {
            batch_storage.batches.push(batch_data);
        }

        // Update latest batch index
        let latest_batch_index = &mut ctx.accounts.latest_batch_index;
        if batch_info.batch_index > latest_batch_index.latest_index {
            latest_batch_index.latest_index = batch_info.batch_index;
        }

        msg!(
            "Batch {} committed with hash: {:?}",
            batch_info.batch_index,
            batch_hash
        );

        Ok(batch_hash)
    }

    // Prove that the state transition of the specified batch is valid
    pub fn prove_state(ctx: Context<ProveState>, batch_proof: BatchProof) -> Result<()> {
        let storage = &ctx.accounts.batch_storage.batches;
        let batch_index = batch_proof.batch_index;

        let batch = storage
            .iter()
            .find(|b| b.batch_index == batch_index)
            .ok_or(Error::from(error::ErrorCode::BatchNotExist))?;

        // Calculate the commitment of publicInput
        let pi_hash = hash_nested_vector(&vec![
            batch.prev_state_root.to_vec(),
            batch.post_state_root.to_vec(),
            batch.batch_hash.to_vec(),
        ]);

        let groth16_proof = Groth16Proof {
            proof: batch_proof.proof,
            public_inputs: pi_hash.to_vec(),
        };

        prove_batch(groth16_proof)?;
        Ok(())
    }

    pub fn get_committed_batch(
        ctx: Context<GetCommittedBatch>,
        batch_index: u64,
    ) -> Result<Option<BatchData>> {
        let storage = &ctx.accounts.batch_storage.batches;
        if let Some(batch) = storage.iter().find(|b| b.batch_index == batch_index) {
            return Ok(Some(batch.clone()));
        } else {
            return Ok(None);
        }
    }

    pub fn get_latest_batch_index(ctx: Context<GetLatestBatchIndex>) -> Result<u64> {
        let latest_batch_index = &ctx.accounts.latest_batch_index;
        Ok(latest_batch_index.latest_index)
    }
}

pub fn hash_nested_vector(data: &Vec<Vec<u8>>) -> [u8; 32] {
    if data.is_empty() {
        return [0u8; 32];
    }
    let concatenated_data = data.concat();
    hash(&concatenated_data).to_bytes()
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchInfo {
    pub batch_index: u64,
    pub blocks: Vec<Vec<u8>>,
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchData {
    pub batch_index: u64,
    pub batch_hash: [u8; 32],
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
}

#[account]
pub struct BatchStorage {
    pub authority: Pubkey,
    pub batches: Vec<BatchData>,
}

impl Space for BatchStorage {
    const INIT_SPACE: usize = 32 + 4 + 0; // authority + vec length + initial empty vec
}

#[account]
pub struct LatestBatchIndex {
    pub authority: Pubkey,
    pub latest_index: u64,
}

impl Space for LatestBatchIndex {
    const INIT_SPACE: usize = 32 + 8; // authority + u64
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + BatchStorage::INIT_SPACE,
        seeds = [b"batch_storage"],
        bump
    )]
    pub batch_storage: Account<'info, BatchStorage>,
    #[account(
        init,
        payer = authority,
        space = 8 + LatestBatchIndex::INIT_SPACE,
        seeds = [b"latest_batch_index"],
        bump
    )]
    pub latest_batch_index: Account<'info, LatestBatchIndex>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(batch_info: BatchInfo)]
pub struct CommitBatch<'info> {
    #[account(
        mut,
        seeds = [b"batch_storage"],
        bump,
        has_one = authority,
        realloc = 8 + 32 + 4 + batch_storage.batches.len().saturating_add(1).saturating_mul(104),
        realloc::payer = authority,
        realloc::zero = false,
    )]
    pub batch_storage: Account<'info, BatchStorage>,
    #[account(
        mut,
        seeds = [b"latest_batch_index"],
        bump,
        has_one = authority,
    )]
    pub latest_batch_index: Account<'info, LatestBatchIndex>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetCommittedBatch<'info> {
    #[account(
        seeds = [b"batch_storage"],
        bump,
    )]
    pub batch_storage: Account<'info, BatchStorage>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchProof {
    pub batch_index: u64,
    pub proof: Vec<u8>,
}

#[derive(Accounts)]
pub struct ProveState<'info> {
    #[account(
        seeds = [b"batch_storage"],
        bump,
    )]
    pub batch_storage: Account<'info, BatchStorage>,
}

#[derive(Accounts)]
pub struct GetLatestBatchIndex<'info> {
    #[account(
        seeds = [b"latest_batch_index"],
        bump,
    )]
    pub latest_batch_index: Account<'info, LatestBatchIndex>,
}

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

use crate::{
    bridge::{BridgeVault, FinalizedWithdrawalRoots, FinalizedWithdrawals},
    util::hash_nested_vector,
};

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                          STATE IMPL                        */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

/// Impl of initialize storage PDA.
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    // Batch (blocks) PDA.
    let batch_storage = &mut ctx.accounts.batch_storage;
    batch_storage.authority = ctx.accounts.authority.key();
    batch_storage.batches = Vec::new();

    // Last finalized batch index PDA.
    let last_finalized = &mut ctx.accounts.last_finalized;
    last_finalized.authority = ctx.accounts.authority.key();
    last_finalized.batch_index = 0;

    // Bridge vault PDA
    let bridge_vault = &mut ctx.accounts.bridge_vault;
    bridge_vault.authority = ctx.accounts.authority.key();
    bridge_vault.balances = Vec::new();

    // Withdrawal roots PDA
    let withdrawal_roots = &mut ctx.accounts.withdrawal_roots;
    withdrawal_roots.authority = ctx.accounts.authority.key();
    withdrawal_roots.withdrawal_roots = Vec::new();

    // Finalized withdrawals PDA
    let withdrawals = &mut ctx.accounts.withdrawals;
    withdrawals.authority = ctx.accounts.authority.key();
    withdrawals.finalized_withdrawals = Vec::new();

    msg!("Batch storage and last_finalized batch index initialized");
    Ok(())
}

// Impl of commit batch.
pub fn commit_batch(ctx: Context<CommitBatch>, batch_info: BatchInfo) -> Result<[u8; 32]> {
    msg!("Committing batch number: {}", batch_info.batch_index);
    msg!("Number of blocks in batch: {}", batch_info.blocks.len());

    let batch_hash = hash_nested_vector(&batch_info.blocks);

    // Create BatchData to store
    let batch_data = BatchData {
        batch_index: batch_info.batch_index,
        start_block_num: batch_info.start_block_num,
        end_block_num: batch_info.end_block_num,
        batch_hash,
        prev_state_root: batch_info.prev_state_root,
        post_state_root: batch_info.post_state_root,
        withdrawal_root: batch_info.withdrawal_root,
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

    msg!(
        "Batch {} committed with hash: {:?}",
        batch_info.batch_index,
        batch_hash
    );

    Ok(batch_hash)
}

/// Impl of get committed batch
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

/// Impl of get latest batch
pub fn get_latest_batch(ctx: Context<GetCommittedBatch>) -> Result<Option<BatchData>> {
    let storage = &ctx.accounts.batch_storage.batches;
    if let Some(batch) = storage.last() {
        return Ok(Some(batch.clone()));
    } else {
        return Ok(None);
    }
}

/// Impl of get batch index
pub fn get_last_finalized_batch_index(ctx: Context<GetLatestFinalizedBatchIndex>) -> Result<u64> {
    let last_finalized = &ctx.accounts.last_finalized;
    Ok(last_finalized.batch_index)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchInfo {
    pub batch_index: u64,
    // Only saved in calldata
    pub blocks: Vec<Vec<u8>>,
    pub start_block_num: u64,
    pub end_block_num: u64,
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
    pub withdrawal_root: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchData {
    pub batch_index: u64,
    pub start_block_num: u64,
    pub end_block_num: u64,
    pub batch_hash: [u8; 32],
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
    pub withdrawal_root: [u8; 32],
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
pub struct LastFinalizedBatchIndex {
    pub authority: Pubkey,
    pub batch_index: u64,
}

impl Space for LastFinalizedBatchIndex {
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
        space = 8 + LastFinalizedBatchIndex::INIT_SPACE,
        seeds = [b"last_finalized_batch_index"],
        bump
    )]
    pub last_finalized: Account<'info, LastFinalizedBatchIndex>,
    #[account(
        init,
        payer = authority,
        space = 8 + BridgeVault::INIT_SPACE,
        seeds = [b"bridge_vault"],
        bump
    )]
    pub bridge_vault: Account<'info, BridgeVault>,
    #[account(
        init,
        payer = authority,
        space = 8 + BridgeVault::INIT_SPACE,
        seeds = [b"finalized_withdrawal_roots"],
        bump,
    )]
    pub withdrawal_roots: Account<'info, FinalizedWithdrawalRoots>,
    #[account(
        init,
        payer = authority,
        space = 8 + BridgeVault::INIT_SPACE,
        seeds = [b"finalized_withdrawals"],
        bump,
    )]
    pub withdrawals: Account<'info, FinalizedWithdrawals>,
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
        realloc = 8 + 32 + 4 + batch_storage.batches.len().saturating_add(1).saturating_mul(152),
        realloc::payer = authority,
        realloc::zero = false,
    )]
    pub batch_storage: Account<'info, BatchStorage>,
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

#[derive(Accounts)]
pub struct GetLatestFinalizedBatchIndex<'info> {
    #[account(
        seeds = [b"last_finalized_batch_index"],
        bump,
    )]
    pub last_finalized: Account<'info, LastFinalizedBatchIndex>,
}

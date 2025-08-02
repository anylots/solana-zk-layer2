#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

mod biz_error;
mod bridge;
mod state;
mod util;
mod verifier;

use crate::bridge::*;
use crate::state::*;
use crate::verifier::*;

declare_id!("9RrUP9zNimDPVeoP47zJAAMnWahf7geUuWgcv3XMCzGq");

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                    L2 STATE MAIN ENTRANCE                  */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

#[program]
pub mod l2_state {
    use super::*;

    /// Initialize program for l2 state.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    ///
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        state::initialize(ctx)
    }

    /// Commit batch, use solana network as DA.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `batch_info` - The batch information to commit
    ///
    pub fn commit_batch(ctx: Context<CommitBatch>, batch_info: BatchInfo) -> Result<[u8; 32]> {
        state::commit_batch(ctx, batch_info)
    }

    /// Prove that the state transition of the specified batch is valid.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `batch_proof` - The batch proof to verify
    ///
    pub fn prove_state(ctx: Context<ProveState>, batch_proof: BatchProof) -> Result<()> {
        verifier::prove_state(ctx, batch_proof)
    }

    /// Get committed batch by index.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `batch_index` - The index of the batch to retrieve
    ///
    pub fn get_committed_batch(
        ctx: Context<GetCommittedBatch>,
        batch_index: u64,
    ) -> Result<Option<BatchData>> {
        state::get_committed_batch(ctx, batch_index)
    }

    /// Get the latest committed batch.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    ///
    pub fn get_latest_batch(ctx: Context<GetCommittedBatch>) -> Result<Option<BatchData>> {
        state::get_latest_batch(ctx)
    }

    /// Get the index of the last finalized batch.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    ///
    pub fn get_last_finalized_batch_index(
        ctx: Context<GetLatestFinalizedBatchIndex>,
    ) -> Result<u64> {
        state::get_last_finalized_batch_index(ctx)
    }

    /// Deposit native token (sol).
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `amount` - The amount of deposit.
    ///
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        bridge::deposit(ctx, amount)
    }

    /// Withdraw native token (sol).
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context of accounts
    /// * `amount` - The amount of withdrawal.
    ///
    pub fn withdrawal(ctx: Context<Withdrawal>, amount: u64) -> Result<()> {
        bridge::withdrawal(ctx, amount)
    }
}

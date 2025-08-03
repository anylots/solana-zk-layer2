#![allow(unexpected_cfgs)]

use anchor_lang::prelude::borsh::{BorshDeserialize, BorshSerialize};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{entrypoint::ProgramResult, msg, program_error::ProgramError};
use sp1_solana::verify_proof;

use crate::biz_error;
use crate::bridge::FinalizedWithdrawalRoots;
use crate::state::{BatchStorage, LastFinalizedBatchIndex};
use crate::util::hash_nested_vector;

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                        ZKP VERIFIER IMPL                   */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

// Represents the commitment of the layer2 verification circuit
const LAYER2_VKEY_HASH: &str = "0x00bb9e57314d7ee4f65a4b9fb46fbeae0495f2015c5a8a737333680ce6bb424e";

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Groth16Proof {
    pub proof: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

// Impl of prove state
pub fn prove_state(ctx: Context<ProveState>, batch_proof: BatchProof) -> Result<()> {
    let storage = &ctx.accounts.batch_storage.batches;
    let batch_index = batch_proof.batch_index;

    let batch = storage
        .iter()
        .find(|b| b.batch_index == batch_index)
        .ok_or(Error::from(biz_error::ErrorCode::BatchNotExist))?;

    // Calculate the commitment of publicInput
    let pi_hash = hash_nested_vector(&vec![
        batch.prev_state_root.to_vec(),
        batch.post_state_root.to_vec(),
        batch.withdrawal_root.to_vec(),
        batch.batch_hash.to_vec(),
    ]);

    let groth16_proof = Groth16Proof {
        proof: batch_proof.proof,
        public_inputs: pi_hash.to_vec(),
    };

    prove_batch(groth16_proof)?;

    // Update last_finalized_batch_index
    let last_finalized = &mut ctx.accounts.last_finalized;
    last_finalized.batch_index = batch_index;

    // Set withdrawal_root finalized
    let withdrawal_roots = &mut ctx.accounts.withdrawal_roots;
    withdrawal_roots.set_finalized(batch.withdrawal_root, true);

    Ok(())
}

/// prove state for batch
fn prove_batch(groth16_proof: Groth16Proof) -> ProgramResult {
    let vk = sp1_solana::GROTH16_VK_5_0_0_BYTES;

    // Verify the proof.
    verify_proof(
        &groth16_proof.proof,
        &groth16_proof.public_inputs,
        &LAYER2_VKEY_HASH,
        vk,
    )
    .map_err(|_| ProgramError::InvalidInstructionData)?;

    // Print out the public values.
    let mut reader = groth16_proof.public_inputs.as_slice();
    let n = u32::deserialize(&mut reader).unwrap();
    let a = u32::deserialize(&mut reader).unwrap();
    let b = u32::deserialize(&mut reader).unwrap();
    msg!("Public values: (n: {}, a: {}, b: {})", n, a, b);

    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchProof {
    pub batch_index: u64,
    pub proof: Vec<u8>,
}

#[derive(Accounts)]
pub struct ProveState<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        seeds = [b"batch_storage"],
        bump,
    )]
    pub batch_storage: Account<'info, BatchStorage>,
    #[account(
        seeds = [b"last_finalized_batch_index"],
        bump,
    )]
    pub last_finalized: Account<'info, LastFinalizedBatchIndex>,
    #[account(
        seeds = [b"finalized_withdrawal_roots"],
        bump,
        mut,
        realloc = 8 + 32 + 4 + withdrawal_roots.withdrawal_roots.len().saturating_add(1).saturating_mul(40),
        realloc::payer = sender,
        realloc::zero = false,
    )]
    pub withdrawal_roots: Account<'info, FinalizedWithdrawalRoots>,
    pub system_program: Program<'info, System>,
}

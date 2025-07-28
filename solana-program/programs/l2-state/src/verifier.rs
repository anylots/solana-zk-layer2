#![allow(unexpected_cfgs)]

use anchor_lang::solana_program::{entrypoint::ProgramResult, msg, program_error::ProgramError};
use borsh::{BorshDeserialize, BorshSerialize};
use sp1_solana::verify_proof;

// Represents the commitment of the layer2 verification circuit
const LAYER2_VKEY_HASH: &str = "0x00bb9e57314d7ee4f65a4b9fb46fbeae0495f2015c5a8a737333680ce6bb424e";

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Groth16Proof {
    pub proof: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

pub fn prove_batch(groth16_proof: Groth16Proof) -> ProgramResult {
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

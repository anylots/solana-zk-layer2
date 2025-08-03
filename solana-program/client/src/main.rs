use anchor_lang::prelude::*;
use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, pubkey::Pubkey, signature::Keypair, signer::Signer, system_program,
    transaction::Transaction,
};

// Import from this lib
use l2_state_client::{hash_nested_vector, BatchInfo, L2StateClient};

fn main() -> Result<()> {
    // Use a fixed keypair.
    let fee_payer = &[
        174, 47, 154, 16, 202, 193, 206, 113, 199, 190, 53, 133, 169, 175, 31, 56, 222, 53, 138,
        189, 224, 216, 117, 173, 10, 149, 53, 45, 73, 251, 237, 246, 15, 185, 186, 82, 177, 240,
        148, 69, 241, 227, 167, 80, 141, 89, 240, 121, 121, 35, 172, 247, 68, 251, 226, 218, 48,
        63, 176, 109, 168, 89, 238, 135,
    ];
    // Create connection to local validator
    let client = L2StateClient::new(String::from("http://localhost:8899"), fee_payer)?;

    // Airdrop 1 SOL to fee payer
    client.request_airdrop(1_000_000_000)?;

    // Call the initialize_batch_storage function
    println!("------------> Start call the initialize function");
    client.initialize()?;

    // Call the commit_committed_batch function
    println!("------------> Start call the commit_batch function");
    let batch_info = BatchInfo {
        batch_index: 1,
        blocks: vec![
            vec![1, 2, 3, 4, 5],
            vec![6, 7, 8, 9, 10],
            vec![11, 12, 13, 14, 15],
        ],
        start_block_num: 101,
        end_block_num: 201,
        prev_state_root: [0u8; 32],
        post_state_root: [1u8; 32],
        withdrawal_root: [3u8; 32],
    };

    let blocks_hash = hash_nested_vector(&batch_info.blocks);
    println!("blocks_hash caculate offchain: {:?}", blocks_hash);
    client.commit_batch(batch_info)?;

    // Call the get_batch function
    println!("------------> Start call the get_batch function");
    let onchain_batch = client.get_committed_batch(1)?.unwrap();
    println!("onchain_batch: {:?}", onchain_batch);

    // Call the latest_batch function
    println!("------------> Start call the get_latest_batch function");
    let latest_batch = client.get_latest_batch()?;
    println!("latest_batch: {:?}", latest_batch);

    // Call the get_latest_batch_index function
    println!("------------> Start call the get_last_finalized_batch_index function");
    let latest_batch_index = client.get_last_finalized_batch_index()?;
    println!("last_finalized_batch_index: {:?}", latest_batch_index);

    Ok(())
}

pub fn initialize(client: &RpcClient, fee_payer: &Keypair, program_id: &Pubkey) -> Result<()> {
    let instruction_data: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(
                Pubkey::find_program_address(&[b"batch_storage"], program_id).0,
                false,
            ),
            AccountMeta::new(
                Pubkey::find_program_address(&[b"latest_batch_index"], program_id).0,
                false,
            ),
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data.to_vec(),
    };
    let recent_blockhash = client.get_latest_blockhash()?;
    // Create and sign the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&fee_payer.pubkey()),
        &[&fee_payer],
        recent_blockhash,
    );

    // Send the transaction
    let signature = client.send_and_confirm_transaction(&transaction)?;
    println!("initialize Transaction signature: {}", signature);

    Ok(())
}

use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anyhow::Result;
use base64::{self, engine::general_purpose, Engine};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey,
    signature::Keypair, signer::Signer, system_program, transaction::Transaction,
};

// Program ID from the Anchor.toml
const PROGRAM_ID: &str = "C5bPpisqFtj8oCUd6MFi648pZhpXXVLm1qmvtj21iS3Y";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchInfo {
    pub batch_index: u64,
    pub blocks: Vec<Vec<u8>>,
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BatchData {
    pub batch_index: u64,
    pub batch_hash: [u8; 32],
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
}

fn call_commit_batch(client: &RpcClient, fee_payer: &Keypair, program_id: &Pubkey) -> Result<()> {
    // Create the commit_batch instruction
    // The discriminator for "commit_batch" is provided
    let discriminator: [u8; 8] = [27, 234, 100, 224, 134, 31, 168, 142];

    // Create sample BatchInfo data
    let batch_info = BatchInfo {
        batch_index: 1,
        blocks: vec![
            vec![1, 2, 3, 4, 5],
            vec![6, 7, 8, 9, 10],
            vec![11, 12, 13, 14, 15],
        ],
        prev_state_root: [0u8; 32],
        post_state_root: [1u8; 32],
    };

    let blocks_hash = hash_nested_vector(&batch_info.blocks);
    println!("blocks_hash caculate offchain: {:?}", blocks_hash);

    // Serialize the BatchInfo
    let mut instruction_data = discriminator.to_vec();
    let serialized_batch_info = batch_info.try_to_vec()?;
    instruction_data.extend_from_slice(&serialized_batch_info);

    // Create the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(
                Pubkey::find_program_address(&[b"batch_storage"], program_id).0,
                false,
            ),
            AccountMeta::new(fee_payer.pubkey(), true), // authority must be mutable for realloc
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: instruction_data,
    };

    // Get recent blockhash
    let recent_blockhash = client.get_latest_blockhash()?;

    // Create and sign the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&fee_payer.pubkey()),
        &[fee_payer],
        recent_blockhash,
    );

    // Send the transaction
    let signature = client.send_and_confirm_transaction(&transaction)?;
    println!("call_commit_batch Transaction signature: {}", signature);

    Ok(())
}

pub fn hash_nested_vector(data: &Vec<Vec<u8>>) -> [u8; 32] {
    if data.is_empty() {
        return [0u8; 32];
    }
    let concatenated_data = data.concat();
    hash(&concatenated_data).to_bytes()
}

fn call_get_committed_batch(
    client: &RpcClient,
    fee_payer: &Keypair,
    program_id: &Pubkey,
) -> Result<()> {
    let discriminator: [u8; 8] = [246, 70, 81, 64, 254, 87, 48, 173];
    let mut instruction_data = discriminator.to_vec();
    instruction_data.extend_from_slice(&1u64.try_to_vec().unwrap());
    // Create the instruction
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(
            Pubkey::find_program_address(&[b"batch_storage"], program_id).0,
            false,
        )],
        data: instruction_data,
    };

    // Get recent blockhash
    let recent_blockhash = client.get_latest_blockhash()?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&fee_payer.pubkey()),
        &[fee_payer],
        recent_blockhash,
    );

    // Send the transaction
    let result = client.simulate_transaction(&transaction)?;

    if let Some(err) = result.value.err {
        println!("Transaction simulation failed: {:?}", err);
        return Ok(());
    }
    println!("Get get_batch transaction result: {:?}", result);

    // Extract return value from the simulation result
    if let Some(return_data) = &result.value.return_data {
        let (data, _encoding) = &return_data.data;
        // The return data is base64 encoded, decode it
        if let Ok(decoded_data) = general_purpose::STANDARD.decode(data) {
            // Deserialize to Option<BatchData>
            match Option::<BatchData>::try_from_slice(&decoded_data) {
                Ok(batch_data_option) => match batch_data_option {
                    Some(batch_data) => {
                        println!("Successfully retrieved batch data:");
                        println!("  Batch number: {}", batch_data.batch_index);
                        println!("  Batch hash: {:?}", batch_data.batch_hash);
                        println!("  Previous state root: {:?}", batch_data.prev_state_root);
                        println!("  Post state root: {:?}", batch_data.post_state_root);
                    }
                    None => {
                        println!("No batch found for the given index");
                    }
                },
                Err(e) => {
                    println!("Failed to deserialize batch data: {:?}", e);
                    println!("Raw decoded data length: {}", decoded_data.len());
                    println!("Raw decoded data: {:?}", decoded_data);
                }
            }
        } else {
            println!("Failed to decode return data from base64");
        }
    } else {
        println!("No return data in simulation result");
    }

    Ok(())
}

fn main() -> Result<()> {
    // Create connection to local validator
    let client = RpcClient::new_with_commitment(
        String::from("http://localhost:8899"),
        CommitmentConfig::confirmed(),
    );

    // Use a fixed keypair.
    let fee_payer = Keypair::from_bytes(&[
        174, 47, 154, 16, 202, 193, 206, 113, 199, 190, 53, 133, 169, 175, 31, 56, 222, 53, 138,
        189, 224, 216, 117, 173, 10, 149, 53, 45, 73, 251, 237, 246, 15, 185, 186, 82, 177, 240,
        148, 69, 241, 227, 167, 80, 141, 89, 240, 121, 121, 35, 172, 247, 68, 251, 226, 218, 48,
        63, 176, 109, 168, 89, 238, 135,
    ])?;

    // Airdrop 1 SOL to fee payer
    let airdrop_signature = client.request_airdrop(&fee_payer.pubkey(), 1_000_000_000)?;
    client.confirm_transaction(&airdrop_signature)?;

    loop {
        let confirmed = client.confirm_transaction(&airdrop_signature)?;
        if confirmed {
            break;
        }
    }

    // Parse the program ID
    let program_id = PROGRAM_ID.parse::<Pubkey>()?;

    // Call the initialize_batch_storage function
    println!("------------> Start call the initialize function");
    initialize(&client, &fee_payer, &program_id)?;

    // Call the commit_committed_batch function
    println!("------------> Start call the commit_batch function");
    call_commit_batch(&client, &fee_payer, &program_id)?;

    // Call the get_batch function
    println!("------------> Start call the get_batch function");
    call_get_committed_batch(&client, &fee_payer, &program_id)?;

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
    println!(
        "initialize_batch_storage Transaction signature: {}",
        signature
    );

    Ok(())
}

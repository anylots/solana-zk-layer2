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
const PROGRAM_ID: &str = "9RrUP9zNimDPVeoP47zJAAMnWahf7geUuWgcv3XMCzGq";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchInfo {
    pub batch_index: u64,
    pub blocks: Vec<Vec<u8>>,
    pub start_block_num: u64,
    pub end_block_num: u64,
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
    pub withdrawal_root: [u8; 32],
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct BatchProof {
    pub batch_index: u64,
    pub proof: Vec<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct BatchData {
    pub batch_index: u64,
    pub start_block_num: u64,
    pub end_block_num: u64,
    pub batch_hash: [u8; 32],
    pub prev_state_root: [u8; 32],
    pub post_state_root: [u8; 32],
    pub withdrawal_root: [u8; 32],
}

pub struct L2StateClient {
    client: RpcClient,
    fee_payer: Keypair,
    program_id: Pubkey,
}

impl L2StateClient {
    /// Create a new L2StateClient instance
    pub fn new(rpc_url: String, fee_payer_bytes: &[u8]) -> Result<Self> {
        let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
        let fee_payer = Keypair::from_bytes(fee_payer_bytes)?;
        let program_id = PROGRAM_ID.parse::<Pubkey>()?;

        Ok(Self {
            client,
            fee_payer,
            program_id,
        })
    }

    /// Create a new L2StateClient with default local validator settings
    pub fn new_local() -> Result<Self> {
        let default_keypair = [
            174, 47, 154, 16, 202, 193, 206, 113, 199, 190, 53, 133, 169, 175, 31, 56, 222, 53,
            138, 189, 224, 216, 117, 173, 10, 149, 53, 45, 73, 251, 237, 246, 15, 185, 186, 82,
            177, 240, 148, 69, 241, 227, 167, 80, 141, 89, 240, 121, 121, 35, 172, 247, 68, 251,
            226, 218, 48, 63, 176, 109, 168, 89, 238, 135,
        ];

        Self::new("http://localhost:8899".to_string(), &default_keypair)
    }

    /// Initialize the batch storage (should be called once)
    pub fn initialize(&self) -> Result<()> {
        let instruction_data: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(
                    Pubkey::find_program_address(&[b"batch_storage"], &self.program_id).0,
                    false,
                ),
                AccountMeta::new(
                    Pubkey::find_program_address(
                        &[b"last_finalized_batch_index"],
                        &self.program_id,
                    )
                    .0,
                    false,
                ),
                AccountMeta::new(
                    Pubkey::find_program_address(&[b"bridge_vault"], &self.program_id).0,
                    false,
                ),
                AccountMeta::new(
                    Pubkey::find_program_address(
                        &[b"finalized_withdrawal_roots"],
                        &self.program_id,
                    )
                    .0,
                    false,
                ),
                AccountMeta::new(
                    Pubkey::find_program_address(&[b"finalized_withdrawals"], &self.program_id).0,
                    false,
                ),
                AccountMeta::new(self.fee_payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data.to_vec(),
        };

        let recent_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        log::info!("Initialize transaction signature: {}", signature);

        Ok(())
    }

    /// Commit a batch to the Solana program
    pub fn commit_batch(&self, batch_info: BatchInfo) -> Result<()> {
        let discriminator: [u8; 8] = [27, 234, 100, 224, 134, 31, 168, 142];

        let blocks_hash = hash_nested_vector(&batch_info.blocks);
        log::info!("blocks_hash calculated offchain: {:?}", blocks_hash);

        // Serialize the BatchInfo
        let mut instruction_data = discriminator.to_vec();
        let serialized_batch_info = batch_info.try_to_vec()?;
        instruction_data.extend_from_slice(&serialized_batch_info);

        // Create the instruction
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(
                    Pubkey::find_program_address(&[b"batch_storage"], &self.program_id).0,
                    false,
                ),
                AccountMeta::new(self.fee_payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        log::info!("Commit batch transaction signature: {}", signature);

        Ok(())
    }

    /// Prove batch
    pub fn prove_batch(&self, batch_proof: BatchProof) -> Result<()> {
        let discriminator: [u8; 8] = [27, 234, 100, 224, 134, 31, 168, 142];

        // Serialize the BatchInfo
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&batch_proof.try_to_vec()?);

        // Create the instruction
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(
                    Pubkey::find_program_address(&[b"batch_storage"], &self.program_id).0,
                    false,
                ),
                AccountMeta::new(
                    Pubkey::find_program_address(
                        &[b"last_finalized_batch_index"],
                        &self.program_id,
                    )
                    .0,
                    false,
                ),
                AccountMeta::new(self.fee_payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        log::info!("Prove batch transaction signature: {}", signature);

        Ok(())
    }

    /// Get the last finalized batch index
    pub fn get_last_finalized_batch_index(&self) -> Result<u64> {
        let discriminator: [u8; 8] = [224, 183, 118, 226, 186, 198, 245, 187];
        let instruction_data = discriminator.to_vec();

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![AccountMeta::new_readonly(
                Pubkey::find_program_address(&[b"last_finalized_batch_index"], &self.program_id).0,
                false,
            )],
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        let result = self.client.simulate_transaction(&transaction)?;

        if let Some(err) = result.value.err {
            return Err(anyhow::anyhow!("Transaction simulation failed: {:?}", err));
        }

        if let Some(return_data) = &result.value.return_data {
            let (data, _encoding) = &return_data.data;
            if let Ok(decoded_data) = general_purpose::STANDARD.decode(data) {
                match u64::try_from_slice(&decoded_data) {
                    Ok(batch_index) => {
                        log::info!(
                            "Successfully retrieved last finalized batch index: {}",
                            batch_index
                        );
                        return Ok(batch_index);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to deserialize last finalized batch index: {:?}",
                            e
                        ));
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Failed to decode return data from base64"));
            }
        } else {
            return Err(anyhow::anyhow!("No return data in simulation result"));
        }
    }

    /// Get committed batch data by index
    pub fn get_committed_batch(&self, batch_index: u64) -> Result<Option<BatchData>> {
        let discriminator: [u8; 8] = [246, 70, 81, 64, 254, 87, 48, 173];
        let mut instruction_data = discriminator.to_vec();
        instruction_data.extend_from_slice(&batch_index.try_to_vec().unwrap());

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![AccountMeta::new(
                Pubkey::find_program_address(&[b"batch_storage"], &self.program_id).0,
                false,
            )],
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        let result = self.client.simulate_transaction(&transaction)?;

        if let Some(err) = result.value.err {
            return Err(anyhow::anyhow!("Transaction simulation failed: {:?}", err));
        }

        if let Some(return_data) = &result.value.return_data {
            let (data, _encoding) = &return_data.data;
            if let Ok(decoded_data) = general_purpose::STANDARD.decode(data) {
                match Option::<BatchData>::try_from_slice(&decoded_data) {
                    Ok(batch_data_option) => {
                        if let Some(batch_data) = &batch_data_option {
                            log::info!(
                                "Successfully retrieved batch data for index {}: {:?}",
                                batch_index,
                                batch_data
                            );
                        } else {
                            log::info!("No batch found for index {}", batch_index);
                        }
                        return Ok(batch_data_option);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to deserialize batch data: {:?}", e));
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Failed to decode return data from base64"));
            }
        } else {
            return Err(anyhow::anyhow!("No return data in simulation result"));
        }
    }

    /// Get latest batch data
    pub fn get_latest_batch(&self) -> Result<Option<BatchData>> {
        let discriminator: [u8; 8] = [161, 68, 127, 180, 29, 0, 183, 142];
        let instruction_data = discriminator.to_vec();

        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![AccountMeta::new(
                Pubkey::find_program_address(&[b"batch_storage"], &self.program_id).0,
                false,
            )],
            data: instruction_data,
        };

        let recent_blockhash = self.client.get_latest_blockhash()?;
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.fee_payer.pubkey()),
            &[&self.fee_payer],
            recent_blockhash,
        );

        let result = self.client.simulate_transaction(&transaction)?;

        if let Some(err) = result.value.err {
            return Err(anyhow::anyhow!("Transaction simulation failed: {:?}", err));
        }

        if let Some(return_data) = &result.value.return_data {
            let (data, _encoding) = &return_data.data;
            if let Ok(decoded_data) = general_purpose::STANDARD.decode(data) {
                match Option::<BatchData>::try_from_slice(&decoded_data) {
                    Ok(batch_data_option) => {
                        if let Some(batch_data) = &batch_data_option {
                            log::info!("Successfully retrieved latest batch data {:?}", batch_data);
                        } else {
                            log::info!("No latest batch found");
                        }
                        return Ok(batch_data_option);
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to deserialize batch data: {:?}", e));
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Failed to decode return data from base64"));
            }
        } else {
            return Err(anyhow::anyhow!("No return data in simulation result"));
        }
    }

    /// Request airdrop for the fee payer (useful for testing)
    pub fn request_airdrop(&self, amount: u64) -> Result<()> {
        let airdrop_signature = self
            .client
            .request_airdrop(&self.fee_payer.pubkey(), amount)?;
        self.client.confirm_transaction(&airdrop_signature)?;

        loop {
            let confirmed = self.client.confirm_transaction(&airdrop_signature)?;
            if confirmed {
                break;
            }
        }

        log::info!("Airdrop completed: {} lamports", amount);
        Ok(())
    }
}

/// Hash a nested vector of bytes
pub fn hash_nested_vector(data: &Vec<Vec<u8>>) -> [u8; 32] {
    if data.is_empty() {
        return [0u8; 32];
    }
    let concatenated_data = data.concat();
    hash(&concatenated_data).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_nested_vector() {
        let data = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let hash = hash_nested_vector(&data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_empty_hash_nested_vector() {
        let data = vec![];
        let hash = hash_nested_vector(&data);
        assert_eq!(hash, [0u8; 32]);
    }
}

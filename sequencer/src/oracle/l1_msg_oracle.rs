use anyhow::Result;
use l2_state_client::event_listen::{self, EventData};
use share::{utils::read_env_var, DEFAULT_L1_RPC, DEFAULT_L1_WS, L2_SYS_PROGRAM_ID};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey,
    signature::Keypair, signer::Signer, transaction::Transaction,
};
use tokio::sync::mpsc;

pub struct L1MsgOracle {
    client: RpcClient,
    signer: Keypair,
    program_id: Pubkey,
}

impl L1MsgOracle {
    pub fn new(rpc_url: String, signer_key: &str) -> Result<Self> {
        let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
        let signer = Keypair::from_base58_string(signer_key);
        let program_id = Pubkey::from_str_const(L2_SYS_PROGRAM_ID);

        Ok(Self {
            client,
            signer,
            program_id,
        })
    }

    pub async fn listen_deposite_event(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<EventData>();
        let listener_handle = tokio::spawn(async {
            let _ = event_listen::create_listener(
                read_env_var("L1_RPC", DEFAULT_L1_RPC.to_owned()),
                read_env_var("L1_WS", DEFAULT_L1_WS.to_owned()),
                tx,
            )
            .await;
        });
        while let Some(event_data) = rx.recv().await {
            log::info!(
                "Received event: {} lamports from {}",
                event_data.event.amount,
                event_data.event.sender
            );
            let mut param: Vec<u8> = event_data.event.sender.to_bytes().to_vec();
            param.extend_from_slice(&event_data.event.amount.to_be_bytes());
            // Send deposite msg from L1 to L2;
            let _ = self.send_to_layer2(param);
        }
        listener_handle.await?;
        Ok(())
    }

    fn send_to_layer2(&self, param: Vec<u8>) -> Result<()> {
        // create sys ins.
        let instruction = Instruction {
            program_id: self.program_id,
            accounts: vec![],
            data: param,
        };

        // fetch latest block hash.
        let recent_blockhash = self.client.get_latest_blockhash()?;

        // create txn.
        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.signer.pubkey()),
            &[&self.signer],
            recent_blockhash,
        );

        // send txn.
        let signature = self.client.send_and_confirm_transaction(&transaction)?;
        log::info!("Initialize transaction signature: {}", signature);
        Ok(())
    }
}

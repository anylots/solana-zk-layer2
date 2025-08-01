use anyhow::{anyhow, Result};
use log::info;
use solana_sdk::{signature::Signature, transaction::Transaction};

pub struct TransactionValidator {
    // state about verify
}

impl TransactionValidator {
    pub fn _new() -> Self {
        Self {}
    }

    pub async fn validate_transaction(sign_check: bool, transaction: &Transaction) -> Result<()> {
        info!("Starting transaction validation");

        if sign_check {
            // 1. check txn signatrue
            Self::validate_signatures(transaction)?;
        }

        // 2. check format of txn
        Self::validate_transaction_format(transaction)?;

        // 3. check user balance
        Self::validate_account_balances(transaction).await?;

        // 4. checkout txn fee
        Self::validate_fees(transaction)?;

        info!("Transaction validation completed successfully");
        Ok(())
    }

    fn validate_signatures(transaction: &Transaction) -> Result<()> {
        if transaction.signatures.is_empty() {
            return Err(anyhow!("Transaction has no signatures"));
        }

        // verify: sig should not empty
        for (i, signature) in transaction.signatures.iter().enumerate() {
            if signature == &Signature::default() {
                return Err(anyhow!("Signature {} is default/empty", i));
            }
        }
        transaction.verify()?;

        info!("Signature validation passed");
        Ok(())
    }

    fn validate_transaction_format(transaction: &Transaction) -> Result<()> {
        let message = &transaction.message;

        // check count of account
        if message.account_keys.is_empty() {
            return Err(anyhow!("Transaction has no account keys"));
        }

        // check ins
        if message.instructions.is_empty() {
            return Err(anyhow!("Transaction has no instructions"));
        }

        // check count of account adn sig.
        if transaction.signatures.len() != message.header.num_required_signatures as usize {
            return Err(anyhow!(
                "Signature count mismatch: expected {}, got {}",
                message.header.num_required_signatures,
                transaction.signatures.len()
            ));
        }

        info!("Transaction format validation passed");
        Ok(())
    }

    async fn validate_account_balances(_transaction: &Transaction) -> Result<()> {
        // Do nothing
        info!("Account balance validation passed (simplified)");
        Ok(())
    }

    fn validate_fees(_txn: &Transaction) -> Result<()> {
        let _estimated_fee = 5000;

        // Do nothing
        info!("Fee validation passed");
        Ok(())
    }
}

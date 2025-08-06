#![allow(unexpected_cfgs)]

use crate::biz_error;
use crate::util::verify_merkle_proof;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::system_program;

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                         EVENTS                             */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

#[event]
pub struct DepositEvent {
    /// The account that made the deposit
    pub sender: Pubkey,
    /// The amount deposited in lamports
    pub amount: u64,
    /// The new balance of the sender after deposit
    pub new_balance: u64,
    /// Timestamp of the deposit
    pub timestamp: i64,
}

#[event]
pub struct WithdrawalEvent {
    /// The account that initiated the withdrawal
    pub sender: Pubkey,
    /// The account that will receive the funds
    pub to: Pubkey,
    /// The amount withdrawn in lamports
    pub amount: u64,
    /// The new balance of the sender after withdrawal
    pub new_balance: u64,
    /// The withdrawal data hash
    pub withdrawal_hash: [u8; 32],
    /// Timestamp of the withdrawal
    pub timestamp: i64,
}

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                         BRIDGE IMPL                        */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

/// Impl of deposit for native token (sol).
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let from = &ctx.accounts.sender;
    let bridge_vault = &mut ctx.accounts.bridge_vault;
    let program = &ctx.accounts.system_program;

    // Transfer SOL from sender to bridge vault
    system_program::transfer(
        CpiContext::new(
            program.to_account_info(),
            system_program::Transfer {
                from: from.to_account_info(),
                to: bridge_vault.to_account_info(),
            },
        ),
        amount,
    )?;

    // Update balance
    let current_balance = bridge_vault.get_balance(from.key);
    let new_balance = current_balance + amount;
    bridge_vault.set_balance(*from.key, new_balance);

    // Get current timestamp
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp;

    // Emit deposit event
    emit!(DepositEvent {
        sender: *from.key,
        amount,
        new_balance,
        timestamp,
    });

    // Keep the original log message for backward compatibility
    msg!("deposit for account: {:?}, amount: {:?}", from.key, amount);

    Ok(())
}

/// Impl of withdrawal for native token (sol).
pub fn withdrawal(ctx: Context<Withdrawal>, withdrawal: WithdrawalData) -> Result<()> {
    let from = &ctx.accounts.sender;
    let to = &ctx.accounts.to;
    let bridge_vault = &mut ctx.accounts.bridge_vault;
    let amount = withdrawal.amount;
    let withdraw_root = withdrawal.withdraw_root;
    let withdrawal_proof = withdrawal.withdrawal_proof;
    let index = withdrawal.index;

    // Check that this withdrawal has not already been finalized.
    let withdrawal_roots = &mut ctx.accounts.withdrawal_roots;
    if !withdrawal_roots.get_finalized(withdraw_root) {
        return Err(Error::from(
            biz_error::ErrorCode::WithdrawalRootNotFinalized,
        ));
    }

    // Verify that the hash of this withdrawal was stored in the  withdrawal_root.
    let mut withdrawal_data = from.key.to_bytes().to_vec();
    withdrawal_data.extend_from_slice(&to.key.to_bytes());
    withdrawal_data.extend_from_slice(&amount.to_be_bytes());
    withdrawal_data.extend_from_slice(&index.to_be_bytes());
    let withdrawal_data_hash = hash(&withdrawal_data).to_bytes();
    if !verify_merkle_proof(withdrawal_data_hash, withdrawal_proof, index, withdraw_root) {
        return Err(Error::from(
            biz_error::ErrorCode::InvalidWithdrawalInclusionProof,
        ));
    }

    // Check balance available.
    let current_amount = bridge_vault.get_balance(from.key);
    if current_amount < amount {
        return Err(Error::from(biz_error::ErrorCode::UserBalanceInsufficent));
    }

    // Mark the withdrawal as finalized so it can't be replayed.
    let withdrawals = &mut ctx.accounts.withdrawals;
    withdrawals.set_finalized(withdrawal_data_hash, true);

    // Account balance operations
    let new_balance = current_amount - amount;
    bridge_vault.set_balance(*from.key, new_balance);
    bridge_vault.sub_lamports(amount)?;
    to.add_lamports(amount)?;

    // Get current timestamp
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp;

    // Emit withdrawal event
    emit!(WithdrawalEvent {
        sender: *from.key,
        to: to.key(),
        amount,
        new_balance,
        withdrawal_hash: withdrawal_data_hash,
        timestamp,
    });

    Ok(())
}

#[account]
pub struct BridgeVault {
    pub authority: Pubkey,
    pub balances: Vec<(Pubkey, u64)>,
}

impl BridgeVault {
    pub fn get_balance(&self, pubkey: &Pubkey) -> u64 {
        self.balances
            .iter()
            .find(|(key, _)| key == pubkey)
            .map(|(_, balance)| *balance)
            .unwrap_or(0)
    }

    pub fn set_balance(&mut self, pubkey: Pubkey, amount: u64) {
        if let Some(entry) = self.balances.iter_mut().find(|(key, _)| key == &pubkey) {
            entry.1 = amount;
        } else {
            self.balances.push((pubkey, amount));
        }
    }
}

impl Space for BridgeVault {
    const INIT_SPACE: usize = 32 + 4 + 0; // authority + vec length + 0 entries
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    #[account(
        seeds = [b"bridge_vault"],
        bump,
        mut,
        realloc = 8 + 32 + 4 + bridge_vault.balances.len().saturating_add(1).saturating_mul(40),
        realloc::payer = sender,
        realloc::zero = false,
    )]
    pub bridge_vault: Account<'info, BridgeVault>,
    pub system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct WithdrawalData {
    pub amount: u64,
    pub index: u64,
    pub withdraw_root: [u8; 32],
    pub withdrawal_proof: Vec<[u8; 32]>,
}

#[derive(Accounts)]
pub struct Withdrawal<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,
    /// CHECK: Destination account to receive the withdrawn funds
    #[account(mut)]
    pub to: AccountInfo<'info>,
    #[account(
        seeds = [b"bridge_vault"],
        bump,
        mut
    )]
    pub bridge_vault: Account<'info, BridgeVault>,
    #[account(
        seeds = [b"finalized_withdrawal_roots"],
        bump,
    )]
    pub withdrawal_roots: Account<'info, FinalizedWithdrawalRoots>,
    #[account(
        seeds = [b"finalized_withdrawals"],
        bump,
        mut,
        realloc = 8 + 32 + 4 + withdrawals.finalized_withdrawals.len().saturating_add(1).saturating_mul(40),
        realloc::payer = sender,
        realloc::zero = false,
    )]
    pub withdrawals: Account<'info, FinalizedWithdrawals>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct FinalizedWithdrawalRoots {
    pub authority: Pubkey,
    pub withdrawal_roots: Vec<([u8; 32], bool)>,
}

impl FinalizedWithdrawalRoots {
    pub fn get_finalized(&self, withdrawal_root: [u8; 32]) -> bool {
        self.withdrawal_roots
            .iter()
            .find(|(key, _)| key == &withdrawal_root)
            .map(|(_, finalized)| *finalized)
            .unwrap_or(false)
    }

    pub fn set_finalized(&mut self, withdrawal_root: [u8; 32], finalized: bool) {
        if let Some(entry) = self
            .withdrawal_roots
            .iter_mut()
            .find(|(key, _)| key == &withdrawal_root)
        {
            entry.1 = finalized;
        } else {
            self.withdrawal_roots.push((withdrawal_root, finalized));
        }
    }
}

impl Space for FinalizedWithdrawalRoots {
    const INIT_SPACE: usize = 32 + 4 + 0; // authority + vec length + 0 entries
}

#[account]
pub struct FinalizedWithdrawals {
    pub authority: Pubkey,
    pub finalized_withdrawals: Vec<([u8; 32], bool)>,
}

impl FinalizedWithdrawals {
    pub fn get_finalized(&self, withdrawal_data_hash: [u8; 32]) -> bool {
        self.finalized_withdrawals
            .iter()
            .find(|(key, _)| key == &withdrawal_data_hash)
            .map(|(_, finalized)| *finalized)
            .unwrap_or(false)
    }

    pub fn set_finalized(&mut self, withdrawal_data_hash: [u8; 32], finalized: bool) {
        if let Some(entry) = self
            .finalized_withdrawals
            .iter_mut()
            .find(|(key, _)| key == &withdrawal_data_hash)
        {
            entry.1 = finalized;
        } else {
            self.finalized_withdrawals
                .push((withdrawal_data_hash, finalized));
        }
    }
}

impl Space for FinalizedWithdrawals {
    const INIT_SPACE: usize = 32 + 4 + 0; // authority + vec length + 0 entries
}

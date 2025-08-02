#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::system_program;

use crate::biz_error;

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                         BRIDGE IMPL                        */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

/// Impl of deposit for native token (sol).
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let from = &ctx.accounts.sender;
    let bridge_vault = &mut ctx.accounts.bridge_vault;
    let program = &ctx.accounts.system_program;
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

    let current_balance = bridge_vault.get_balance(from.key);
    bridge_vault.set_balance(*from.key, current_balance + amount);
    msg!("deposit for account: {:?}, amount: {:?}", from.key, amount);
    Ok(())
}

/// Impl of withdrawal for native token (sol).
pub fn withdrawal(ctx: Context<Withdrawal>, amount: u64) -> Result<()> {
    let from = &ctx.accounts.sender;
    let to = &ctx.accounts.to;
    let bridge_vault = &mut ctx.accounts.bridge_vault;

    let current_amount = bridge_vault.get_balance(from.key);
    if current_amount < amount {
        return Err(Error::from(biz_error::ErrorCode::UserBalanceInsufficent));
    }

    bridge_vault.set_balance(*from.key, current_amount - amount);

    bridge_vault.sub_lamports(amount)?;
    to.add_lamports(amount)?;

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
}

#[account]
pub struct WithdrawalRoot {
    pub authority: Pubkey,
    pub withdrawal_root: [u8; 32],
}

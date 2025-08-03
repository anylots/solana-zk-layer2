use anchor_lang::prelude::*;

/*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
/*                          BIZ ERROR                         */
/*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/

/// Biz error code.
#[error_code]
pub enum ErrorCode {
    #[msg("Not approved")]
    NotApproved,
    #[msg("Not approved")]
    BatchNotExist,
    #[msg("Insufficient user balance")]
    UserBalanceInsufficent,
    #[msg("WithdrawalRoot not finalized")]
    WithdrawalRootNotFinalized,
    #[msg("invalid withdrawal inclusion proof")]
    InvalidWithdrawalInclusionProof,
}

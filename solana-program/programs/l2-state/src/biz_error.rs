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
}

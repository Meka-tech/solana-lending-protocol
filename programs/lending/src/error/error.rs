use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError{
    #[msg("Insufficient Funds")]
    InsufficientFunds,

    #[msg("Requested amount exceeds borrowable amount")]
    OverBorrowableAmount,

    #[msg("Amount exceeds debt ")]
    OverRepay,

    #[msg("User is not under collateralized so it can't be liquidated ")]
    NotUnderCollateralized
}
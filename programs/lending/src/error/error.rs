use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError{
    #[msg("Insufficient Funds")]
    InsufficientFunds,

    #[msg("Requested amount exceeds borrowable amount")]
    OverBorrowableAmount
}
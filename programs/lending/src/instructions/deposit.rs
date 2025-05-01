use anchor_lang::{prelude::*, solana_program::clock};
use anchor_spl::{associated_token::AssociatedToken,  token_interface::{Mint , TokenAccount, TokenInterface , TransferChecked , transfer_checked }} ;

use crate::{Bank, User};


#[derive(Accounts)]
pub struct Deposit <'info>{

    #[account(mut)]
    pub signer : Signer<'info>,

  pub mint : InterfaceAccount<'info , Mint>,

  #[account(
        mut,
        seeds = [mint.key().as_ref()],
        bump
        )]
    pub bank : Account<'info , Bank>,


    #[account(
        mut,
        seeds=[b"treasury" , mint.key().as_ref()],
        bump
    )]
    pub bank_token_account : InterfaceAccount<'info ,  TokenAccount>,


    #[account(
        mut,
        seeds = [signer.key().as_ref()],
        bump
    )]
    pub user_account : Account<'info , User>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    pub user_token_account : InterfaceAccount<'info  , TokenAccount>,

    pub token_program :  Interface<'info , TokenInterface>,

    pub associated_token_program : Program<'info , AssociatedToken> ,

    pub system_program : Program<'info , System>



}

pub fn process_deposit(ctx : Context<Deposit> ,  amount : u64) -> Result<()>{

    // transfer from user to bank
    let transfer_cpi_accounts = TransferChecked{
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.bank_token_account.to_account_info(),
        authority:ctx.accounts.signer.to_account_info(),
        mint: ctx.accounts.mint.to_account_info()
    };

    let cpi_progarm =  ctx.accounts.token_program.to_account_info();

    let cpi_ctx = CpiContext::new(cpi_progarm, transfer_cpi_accounts);

    let  decimals = ctx.accounts.mint.decimals;

    transfer_checked(cpi_ctx, amount, decimals)?;

    //update bank and user states

    let bank = &mut ctx.accounts.bank;

    if bank.total_deposits == 0 {
       bank.total_deposits = amount;
       bank.total_deposit_shares = amount;
    }

    let deposit_ratio = amount.checked_div(bank.total_deposits).unwrap();

    let user_shares = bank.total_deposit_shares.checked_mul(deposit_ratio).unwrap();


    let user_account = &mut ctx.accounts.user_account;


    match ctx.accounts.mint.to_account_info().key() {
       key if key == user_account.usdc_address => {
            user_account.deposited_usdc += amount;
            user_account.deposited_usdc_shares += user_shares
        }
        _ => {
            user_account.deposited_sol += amount;
            user_account.deposited_sol_shares += user_shares
        }
        

        
    }


    if bank.total_deposits != amount{
    bank.total_deposits += amount;
    bank.total_deposits +=  user_shares;}

    user_account.last_updated = Clock::get()?.unix_timestamp;


    Ok(())
}
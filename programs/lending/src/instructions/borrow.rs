use std::f32::consts::E;

use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint , TokenAccount, TokenInterface , TransferChecked , transfer_checked}};
use pyth_solana_receiver_sdk::price_update::{ get_feed_id_from_hex, PriceUpdateV2};

use crate::{Bank, CustomError, User, MAX_AGE, SOL_USD_FEED_ID, USDC_USD_FEED_ID};


#[derive(Accounts)]
pub struct  Borrow <'info> {

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
        init_if_needed,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
    pub user_token_account : InterfaceAccount<'info  , TokenAccount>,

    pub price_update : Account<'info , PriceUpdateV2>,




    pub token_program :  Interface<'info , TokenInterface>,

    pub associated_token_program : Program<'info , AssociatedToken> ,

    pub system_program : Program<'info , System>

}

pub fn process_borrow(ctx  : Context<Borrow>,  amount : u64) ->Result<()>{

    let bank = &mut ctx.accounts.bank;
    let user_account = &mut ctx.accounts.user_account;

    let price_update  = &mut ctx.accounts.price_update;

    let total_collateral : u64;

    match ctx.accounts.mint.to_account_info().key()  {
        key if key == user_account.usdc_address => {
            //user wants to borrow usdc , find collateral from amount of sol deposited
        let sol_feed_id =  get_feed_id_from_hex(SOL_USD_FEED_ID)?;
        let sol_price = price_update.get_price_no_older_than(&Clock::get()? , MAX_AGE, &sol_feed_id)?;

        let new_value = calculate_accrued_interest(user_account.deposited_sol, bank.interest_rate, user_account.last_updated)?;

         total_collateral = sol_price.price as u64 *  new_value ;

        }
        _ => {
            //user wants to borrow sol , find collateral from amount of usdc deposited
            let usdc_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID)?;
            let usdc_price = price_update.get_price_no_older_than(&Clock::get()? , MAX_AGE, &usdc_feed_id)?;

            let new_value =  calculate_accrued_interest(user_account.deposited_usdc, bank.interest_rate, user_account.last_updated)?;

            total_collateral = usdc_price.price as u64 *  new_value;
        }
    }

    let borrowable_amount = total_collateral.checked_mul(bank.liquidation_threshold).unwrap();

    if borrowable_amount < amount {
        return Err(CustomError::OverBorrowableAmount.into());
    }

    //transfer funds from bank to user

    let mint_key = ctx.accounts.mint.key();

    let signer_seeds: &[&[&[u8]]]  = &[
        &[
            b"treasury" ,
            mint_key.as_ref(),
             &[ctx.bumps.bank_token_account]
        ]
    ];

    let transfer_accounts  = TransferChecked{
        from : ctx.accounts.bank_token_account.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        mint : ctx.accounts.mint.to_account_info(),
        authority : ctx.accounts.bank_token_account.to_account_info()
    };


    let cpi_context = CpiContext::new(ctx.accounts.token_program.to_account_info(), transfer_accounts).with_signer(signer_seeds);


    let decimals = ctx.accounts.mint.decimals;

    transfer_checked(cpi_context, amount, decimals)?;

    //update state

    if bank.total_borrowed == 0 {
        bank.total_borrowed = amount;
        bank.total_borrowed_shares = amount;
    }

    let borrow_ratio =  amount.checked_div(bank.total_borrowed).unwrap();
    let user_shares =  bank.total_borrowed_shares.checked_mul(borrow_ratio).unwrap();

    match ctx.accounts.mint.to_account_info().key()  {
        key if key == user_account.usdc_address => {
            user_account.borrowed_usdc += amount;
            user_account.borrowed_usdc_shares += user_shares

        }
        _=> {
            user_account.borrowed_sol += amount;
            user_account.borrowed_sol_shares += user_shares
        }
    }




    Ok(())
}


fn calculate_accrued_interest (deposited_value : u64 ,  interest_rate : u64 , last_updated : i64) -> Result<u64> {

    let current_time = Clock::get()?.unix_timestamp;

    let time_diff = current_time - last_updated;

    let new_value = (deposited_value as f64 * E.powf(interest_rate as f32 * time_diff as f32) as f64) as u64;

    Ok(new_value)
}
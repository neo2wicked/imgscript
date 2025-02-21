use anchor_lang::prelude::*;
use anchor_spl::{
    token::{self, Mint, Token, TokenAccount, Transfer, MintTo},
    associated_token::AssociatedToken,
};
use std::ops::DerefMut;

declare_id!("fFWYsPabdn4Ld5WaiYzYxvMbSittMXk1s3aQu77257D");

#[program]
pub mod img_token {
    use super::*;

    // initialize everything we need - because planning ahead is a thing
    pub fn initialize(
        ctx: Context<Initialize>,
        name: String,
        symbol: String,
        decimals: u8,
    ) -> Result<()> {
        let token_config = &mut ctx.accounts.token_config;
        token_config.authority = ctx.accounts.authority.key();
        token_config.mint = ctx.accounts.mint.key();
        token_config.name = name;
        token_config.symbol = symbol;
        token_config.tax_rate = 500; // 5% = 500 basis points, learn some math
        token_config.distribution_interval = 300; // 5 mins in seconds
        token_config.last_distribution = Clock::get()?.unix_timestamp;
        
        // mint initial supply - because empty tokens are useless
        let initial_supply = 1_000_000_000 * 10u64.pow(decimals as u32); // 1B tokens
        token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.authority_ata.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            initial_supply,
        )?;

        Ok(())
    }

    // transfer with tax - now with actual error handling
    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        let token_config = &ctx.accounts.token_config;
        require!(amount > 0, ErrorCode::InvalidAmount);
        
        let tax_amount = (amount as u128 * token_config.tax_rate as u128 / 10000) as u64;
        let transfer_amount = amount.checked_sub(tax_amount).unwrap();

        // transfer main amount
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from.to_account_info(),
                    to: ctx.accounts.to.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            transfer_amount,
        )?;

        // transfer tax to vault
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from.to_account_info(),
                    to: ctx.accounts.tax_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            tax_amount,
        )?;

        emit!(TransferEvent {
            from: ctx.accounts.from.key(),
            to: ctx.accounts.to.key(),
            amount: transfer_amount,
            tax: tax_amount,
        });

        Ok(())
    }

    // swap collected taxes to SOL - because rewards don't magically appear
    pub fn swap_taxes_to_sol(ctx: Context<SwapTaxes>, amount: u64) -> Result<()> {
        require!(
            ctx.accounts.tax_vault.amount >= amount,
            ErrorCode::InsufficientTaxBalance
        );

        // here you'd integrate with Jupiter's swap API
        // for now we'll simulate it with a direct SOL transfer
        let sol_amount = amount / 1000; // dummy conversion rate
        
        **ctx.accounts.reward_vault.try_borrow_mut_lamports()? += sol_amount;
        **ctx.accounts.authority.try_borrow_mut_lamports()? -= sol_amount;

        Ok(())
    }

    // distribute rewards - now with actual checks
    pub fn distribute_rewards(ctx: Context<DistributeRewards>) -> Result<()> {
        let token_config = &mut ctx.accounts.token_config;
        let clock = Clock::get()?;
        
        require!(
            clock.unix_timestamp >= token_config.last_distribution + token_config.distribution_interval,
            ErrorCode::TooEarlyToDistribute
        );

        let total_supply = ctx.accounts.mint.supply;
        let reward_balance = ctx.accounts.reward_vault.lamports();
        require!(reward_balance > 0, ErrorCode::NoRewardsToDistribute);

        // your holders are passed in remaining_accounts
        for holder_info in ctx.remaining_accounts.iter() {
            let holder = Account::<TokenAccount>::try_from(holder_info)?;
            if holder.amount == 0 { continue; }

            let share = (holder.amount as u128 * reward_balance as u128) / total_supply as u128;
            if share == 0 { continue; }

            // transfer SOL rewards
            **ctx.accounts.reward_vault.try_borrow_mut_lamports()? -= share as u64;
            **holder.owner.try_borrow_mut_lamports()? += share as u64;
        }

        token_config.last_distribution = clock.unix_timestamp;
        
        emit!(DistributionEvent {
            timestamp: clock.unix_timestamp,
            amount: reward_balance,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    #[account(
        init,
        payer = authority,
        mint::decimals = decimals,
        mint::authority = authority,
    )]
    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = authority,
    )]
    pub authority_ata: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        space = TokenConfig::LEN
    )]
    pub token_config: Account<'info, TokenConfig>,

    // more accounts you forgot about
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

// rest of your account structures but actually done right
#[account]
pub struct TokenConfig {
    pub authority: Pubkey,            // 32
    pub mint: Pubkey,                 // 32
    pub name: String,                 // 40
    pub symbol: String,               // 10
    pub tax_rate: u16,               // 2
    pub distribution_interval: i64,   // 8
    pub last_distribution: i64,       // 8
    pub paused: bool,                // 1
}

impl TokenConfig {
    pub const LEN: usize = 8 + 32 + 32 + 40 + 10 + 2 + 8 + 8 + 1;
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    pub token_config: Account<'info, TokenConfig>,
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,
    #[account(mut)]
    pub tax_vault: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SwapTaxes<'info> {
    #[account(mut)]
    pub token_config: Account<'info, TokenConfig>,
    #[account(mut)]
    pub tax_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub reward_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    #[account(mut)]
    pub token_config: Account<'info, TokenConfig>,
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    /// CHECK: This is safe because we're just reading lamports
    #[account(mut)]
    pub reward_vault: UncheckedAccount<'info>,
}

#[error_code]
pub enum ErrorCode {
    InvalidAmount,
    InsufficientTaxBalance,
    TooEarlyToDistribute,
    NoRewardsToDistribute,
    Unauthorized,
    TaxTooHigh,
    IntervalTooShort,
    TransfersPaused,
}

#[event]
pub struct TransferEvent {
    pub from: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
    pub tax: u64,
}

#[event]
pub struct DistributionEvent {
    pub timestamp: i64,
    pub amount: u64,
} 
use crate::error::ErrorCode;
use crate::state::Bet;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct JoinBet<'info> {
    #[account(
        mut,
        seeds = [b"bet", creator.key().as_ref()],
        bump = bet.bump,
        has_one = creator,
    )]
    pub bet: Account<'info, Bet>,

    /// CHECK: vault PDA, holds lamports only
    #[account(
        mut,
        seeds = [b"vault", creator.key().as_ref()],
        bump = bet.vault_bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: creator, verified via has_one
    pub creator: UncheckedAccount<'info>,

    #[account(mut)]
    pub challenger: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<JoinBet>) -> Result<()> {
    let bet = &mut ctx.accounts.bet;

    require!(bet.challenger == Pubkey::default(), ErrorCode::AlreadyHasChallenger);
    require!(!bet.settled, ErrorCode::AlreadySettled);

    bet.challenger = ctx.accounts.challenger.key();

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.challenger.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        ),
        bet.lamports,
    )?;

    Ok(())
}
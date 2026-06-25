use crate::error::ErrorCode;
use crate::state::Bet;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CancelBet<'info> {
    #[account(
        mut,
        seeds = [b"bet", creator.key().as_ref()],
        bump = bet.bump,
        has_one = creator,
        close = creator,
    )]
    pub bet: Account<'info, Bet>,

    /// CHECK: vault PDA, holds lamports only
    #[account(
        mut,
        seeds = [b"vault", creator.key().as_ref()],
        bump = bet.vault_bump,
    )]
    pub vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CancelBet>) -> Result<()> {
    let bet = &ctx.accounts.bet;

    require!(bet.challenger == Pubkey::default(), ErrorCode::CannotCancel);
    require!(!bet.settled, ErrorCode::AlreadySettled);

    let vault_lamports = ctx.accounts.vault.lamports();
    **ctx.accounts.vault.try_borrow_mut_lamports()? -= vault_lamports;
    **ctx.accounts.creator.try_borrow_mut_lamports()? += vault_lamports;

    Ok(())
}
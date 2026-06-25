use crate::state::Bet;
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct CreateBet<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + Bet::INIT_SPACE,
        seeds = [b"bet", creator.key().as_ref()],
        bump,
    )]
    pub bet: Account<'info, Bet>,

    /// CHECK: vault PDA, holds lamports only
    #[account(
        init,
        payer = creator,
        space = 0,
        seeds = [b"vault", creator.key().as_ref()],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateBet>,
    city: u8,
    threshold: i32,
    direction: bool,
    deadline: i64,
    lamports: u64,
) -> Result<()> {
    let bet = &mut ctx.accounts.bet;
    bet.creator = ctx.accounts.creator.key();
    bet.challenger = Pubkey::default();
    bet.city = city;
    bet.threshold = threshold;
    bet.direction = direction;
    bet.deadline = deadline;
    bet.lamports = lamports;
    bet.settled = false;
    bet.bump = ctx.bumps.bet;
    bet.vault_bump = ctx.bumps.vault;

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.creator.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        ),
        lamports,
    )?;

    Ok(())
}
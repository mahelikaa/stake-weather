use crate::constants::{BANGALORE_FEED_HASH, DELHI_FEED_HASH, MUMBAI_FEED_HASH};
use crate::error::ErrorCode;
use crate::state::Bet;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct SettleBet<'info> {
    #[account(
    mut,
    seeds = [b"bet", creator.key().as_ref()],
    bump = bet.bump,
    has_one = creator,
    has_one = challenger,
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

    /// CHECK: creator, verified via has_one
    #[account(mut)]
    pub creator: UncheckedAccount<'info>,

    /// CHECK: challenger, verified via has_one
    #[account(mut)]
    pub challenger: UncheckedAccount<'info>,

    /// CHECK: switchboard oracle quote account
    pub oracle: UncheckedAccount<'info>,

    #[account(mut)]
    pub caller: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SettleBet>) -> Result<()> {
    let bet = &mut ctx.accounts.bet;

    require!(!bet.settled, ErrorCode::AlreadySettled);

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp >= bet.deadline,
        ErrorCode::DeadlineNotReached
    );

    let expected_hash = match bet.city {
        0 => MUMBAI_FEED_HASH,
        1 => DELHI_FEED_HASH,
        2 => BANGALORE_FEED_HASH,
        _ => return err!(ErrorCode::InvalidCity),
    };

    let oracle_data = ctx.accounts.oracle.try_borrow_data()?;
    if oracle_data.len() < 42 {
        return err!(ErrorCode::InvalidFeed);
    }
    let data_len = u16::from_le_bytes([oracle_data[40], oracle_data[41]]) as usize;
    if oracle_data.len() < 42 + data_len {
        return err!(ErrorCode::InvalidFeed);
    }
    let quote_bytes = &oracle_data[42..42 + data_len];
    if quote_bytes.len() < 46 {
        return err!(ErrorCode::FeedValueMissing);
    }
    let feeds_bytes = &quote_bytes[46..];
    const FEED_SIZE: usize = 49;
    let num_feeds = feeds_bytes.len() / FEED_SIZE;

    let mut raw_value: Option<i128> = None;
    for i in 0..num_feeds {
        let offset = i * FEED_SIZE;
        let feed_id: [u8; 32] = feeds_bytes[offset..offset + 32].try_into().unwrap();
        if feed_id == expected_hash {
            let val_bytes: [u8; 16] = feeds_bytes[offset + 32..offset + 48].try_into().unwrap();
            raw_value = Some(i128::from_le_bytes(val_bytes));
            break;
        }
    }

    let raw = raw_value.ok_or(ErrorCode::FeedValueMissing)?;
    // raw is scaled by 1e18; threshold is temp * 10, so divide by 1e17
    let temp = (raw / 100_000_000_000_000_000i128) as i32;

    let creator_wins = if bet.direction {
        temp >= bet.threshold
    } else {
        temp < bet.threshold
    };

    let vault_lamports = ctx.accounts.vault.lamports();

    if creator_wins {
        **ctx.accounts.vault.try_borrow_mut_lamports()? -= vault_lamports;
        **ctx.accounts.creator.try_borrow_mut_lamports()? += vault_lamports;
    } else {
        **ctx.accounts.vault.try_borrow_mut_lamports()? -= vault_lamports;
        **ctx.accounts.challenger.try_borrow_mut_lamports()? += vault_lamports;
    }

    let remaining = ctx.accounts.vault.lamports();
    if remaining > 0 {
        **ctx.accounts.vault.try_borrow_mut_lamports()? -= remaining;
        **ctx.accounts.creator.try_borrow_mut_lamports()? += remaining;
    }

    bet.settled = true;

    Ok(())
}

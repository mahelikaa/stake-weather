pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5");

#[program]
pub mod stake_weather {
    use super::*;

    pub fn create_bet(ctx: Context<CreateBet>, city: u8, threshold: i32, direction: bool, deadline: i64, lamports: u64) -> Result<()> {
        create_bet::handler(ctx, city, threshold, direction, deadline, lamports)
    }

    pub fn join_bet(ctx: Context<JoinBet>) -> Result<()> {
        join_bet::handler(ctx)
    }

    pub fn cancel_bet(ctx: Context<CancelBet>) -> Result<()> {
        cancel_bet::handler(ctx)
    }

   pub fn settle_bet(ctx: Context<SettleBet>) -> Result<()> {
    settle_bet::handler(ctx)
}
}

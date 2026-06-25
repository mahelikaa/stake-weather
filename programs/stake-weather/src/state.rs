use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Bet {
    pub creator: Pubkey,
    pub challenger: Pubkey,
    pub city: u8,
    pub threshold: i32,
    pub direction: bool,
    pub deadline: i64,
    pub lamports: u64,
    pub settled: bool,
    pub bump: u8,
    pub vault_bump: u8,
}
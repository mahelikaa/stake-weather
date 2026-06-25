use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Bet already settled")]
    AlreadySettled,
    #[msg("Deadline not reached")]
    DeadlineNotReached,
    #[msg("Bet already has a challenger")]
    AlreadyHasChallenger,
    #[msg("Only creator can cancel")]
    Unauthorized,
    #[msg("Cannot cancel after challenger joined")]
    CannotCancel,
    #[msg("Invalid city index")]
    InvalidCity,
    #[msg("Invalid Switchboard feed account")]
    InvalidFeed,
    #[msg("Feed has no value yet")]
    FeedValueMissing,
}
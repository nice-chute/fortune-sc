use anchor_lang::prelude::*;

#[error]
pub enum FortuneError {
    #[msg("Bid is too low")]
    BidTooLow,
    #[msg("Prob pool is closed")]
    PoolClosed,
    #[msg("Only seller can change ask")]
    InvalidAskAuth,
    #[msg("Locked listing")]
    LockedListing,
    #[msg("Ask cannot be less than zero")]
    ZeroAsk,
    #[msg("Maximum burn amount is 9")]
    BurnLimit,
    #[msg("Outstanding ptokens, cannot close pool")]
    OutstandingProb,
    #[msg("Active claim against pool")]
    ActiveClaim,
    #[msg("Creator cannot claim, close the pool instead")]
    CreatorCannotClaim,
}

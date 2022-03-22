use anchor_lang::prelude::*;

#[error_code]
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
    #[msg("No active claim on this pool")]
    NoClaim,
    #[msg("Lamport init amount too low")]
    LamportInitMin,
    #[msg("Lamport init amount too high")]
    LamportInitMax,
    #[msg("pToken init amount too low")]
    PtokenInitMin,
    #[msg("pToken init amount too high")]
    PtokenInitMax,
    #[msg("No more pTokens left to buy")]
    SoldOut,
}

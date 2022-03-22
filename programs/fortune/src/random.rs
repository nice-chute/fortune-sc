use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use std::convert::TryInto;

//https://docs.chain.link/docs/chainlink-vrf-best-practices/#getting-multiple-random-number
pub fn expand_with_num(randomness: [u8; 32], n: u64) -> u64 {
    let mut hasher = keccak::Hasher::default();
    hasher.hash(&randomness);
    hasher.hash(&n.to_le_bytes());

    u64::from_le_bytes(
        hasher.result().to_bytes()[0..8]
            .try_into()
            .expect("slice with incorrect length"),
    )
}

//https://docs.chain.link/docs/chainlink-vrf-best-practices/#getting-multiple-random-number
pub fn expand_with_pubkey(randomness: [u8; 32], n: [u8; 32]) -> u64 {
    let mut hasher = keccak::Hasher::default();
    hasher.hash(&randomness);
    hasher.hash(&n);

    u64::from_le_bytes(
        hasher.result().to_bytes()[0..8]
            .try_into()
            .expect("slice with incorrect length"),
    )
}

// // https://docs.rs/solana-program/1.8.2/solana_program/sysvar/recent_blockhashes/struct.RecentBlockhashes.html
// pub fn last_blockhash_accessor(recent_blockhashes: &AccountInfo) -> Result<[u8; 32], ProgramError> {
//     let bytes = recent_blockhashes.try_borrow_data()?;
//     let mut entry_length = [0u8; 8];
//     entry_length.copy_from_slice(&bytes[0..8]);
//     if u64::from_le_bytes(entry_length) == 0 {
//         // Impossible
//         return Err(ProgramError::InvalidAccountData);
//     }
//     let mut last_blockhash = [0u8; 32];
//     last_blockhash.copy_from_slice(&bytes[8..(8 + 32)]);
//     Ok(last_blockhash)
// }

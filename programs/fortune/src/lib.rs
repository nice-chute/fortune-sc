use anchor_lang::prelude::*;
use anchor_spl::token;
use anchor_spl::token::{Mint, Token, TokenAccount};
use solana_program::program::invoke_signed;
use solana_program::{system_instruction, sysvar};
use spl_token::instruction::sync_native;

declare_id!("7tSKVgnzdSAStFuDzPjqE7mhCtXrnX9KLTsbJuGrn52C");

mod error;
mod random;

#[program]
pub mod fortune {
    use super::*;

    // Create program vaults
    pub fn initialize(
        ctx: Context<Initialize>,
        swap_fee: u64,
        burn_cost: u64,
        fee_scalar: u64,
        lamport_min: u64,
        lamport_max: u64,
        ptoken_max: u64,
        ptoken_min: u64,
    ) -> ProgramResult {
        // Set state
        let state = &mut ctx.accounts.state;
        state.burn_cost = burn_cost;
        state.fee_scalar = fee_scalar;
        state.authority = ctx.accounts.signer.key();
        state.swap_fee = swap_fee;
        state.lamport_init_min = lamport_min;
        state.lamport_init_max = lamport_max;
        state.ptoken_init_max = ptoken_max;
        state.ptoken_init_min = ptoken_min;
        Ok(())
    }

    // Create probability pool and its vaults
    pub fn create_pool(
        ctx: Context<CreatePool>,
        lamport_amount: u64,
        ptoken_amount: u64,
    ) -> ProgramResult {
        require!(
            lamport_amount >= ctx.accounts.state.lamport_init_min,
            error::FortuneError::LamportInitMin
        );
        require!(
            lamport_amount < ctx.accounts.state.lamport_init_max,
            error::FortuneError::LamportInitMax
        );
        require!(
            ptoken_amount < ctx.accounts.state.ptoken_init_max,
            error::FortuneError::PtokenInitMax
        );
        require!(
            ptoken_amount >= ctx.accounts.state.ptoken_init_min,
            error::FortuneError::PtokenInitMin
        );
        // Set pool data
        ctx.accounts.prob_pool.authority = ctx.accounts.signer.key();
        ctx.accounts.prob_pool.nft_authority = ctx.accounts.signer.key();
        ctx.accounts.prob_pool.lamport_vault = ctx.accounts.lamport_vault.key();
        ctx.accounts.prob_pool.ptoken_vault = ctx.accounts.ptoken_vault.key();
        ctx.accounts.prob_pool.lamport_vault = ctx.accounts.lamport_vault.key();
        ctx.accounts.prob_pool.ptoken_vault = ctx.accounts.ptoken_vault.key();
        ctx.accounts.prob_pool.ptoken_mint = ctx.accounts.ptoken_mint.key();
        ctx.accounts.prob_pool.nft_mint = ctx.accounts.nft_mint.key();
        // Set pool params
        ctx.accounts.prob_pool.claimed = false;
        ctx.accounts.prob_pool.ptoken_supply = ptoken_amount;
        ctx.accounts.prob_pool.lamport_supply = lamport_amount;
        ctx.accounts.prob_pool.outstanding_ptokens = 0;

        let ptoken_mint_bump = *ctx.bumps.get("ptoken_mint").unwrap();

        // Mint ptokens to vault
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.ptoken_mint.to_account_info(),
                    to: ctx.accounts.ptoken_vault.to_account_info(),
                    authority: ctx.accounts.ptoken_mint.to_account_info(),
                },
                &[&[
                    &b"mint"[..],
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &[ptoken_mint_bump],
                ]],
            ),
            ptoken_amount,
        )?;

        // Transfer nft to vault
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.nft_account.to_account_info(),
                    to: ctx.accounts.nft_vault.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
                &[&[]],
            ),
            1,
        )?;
        Ok(())
    }

    // Swap SPL for ptokens
    pub fn buy(ctx: Context<Buy>, ptoken_amount: u64) -> ProgramResult {
        // Prob pool is active
        require!(
            ctx.accounts.prob_pool.claimed == false,
            error::FortuneError::PoolClosed
        );
        msg!("swap_fee: {:?}", ctx.accounts.state.swap_fee);
        msg!("scalar: {:?}", ctx.accounts.state.fee_scalar);
        // Calculate new AMM token supply, costs, and fees
        let k = ctx.accounts.prob_pool.ptoken_supply * ctx.accounts.prob_pool.lamport_supply;
        msg!("k: {:?}", k);
        let new_ptoken_supply = ctx.accounts.prob_pool.ptoken_supply - ptoken_amount;
        msg!("new_ptoken_suppl: {:?}", new_ptoken_supply);
        let new_spl_supply = k / new_ptoken_supply;
        msg!("new_spl_supply: {:?}", new_spl_supply);
        let spl_cost = new_spl_supply - ctx.accounts.prob_pool.lamport_supply;
        let spl_fee = (spl_cost * ctx.accounts.state.swap_fee) / ctx.accounts.state.fee_scalar;
        msg!("fee: {:?}", spl_fee);
        msg!("cost: {:?}", spl_cost);

        let pool_token_bump = *ctx.bumps.get("pool_ptoken_vault").unwrap();

        // Transfer spl cost to pool lamport vault
        invoke_signed(
            &system_instruction::transfer(
                &ctx.accounts.signer.key(),
                &ctx.accounts.pool_lamport_vault.key(),
                spl_cost,
            ),
            &[
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.pool_lamport_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;
        // Sync native
        let ix_1 = sync_native(
            &ctx.accounts.token_program.key(),
            &ctx.accounts.pool_lamport_vault.key(),
        )?;
        invoke_signed(
            &ix_1,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.pool_lamport_vault.to_account_info(),
            ],
            &[],
        )?;
        // Transfer fees to fortune vault
        invoke_signed(
            &system_instruction::transfer(
                &ctx.accounts.signer.key(),
                &ctx.accounts.fortune_lamport_vault.key(),
                spl_fee,
            ),
            &[
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.fortune_lamport_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;
        // Sync native
        let ix_2 = sync_native(
            &ctx.accounts.token_program.key(),
            &ctx.accounts.fortune_lamport_vault.key(),
        )?;
        invoke_signed(
            &ix_2,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.fortune_lamport_vault.to_account_info(),
            ],
            &[],
        )?;
        // Transfer ptokens to prob pool user vault
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.pool_ptoken_vault.to_account_info(),
                    to: ctx.accounts.user_ptoken_vault.to_account_info(),
                    authority: ctx.accounts.pool_ptoken_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.ptoken_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &[pool_token_bump],
                ]],
            ),
            ptoken_amount,
        )?;
        // Set prob pool data
        ctx.accounts.prob_pool.ptoken_supply = new_ptoken_supply;
        ctx.accounts.prob_pool.lamport_supply = new_spl_supply;
        ctx.accounts.prob_pool.outstanding_ptokens += ptoken_amount;
        // TODO: RNG might change here?
        Ok(())
    }

    pub fn request_burn(ctx: Context<RequestBurn>, ptoken_amount: u64) -> ProgramResult {
        // Bump
        let user_ptoken_vault_bump = *ctx.bumps.get("user_ptoken_vault").unwrap();
        // Transfer from user vault to user burn
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.user_ptoken_vault.to_account_info(),
                    to: ctx.accounts.user_burn.to_account_info(),
                    authority: ctx.accounts.user_ptoken_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.ptoken_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &ctx.accounts.signer.key().as_ref(),
                    &[user_ptoken_vault_bump],
                ]],
            ),
            ptoken_amount,
        )?;
        // Pay burn fees
        invoke_signed(
            &system_instruction::transfer(
                &ctx.accounts.signer.key(),
                &ctx.accounts.fortune_lamport_vault.key(),
                ctx.accounts.state.burn_cost,
            ),
            &[
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.fortune_lamport_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[],
        )?;
        // Sync native
        let ix_2 = sync_native(
            &ctx.accounts.token_program.key(),
            &ctx.accounts.fortune_lamport_vault.key(),
        )?;
        invoke_signed(
            &ix_2,
            &[
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.fortune_lamport_vault.to_account_info(),
            ],
            &[],
        )?;
        Ok(())
    }

    pub fn user_withdraw(ctx: Context<UserWithdraw>, token_amount: u64) -> ProgramResult {
        // Bump
        let vault_bump = *ctx.bumps.get("user_vault").unwrap();
        // Transfer from user vault to user account
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.user_vault.to_account_info(),
                    to: ctx.accounts.user_account.to_account_info(),
                    authority: ctx.accounts.user_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.vault_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &ctx.accounts.signer.key().as_ref(),
                    &[vault_bump],
                ]],
            ),
            token_amount,
        )?;
        // Close empty accounts
        if ctx.accounts.user_vault.amount == 0 {
            token::close_account(CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::CloseAccount {
                    account: ctx.accounts.user_vault.to_account_info(),
                    destination: ctx.accounts.signer.to_account_info(),
                    authority: ctx.accounts.user_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.vault_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &ctx.accounts.signer.key().as_ref(),
                    &[vault_bump],
                ]],
            ))?;
        }
        Ok(())
    }

    // Burn ptokens in order to try to win the asset
    pub fn execute_burn(ctx: Context<ExecuteBurn>, burn_amount: u64) -> ProgramResult {
        // Bump
        let user_burn_bump = *ctx.bumps.get("user_burn").unwrap();
        // Burn ptokens in user burn
        token::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: ctx.accounts.ptoken_mint.to_account_info(),
                    to: ctx.accounts.user_burn.to_account_info(),
                    authority: ctx.accounts.user_burn.to_account_info(),
                },
                &[&[
                    &b"burn"[..],
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &ctx.accounts.user.key().as_ref(),
                    &[user_burn_bump],
                ]],
            ),
            burn_amount,
        )?;

        // // Randomness from blockhash
        // let randomness = random::last_blockhash_accessor(&ctx.accounts.recent_blockhashes)?;
        // // X = number between 1 and ptoken supply. Generated using the user pubkey
        // let mut rng: u64 =
        //     random::expand_with_pubkey(randomness, ctx.accounts.user.key().to_bytes())
        //         % ctx.accounts.prob_pool.ptoken_supply;

        let rng = ctx.accounts.prob_pool.ptoken_supply; // REMOVE: TESTING ONLY

        // P(win) = P(X < burn_amount) = 1-P(X >= burn_amount)
        if rng >= (ctx.accounts.prob_pool.ptoken_supply - burn_amount) {
            // Transfer nft to user
            ctx.accounts.prob_pool.nft_authority = ctx.accounts.user.key();
        }
        // Update prob pool data
        ctx.accounts.prob_pool.claimed = true;
        ctx.accounts.prob_pool.outstanding_ptokens -= burn_amount;
        Ok(())
    }

    // Claim underlying asset
    pub fn claim_asset(ctx: Context<ClaimAsset>) -> ProgramResult {
        // Bump
        let nft_vault_bump = *ctx.bumps.get("nft_vault").unwrap();
        // Creator cannot claim
        require!(
            ctx.accounts.signer.key() != ctx.accounts.prob_pool.authority,
            error::FortuneError::CreatorCannotClaim
        );
        // Transfer nft to claimer
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.nft_vault.to_account_info(),
                    to: ctx.accounts.nft_account.to_account_info(),
                    authority: ctx.accounts.nft_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.nft_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &[nft_vault_bump],
                ]],
            ),
            1,
        )?;
        // Close pool
        ctx.accounts.prob_pool.claimed = true;
        Ok(())
    }

    // Close a probability pool, requires no outstanding ptokens
    pub fn close_pool(ctx: Context<ClosePool>) -> ProgramResult {
        // Bumps
        let nft_vault_bump = *ctx.bumps.get("nft_vault").unwrap();
        let spl_vault_bump = *ctx.bumps.get("pool_lamport_vault").unwrap();
        let ptoken_vault_bump = *ctx.bumps.get("pool_ptoken_vault").unwrap();

        // No outstanding ptokens
        require!(
            ctx.accounts.prob_pool.outstanding_ptokens == 0,
            error::FortuneError::OutstandingProb
        );
        // Active claim outstanding. Todo: remove this from close critical path
        if ctx.accounts.prob_pool.nft_authority != ctx.accounts.prob_pool.authority {
            require!(
                ctx.accounts.prob_pool.claimed == true,
                error::FortuneError::ActiveClaim
            )
        }
        // Nft was not won
        else {
            // Transfer NFT back to creator
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    anchor_spl::token::Transfer {
                        from: ctx.accounts.nft_vault.to_account_info(),
                        to: ctx.accounts.nft_account.to_account_info(),
                        authority: ctx.accounts.nft_vault.to_account_info(),
                    },
                    &[&[
                        &b"vault"[..],
                        &ctx.accounts.nft_mint.key().as_ref(),
                        &ctx.accounts.prob_pool.key().as_ref(),
                        &[nft_vault_bump],
                    ]],
                ),
                1,
            )?;
        }
        // Transfer pool spl funds to pool authority
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.pool_lamport_vault.to_account_info(),
                    to: ctx.accounts.recipient.to_account_info(),
                    authority: ctx.accounts.pool_lamport_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.native_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &[spl_vault_bump],
                ]],
            ),
            ctx.accounts.pool_lamport_vault.amount,
        )?;
        // Burn all ptokens
        token::burn(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: ctx.accounts.ptoken_mint.to_account_info(),
                    to: ctx.accounts.pool_ptoken_vault.to_account_info(),
                    authority: ctx.accounts.pool_ptoken_vault.to_account_info(),
                },
                &[&[
                    &b"vault"[..],
                    &ctx.accounts.ptoken_mint.key().as_ref(),
                    &ctx.accounts.prob_pool.key().as_ref(),
                    &[ptoken_vault_bump],
                ]],
            ),
            ctx.accounts.pool_ptoken_vault.amount,
        )?;
        // Close pool ptoken vault
        token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.pool_ptoken_vault.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx.accounts.pool_ptoken_vault.to_account_info(),
            },
            &[&[
                &b"vault"[..],
                &ctx.accounts.ptoken_mint.key().as_ref(),
                &ctx.accounts.prob_pool.key().as_ref(),
                &[ptoken_vault_bump],
            ]],
        ))?;
        // Close pool spl vault
        token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.pool_lamport_vault.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx.accounts.pool_lamport_vault.to_account_info(),
            },
            &[&[
                &b"vault"[..],
                &ctx.accounts.native_mint.key().as_ref(),
                &ctx.accounts.prob_pool.key().as_ref(),
                &[spl_vault_bump],
            ]],
        ))?;
        // Close pool nft vault
        token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::CloseAccount {
                account: ctx.accounts.nft_vault.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx.accounts.nft_vault.to_account_info(),
            },
            &[&[
                &b"vault"[..],
                &ctx.accounts.nft_mint.key().as_ref(),
                &ctx.accounts.prob_pool.key().as_ref(),
                &[nft_vault_bump],
            ]],
        ))?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        token::mint = spl_mint,
        token::authority = spl_vault,
        seeds = [b"vault", spl_mint.key().as_ref()],
        bump
    )]
    pub spl_vault: Box<Account<'info, TokenAccount>>,
    #[account(address = spl_token::native_mint::ID)]
    pub spl_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        space = 250,
        payer = signer,
        seeds = [b"fortune"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    // Holds the NFT, owned by the signer
    #[account(
        mut,
        constraint = nft_account.owner == signer.key(),
        constraint = nft_account.mint == nft_mint.key(),
    )]
    pub nft_account: Box<Account<'info, TokenAccount>>,
    // Prob pools are generated from a keypair
    #[account(
        init,
        space = 350,
        payer = signer
    )]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    // Ptoken mint is unique for each pool
    #[account(
        init,
        payer = signer,
        seeds = ["mint".as_bytes(),prob_pool.key().as_ref()],
        bump,
        mint::decimals = 0,
        mint::authority = ptoken_mint
    )]
    pub ptoken_mint: Box<Account<'info, Mint>>,
    // Vault for nft
    #[account(
        init,
        payer = signer,
        token::mint = nft_mint,
        token::authority = nft_vault,
        seeds = [b"vault", nft_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    // Vault for SPL token
    #[account(
        init,
        payer = signer,
        token::mint = native_mint,
        token::authority = lamport_vault,
        seeds = [b"vault", native_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump
    )]
    pub lamport_vault: Box<Account<'info, TokenAccount>>,
    // Vault for ptokens
    #[account(
        init,
        payer = signer,
        token::mint = ptoken_mint,
        token::authority = ptoken_vault,
        seeds = [b"vault", ptoken_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump
    )]
    pub ptoken_vault: Box<Account<'info, TokenAccount>>,
    // Mint address identifies the NFT
    #[account()]
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(address = spl_token::native_mint::ID)]
    pub native_mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [b"fortune"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(
    ptoken_amount: u64)]
pub struct Buy<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub pool_lamport_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"vault", ptoken_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump
    )]
    pub pool_ptoken_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = prob_pool.lamport_vault == pool_lamport_vault.key(),
        constraint = prob_pool.ptoken_vault == pool_ptoken_vault.key(),
        constraint = prob_pool.ptoken_mint == ptoken_mint.key()
        )]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    #[account(
        mut,
        seeds = [b"vault", native_mint.key().as_ref()],
        bump
    )]
    pub fortune_lamport_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = ptoken_mint,
        token::authority = user_ptoken_vault,
        seeds = [b"vault", ptoken_mint.key().as_ref(), prob_pool.key().as_ref(), signer.key().as_ref()],
        bump
    )]
    pub user_ptoken_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        seeds = ["mint".as_bytes(), prob_pool.key().as_ref()],
        bump,
    )]
    pub ptoken_mint: Box<Account<'info, Mint>>,
    #[account(address = spl_token::native_mint::ID)]
    pub native_mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [b"fortune"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(
    ptoken_amount: u64)]
pub struct RequestBurn<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", native_mint.key().as_ref()],
        bump
    )]
    pub fortune_lamport_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"vault", ptoken_mint.key().as_ref(), prob_pool.key().as_ref(), signer.key().as_ref()],
        bump
    )]
    pub user_ptoken_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = ptoken_mint,
        token::authority = user_burn,
        seeds = [b"burn", prob_pool.key().as_ref(), signer.key().as_ref()],
        bump
    )]
    pub user_burn: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = prob_pool.ptoken_mint == ptoken_mint.key()
        )]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    #[account(
        seeds = ["mint".as_bytes(), prob_pool.key().as_ref()],
        bump,
    )]
    pub ptoken_mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [b"fortune"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    #[account(address = spl_token::native_mint::ID)]
    pub native_mint: Box<Account<'info, Mint>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(
    token_amount: u64)]
pub struct UserWithdraw<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_mint.key().as_ref(), prob_pool.key().as_ref(), signer.key().as_ref()],
        bump
    )]
    pub user_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = vault_mint,
        token::authority = signer,
    )]
    pub user_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    #[account()]
    pub vault_mint: Box<Account<'info, Mint>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(
    burn_amount: u64)]
pub struct ExecuteBurn<'info> {
    #[account(
        mut,
        //TODO constraint = fortune_authority.key() == state.authority
    )]
    pub fortune_authority: Signer<'info>,
    #[account(mut)]
    pub user: UncheckedAccount<'info>,
    // Vault for nft
    #[account(
        mut,
        seeds = [b"vault", nft_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump,
        constraint = nft_vault.mint == nft_mint.key(),
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [b"burn", prob_pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_burn: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = prob_pool.nft_mint == nft_mint.key()
        )]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = ["mint".as_bytes(), prob_pool.key().as_ref()],
        bump,
    )]
    pub ptoken_mint: Box<Account<'info, Mint>>,
    #[account(
        seeds = [b"fortune"],
        bump
    )]
    pub state: Box<Account<'info, State>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction()]
pub struct ClaimAsset<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = nft_mint,
        token::authority = signer)]
    pub nft_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = prob_pool.nft_mint == nft_mint.key(),
        constraint = prob_pool.nft_authority == signer.key()
        )]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    #[account(
        mut,
        seeds = [b"vault", nft_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump,
        constraint = nft_vault.mint == nft_mint.key(),
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    pub nft_mint: Box<Account<'info, Mint>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction()]
pub struct ClosePool<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = native_mint,
        token::authority = signer)]
    pub recipient: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = signer,
        token::mint = nft_mint,
        token::authority = signer)]
    pub nft_account: Account<'info, TokenAccount>,
    // Prob pools are generated from a keypair
    #[account(
        mut,
        close = signer,
        constraint = prob_pool.authority == signer.key(),
        constraint = prob_pool.nft_mint == nft_mint.key())]
    pub prob_pool: Box<Account<'info, ProbPool>>,
    // Ptoken mint is unique for each pool
    #[account(
        mut,
        seeds = ["mint".as_bytes(),prob_pool.key().as_ref()],
        bump,
    )]
    pub ptoken_mint: Box<Account<'info, Mint>>,
    // Vault for nft
    #[account(
        mut,
        seeds = [b"vault", nft_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump,
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,
    // Vault for SPL token
    #[account(
        mut,
        seeds = [b"vault", native_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump
    )]
    pub pool_lamport_vault: Box<Account<'info, TokenAccount>>,
    // Vault for ptokens
    #[account(
        mut,
        seeds = [b"vault", ptoken_mint.key().as_ref(), prob_pool.key().as_ref()],
        bump
    )]
    pub pool_ptoken_vault: Box<Account<'info, TokenAccount>>,
    // Mint address identifies the NFT
    #[account()]
    pub nft_mint: Box<Account<'info, Mint>>,
    #[account(address = spl_token::native_mint::ID)]
    pub native_mint: Box<Account<'info, Mint>>,
    // System programs + sysvars
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
// Prob pool is an AMM: pToken/SPL
pub struct ProbPool {
    authority: Pubkey,
    nft_authority: Pubkey,
    lamport_vault: Pubkey,
    ptoken_vault: Pubkey,
    ptoken_mint: Pubkey,
    nft_mint: Pubkey,
    claimed: bool,
    lamport_supply: u64,
    ptoken_supply: u64,
    outstanding_ptokens: u64,
}

#[account]
// Fortune state
pub struct State {
    authority: Pubkey,
    burn_cost: u64,
    fee_scalar: u64,
    swap_fee: u64,
    lamport_init_min: u64,
    lamport_init_max: u64,
    ptoken_init_max: u64,
    ptoken_init_min: u64,
}

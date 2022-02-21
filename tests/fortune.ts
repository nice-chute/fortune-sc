import * as anchor from '@project-serum/anchor';
import { Program, BN } from '@project-serum/anchor';
import { Fortune } from '../target/types/fortune';
import {
  PublicKey, Keypair, SystemProgram, Transaction, TransactionInstruction, LAMPORTS_PER_SOL,
  SYSVAR_RECENT_BLOCKHASHES_PUBKEY,
  SYSVAR_RENT_PUBKEY
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, Token, NATIVE_MINT, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe('fortune', () => {

  // Configure the client to use the local cluster.
  const provider = anchor.Provider.env();
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.Fortune as Program<Fortune>;

  // Auth
  const fortuneAuth = Keypair.generate();
  const creatorAuth = Keypair.generate();
  const buyerAuth = Keypair.generate();
  const mintAuth = Keypair.generate();

  // Params
  const swapFee = new anchor.BN(25);
  const one = new anchor.BN(1);
  const splAmount = new anchor.BN(10 * LAMPORTS_PER_SOL);
  const ptokenAmount = new anchor.BN(LAMPORTS_PER_SOL);
  const buyAmount = new anchor.BN(5);
  const burnAmount = new anchor.BN(5);
  const withdrawAmount = new anchor.BN(0);
  const burnCost = new anchor.BN(10000)
  const feeScalar = new anchor.BN(1000)
  const splMin = new anchor.BN(LAMPORTS_PER_SOL * .01)
  const splMax = new anchor.BN(LAMPORTS_PER_SOL * 100000)
  const ptokenMax = new anchor.BN(LAMPORTS_PER_SOL * 1000000)
  const ptokenMin = new anchor.BN(1)

  // Accounts
  const probPool = Keypair.generate();
  const userPtokenAccount = Keypair.generate();
  const creatorSplAccount = Keypair.generate();
  const buyerNftAccount = Keypair.generate();
  const creatorNftAccount = Keypair.generate();
  let nftAccount = null;
  let ptokenMint = null;
  let nftVault = null;
  let splVault = null;
  let ptokenVault = null;
  let nftMint = null;
  let fortuneVault = null;
  let userPtokenVault = null;
  let userBurn = null;
  let userNftVault = null;
  let state = null;

  // Bumps
  let ptokenMintBump = null;
  let nftVaultBump = null;
  let splVaultBump = null;
  let ptokenVaultBump = null;
  let fortuneVaultBump = null;
  let userPtokenVaultBump = null;
  let userBurnBump = null;
  let userNftVaultBump = null;
  let stateBump = null;

  it('Initialize state', async () => {
    // Airdrop to creator auth
    const creatorAuthAirdrop = await provider.connection.requestAirdrop(creatorAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(creatorAuthAirdrop);
    // Airdrop to mint auth
    const mintAuthAirdrop = await provider.connection.requestAirdrop(mintAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(mintAuthAirdrop);
    // Airdrop to caroline auth
    const fortuneAuthAirdrop = await provider.connection.requestAirdrop(fortuneAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(fortuneAuthAirdrop);
    // Airdrop to buyer auth
    const buyerAuthAirdrop = await provider.connection.requestAirdrop(buyerAuth.publicKey, 100 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(buyerAuthAirdrop);

    // Nft mint
    nftMint = await Token.createMint(
      provider.connection,
      mintAuth,
      mintAuth.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );
    // Nft account owned by creator
    nftAccount = await nftMint.createAccount(
      creatorAuth.publicKey,
    );
    // Mint 1 nft to account
    await nftMint.mintTo(
      nftAccount,
      mintAuth.publicKey,
      [mintAuth],
      1
    );

    // ptoken mint PDA
    [ptokenMint, ptokenMintBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("mint")),
        probPool.publicKey.toBuffer(),
      ],
      program.programId
    );
    // nft vault pda
    [nftVault, nftVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        nftMint.publicKey.toBuffer(),
        probPool.publicKey.toBuffer(),
      ],
      program.programId
    );
    // user nft vault pda
    [userNftVault, userNftVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        nftMint.publicKey.toBuffer(),
        probPool.publicKey.toBuffer(),
        buyerAuth.publicKey.toBuffer(),
      ],
      program.programId
    );
    // SPL vault PDA
    [splVault, splVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        NATIVE_MINT.toBuffer(),
        probPool.publicKey.toBuffer()
      ],
      program.programId
    );
    // ptoken vault PDA
    [ptokenVault, ptokenVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        ptokenMint.toBuffer(),
        probPool.publicKey.toBuffer()
      ],
      program.programId
    );
    // Caroline SPL vault PDA
    [fortuneVault, fortuneVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        NATIVE_MINT.toBuffer(),
      ],
      program.programId
    );
    // User pool ptoken vault
    [userPtokenVault, userPtokenVaultBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("vault")),
        ptokenMint.toBuffer(),
        probPool.publicKey.toBuffer(),
        buyerAuth.publicKey.toBuffer()
      ],
      program.programId
    );
    // User burn
    [userBurn, userBurnBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("burn")),
        probPool.publicKey.toBuffer(),
        buyerAuth.publicKey.toBuffer()
      ],
      program.programId
    );
    // State
    [state, stateBump] = await PublicKey.findProgramAddress(
      [
        Buffer.from(anchor.utils.bytes.utf8.encode("fortune")),
      ],
      program.programId
    );
  });

  it('Initialize program', async () => {
    const tx = await program.rpc.initialize(
      swapFee,
      burnCost,
      feeScalar,
      splMin,
      splMax,
      ptokenMax,
      ptokenMin,
      {
        accounts: {
          signer: fortuneAuth.publicKey,
          splVault: fortuneVault,
          splMint: NATIVE_MINT,
          state: state,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [fortuneAuth]
      });
  });

  it('Create pool', async () => {
    const tx = await program.rpc.createPool(
      splAmount,
      ptokenAmount,
      {
        accounts: {
          signer: creatorAuth.publicKey,
          nftAccount: nftAccount,
          probPool: probPool.publicKey,
          ptokenMint: ptokenMint,
          nftVault: nftVault,
          splVault: splVault,
          ptokenVault: ptokenVault,
          nftMint: nftMint.publicKey,
          nativeMint: NATIVE_MINT,
          state: state,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [creatorAuth, probPool]
      });
    // Pool initialized correctly
    let _pool = await program.account.probPool.fetch(probPool.publicKey)
    assert.ok(_pool.authority.equals(creatorAuth.publicKey))
    assert.ok(_pool.nftAuthority.equals(creatorAuth.publicKey))
    assert.ok(_pool.splVault.equals(splVault))
    assert.ok(_pool.ptokenVault.equals(ptokenVault))
    assert.ok(_pool.ptokenMint.equals(ptokenMint))
    assert.ok(_pool.nftMint.equals(nftMint.publicKey))
    assert.ok(_pool.claimed == false)
    assert.ok(_pool.splSupply.eq(splAmount))
    assert.ok(_pool.ptokenSupply.eq(ptokenAmount))
    assert.ok(_pool.outstandingPtokens.toNumber() == 0)
  });

  it('Buy', async () => {
    const tx = await program.rpc.buy(
      buyAmount,
      {
        accounts: {
          signer: buyerAuth.publicKey,
          poolSpl: splVault,
          poolPtoken: ptokenVault,
          probPool: probPool.publicKey,
          fortuneSpl: fortuneVault,
          userPtoken: userPtokenVault,
          ptokenMint: ptokenMint,
          nativeMint: NATIVE_MINT,
          state: state,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [buyerAuth]
      });
    // User ptoken vault received tokens
    let _userBalance = await provider.connection.getTokenAccountBalance(userPtokenVault)
    assert.ok(_userBalance.value.amount == buyAmount.toString())
    // Pool ptoken vault sold tokens
    let _poolBalance = await provider.connection.getTokenAccountBalance(ptokenVault)
    assert.ok(_poolBalance.value.amount == (ptokenAmount.sub(buyAmount)).toString())
    // Pool metadata updated
    let _pool = await program.account.probPool.fetch(probPool.publicKey)
    assert.ok(_pool.ptokenSupply.eq(ptokenAmount.sub(buyAmount)))
    assert.ok(_pool.outstandingPtokens.eq(buyAmount))
    // TODO: SPL amount
    let _splBalance = await provider.connection.getTokenAccountBalance(splVault)
    let _poolSpl = await provider.connection.getParsedAccountInfo(splVault)
    let _fortuneBalance = await provider.connection.getTokenAccountBalance(fortuneVault)
    console.log(_splBalance)
    console.log(_poolSpl)
    console.log(_fortuneBalance)
  });

  it('Request Burn', async () => {
    const tx = await program.rpc.requestBurn(
      burnAmount,
      {
        accounts: {
          signer: buyerAuth.publicKey,
          userPtokenVault: userPtokenVault,
          userBurn: userBurn,
          probPool: probPool.publicKey,
          ptokenMint: ptokenMint,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [buyerAuth]
      });
  });

  it('User Withdraw', async () => {
    // Add your test here.
    const tx = await program.rpc.userWithdraw(
      withdrawAmount,
      {
        accounts: {
          signer: buyerAuth.publicKey,
          userVault: userPtokenVault,
          userAccount: userPtokenAccount.publicKey,
          probPool: probPool.publicKey,
          vaultMint: ptokenMint,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [buyerAuth, userPtokenAccount]
      });
  });

  it('Execute Burn', async () => {
    // console.log(userBurn.toBase58())
    // console.log(probPool.publicKey.toBase58())
    // console.log(nftVault.toBase58())
    // console.log(userNftVault.toBase58())
    // console.log(buyerAuth.publicKey.toBase58())
    // console.log(nftMint.publicKey.toBase58())
    // console.log(carolineAuth.publicKey.toBase58())
    // console.log(ptokenMint.toBase58())
    const tx = await program.rpc.executeBurn(
      burnAmount,
      {
        accounts: {
          carolineAuthority: fortuneAuth.publicKey,
          user: buyerAuth.publicKey,
          nftVault: nftVault,
          userBurn: userBurn,
          probPool: probPool.publicKey,
          nftMint: nftMint.publicKey,
          ptokenMint: ptokenMint,
          recentBlockhashes: SYSVAR_RECENT_BLOCKHASHES_PUBKEY,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [fortuneAuth]
      });
  });

  it('User claim nft', async () => {
    const tx = await program.rpc.claimAsset(
      {
        accounts: {
          signer: buyerAuth.publicKey,
          nftAccount: buyerNftAccount.publicKey,
          probPool: probPool.publicKey,
          nftVault: nftVault,
          nftMint: nftMint.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [buyerAuth, buyerNftAccount]
      });
  });

  it('Close pool', async () => {
    const tx = await program.rpc.closePool(
      {
        accounts: {
          signer: creatorAuth.publicKey,
          splAccount: creatorSplAccount.publicKey,
          nftAccount: creatorNftAccount.publicKey,
          probPool: probPool.publicKey,
          ptokenMint: ptokenMint,
          nftVault: nftVault,
          splVault: splVault,
          ptokenVault: ptokenVault,
          nftMint: nftMint.publicKey,
          nativeMint: NATIVE_MINT,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY
        },
        signers: [creatorAuth, creatorSplAccount, creatorNftAccount]
      });
  });
});

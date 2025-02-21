import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ImgToken } from "../target/types/img_token";
import { 
  TOKEN_PROGRAM_ID, 
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint, 
  createAccount, 
  getAssociatedTokenAddress 
} from "@solana/spl-token";
import { expect } from "chai";
import 'mocha';

describe("img-token", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ImgToken as Program<ImgToken>;
  
  let mint: anchor.web3.PublicKey;
  let tokenConfig: anchor.web3.PublicKey;
  let taxVault: anchor.web3.PublicKey;
  let rewardVault: anchor.web3.PublicKey;
  let authority: anchor.web3.Keypair;
  let authorityAta: anchor.web3.PublicKey;

  before(async () => {
    authority = anchor.web3.Keypair.generate();

    // Airdrop SOL to authority
    const signature = await provider.connection.requestAirdrop(
      authority.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(signature);

    // Create mint
    mint = await createMint(
      provider.connection,
      authority,
      authority.publicKey,
      null,
      9
    );

    // Get token config PDA
    [tokenConfig] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("token_config"), mint.toBuffer()],
      program.programId
    );

    // Get tax vault PDA
    [taxVault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("tax_vault"), mint.toBuffer()],
      program.programId
    );

    // Get reward vault
    [rewardVault] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("reward_vault"), mint.toBuffer()],
      program.programId
    );

    // Get authority ATA
    authorityAta = await getAssociatedTokenAddress(
      mint,
      authority.publicKey
    );
  });

  it("Initializes token with correct config", async () => {
    await program.methods
      .initialize("Image Token", "IMG", 9)
      .accounts({
        authority: authority.publicKey,
        mint,
        authorityAta,
        tokenConfig,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([authority])
      .rpc();

    const config = await program.account.tokenConfig.fetch(tokenConfig);
    expect(config.name).to.equal("Image Token");
    expect(config.symbol).to.equal("IMG");
    expect(config.taxRate).to.equal(500); // 5%
  });

  it("Transfers tokens with correct tax", async () => {
    // test transfer
  });

  it("Swaps tax tokens for SOL", async () => {
    // test swap
  });

  it("Distributes rewards correctly", async () => {
    // test distribution
  });
}); 
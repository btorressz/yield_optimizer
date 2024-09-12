// No imports needed: web3, anchor, pg, and more are globally available

describe("Yield Optimizer Tests", () => {
  // Set up common variables
  const userKeypair = new web3.Keypair();
  const mintKeypair = new web3.Keypair();
  const userTokenAccountKeypair = new web3.Keypair();
  let userFundsPda, bump;
  
  // Derive the PDA for user funds based on the user's public key
  before(async () => {
    [userFundsPda, bump] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("user-funds"), userKeypair.publicKey.toBuffer()],
      pg.program.programId
    );
  });

  it("initializes user funds", async () => {
    const txHash = await pg.program.methods
      .initializeUserFunds()
      .accounts({
        userFunds: userFundsPda,
        user: userKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([userKeypair])
      .rpc();
    console.log(Transaction successful: ${txHash});

    // Confirm transaction
    await pg.connection.confirmTransaction(txHash);

    // Fetch the user funds account
    const userFundsAccount = await pg.program.account.userFunds.fetch(userFundsPda);

    // Check the fields
    assert.ok(userFundsAccount.owner.equals(userKeypair.publicKey));
    assert.equal(userFundsAccount.balances.length, 0);
    assert.equal(userFundsAccount.currentProtocol.toString(), web3.PublicKey.default.toString());
    console.log("User funds account initialized successfully");
  });

  it("optimizes yield across protocols", async () => {
    const newProtocolPubkey = new web3.PublicKey("SomeProtocolPubkey"); // Replace with actual protocol Pubkey

    // Fund the user token account with some tokens for the test
    const amount = new BN(1000); // 1000 tokens as an example

    const txHash = await pg.program.methods
      .optimizeYield(newProtocolPubkey, mintKeypair.publicKey, amount)
      .accounts({
        userFunds: userFundsPda,
        guard: web3.Keypair.generate().publicKey, // Assume guard is already created; generate a new one for example
        userTokenAccount: userTokenAccountKeypair.publicKey,
        expectedMint: mintKeypair.publicKey,
        tokenProgram: web3.TokenInstructions.TOKEN_PROGRAM_ID,
        currentProtocol: new web3.PublicKey("Pubkey"), // Replace with actual Pubkey
        newProtocol: newProtocolPubkey,
        governance: web3.Keypair.generate().publicKey, // Assume governance is already created; generate a new one for example
        user: userKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([userKeypair])
      .rpc();
    console.log(Transaction successful: ${txHash});

    // Confirm transaction
    await pg.connection.confirmTransaction(txHash);

    // Fetch the user funds account to verify changes
    const userFundsAccount = await pg.program.account.userFunds.fetch(userFundsPda);

    // Check the fields
    assert.ok(userFundsAccount.currentProtocol.equals(newProtocolPubkey));
    console.log("Yield optimization completed successfully");
  });
});
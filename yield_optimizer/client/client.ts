// Minimal client to interact with Solana Playground for the Yield Optimizer Program

async function main() {
  try {
    // Display wallet information and balance
    console.log("My address:", pg.wallet.publicKey.toString());
    const balance = await pg.connection.getBalance(pg.wallet.publicKey);
    console.log(`My balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);

    // Step 1: Initialize User Funds
    await initializeUserFunds();

    // Step 2: Fetch User Funds Data
    const [userFundsPda] = await web3.PublicKey.findProgramAddress(
      [Buffer.from("user-funds"), pg.wallet.publicKey.toBuffer()],
      pg.program.programId
    );
    await fetchUserFundsData(userFundsPda);

  } catch (err) {
    console.error("Error in client:", err);
  }
}

// Initialize user funds on-chain
async function initializeUserFunds() {
  const userKeypair = web3.Keypair.generate(); // Generate a new user keypair
  const [userFundsPda, bump] = await web3.PublicKey.findProgramAddress(
    [Buffer.from("user-funds"), userKeypair.publicKey.toBuffer()],
    pg.program.programId
  );

  console.log(`Initializing user funds for: ${userKeypair.publicKey.toString()}`);

  try {
    const txHash = await pg.program.methods
      .initializeUserFunds()
      .accounts({
        userFunds: userFundsPda,
        user: userKeypair.publicKey,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([userKeypair])
      .rpc();
    console.log(`User funds initialized with transaction: ${txHash}`);
  } catch (err) {
    console.error("Error initializing user funds:", err);
  }
}

// Fetch and display user funds account data
async function fetchUserFundsData(userFundsPda: web3.PublicKey) {
  try {
    const userFundsAccount = await pg.program.account.userFunds.fetch(userFundsPda);

    // Parse and display user funds account data
    console.log("UserFunds account data:", {
      owner: userFundsAccount.owner.toString(),
      balances: userFundsAccount.balances.map((bal: any) => ({
        pubkey: bal.pubkey.toString(),
        amount: bal.amount.toString(),
      })),
      currentProtocol: userFundsAccount.currentProtocol.toString(),
      lastReallocation: userFundsAccount.lastReallocation.toString(),
    });
  } catch (err) {
    console.error("Error fetching user funds data:", err);
  }
}

main();

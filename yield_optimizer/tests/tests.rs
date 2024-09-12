use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, TokenAccount, Mint, Token};
use solana_program::pubkey::Pubkey;
use anchor_lang::prelude::borsh::BorshSerialize;
use solana_program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use yield_optimizer::*;

#[tokio::test]
async fn test_initialize_user_funds() {
    // Initialize the test environment
    let mut test = ProgramTest::new(
        "yield_optimizer", 
        id(), 
        processor!(yield_optimizer::entry),
    );

    // Add necessary accounts and programs to the test environment
    test.add_program("spl_token", spl_token::id(), None);
    test.add_program("system_program", system_program::id(), None);

    // Create a user
    let user = Keypair::new();

    // Start the test environment
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Airdrop some SOL to the user to cover transaction fees
    let airdrop_tx = Transaction::new_signed_with_payer(
        &[solana_sdk::system_instruction::transfer(
            &payer.pubkey(),
            &user.pubkey(),
            1000000000, // 1 SOL
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(airdrop_tx).await.unwrap();

    // Get the Program Derived Address (PDA) for the user's funds
    let (user_funds_pda, _bump_seed) = Pubkey::find_program_address(
        &[b"user-funds", user.pubkey().as_ref()],
        &id(),
    );

    // Call the `initialize_user_funds` function
    let init_funds_tx = Transaction::new_signed_with_payer(
        &[
            yield_optimizer::instruction::initialize_user_funds(
                &id(),
                &user_funds_pda,
                &user.pubkey(),
                &system_program::id(),
            )
        ],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );
    banks_client.process_transaction(init_funds_tx).await.unwrap();

    // Check that the user's funds account was initialized correctly
    let user_funds_account = banks_client
        .get_account(user_funds_pda)
        .await
        .expect("Failed to fetch user funds account")
        .expect("User funds account not found");

    let user_funds_data = UserFunds::try_from_slice(&user_funds_account.data).unwrap();
    assert_eq!(user_funds_data.owner, user.pubkey());
    assert_eq!(user_funds_data.balances.len(), 0);
    assert_eq!(user_funds_data.current_protocol, Pubkey::default());
    println!("User funds account initialized successfully");
}

#[tokio::test]
async fn test_optimize_yield() {
    // Initialize the test environment
    let mut test = ProgramTest::new(
        "yield_optimizer", 
        id(), 
        processor!(yield_optimizer::entry),
    );

    test.add_program("spl_token", spl_token::id(), None);
    test.add_program("system_program", system_program::id(), None);

    // Create a user and protocols
    let user = Keypair::new();
    let protocol1 = Keypair::new(); // Dummy DeFi protocol
    let protocol2 = Keypair::new(); // Dummy DeFi protocol

    // Start the test environment
    let (mut banks_client, payer, recent_blockhash) = test.start().await;

    // Airdrop some SOL to the user for transaction fees
    let airdrop_tx = Transaction::new_signed_with_payer(
        &[solana_sdk::system_instruction::transfer(
            &payer.pubkey(),
            &user.pubkey(),
            1000000000, // 1 SOL
        )],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(airdrop_tx).await.unwrap();

    // Get the Program Derived Address (PDA) for the user's funds
    let (user_funds_pda, _bump_seed) = Pubkey::find_program_address(
        &[b"user-funds", user.pubkey().as_ref()],
        &id(),
    );

    // Initialize the user's funds account
    let init_funds_tx = Transaction::new_signed_with_payer(
        &[
            yield_optimizer::instruction::initialize_user_funds(
                &id(),
                &user_funds_pda,
                &user.pubkey(),
                &system_program::id(),
            )
        ],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );
    banks_client.process_transaction(init_funds_tx).await.unwrap();

    // Simulate optimizing yield between two protocols
    let optimize_tx = Transaction::new_signed_with_payer(
        &[
            yield_optimizer::instruction::optimize_yield(
                &id(),
                &user_funds_pda,
                &protocol1.pubkey(),  // Assuming protocol1 as current
                &protocol2.pubkey(),  // New protocol to switch to
                100, // Amount to reallocate (example amount)
            )
        ],
        Some(&payer.pubkey()),
        &[&payer, &user],
        recent_blockhash,
    );
    banks_client.process_transaction(optimize_tx).await.unwrap();

    // Check that the user's funds account has been updated with the new protocol
    let user_funds_account = banks_client
        .get_account(user_funds_pda)
        .await
        .expect("Failed to fetch user funds account")
        .expect("User funds account not found");

    let user_funds_data = UserFunds::try_from_slice(&user_funds_account.data).unwrap();
    assert_eq!(user_funds_data.current_protocol, protocol2.pubkey()); // Ensure funds reallocated to protocol2
    println!("Yield optimization completed successfully");
}

#[tokio::test]
async fn test_yield_rate_fetching() {
    // This test simulates yield rate fetching.
    // Normally, this would require an Oracle or off-chain data provider.

    // Simulate fetching a yield rate from an on-chain source
    let simulated_protocol_pubkey = Pubkey::new_unique();
    let yield_rate = fetch_yield_rate(simulated_protocol_pubkey).unwrap();
    assert_eq!(yield_rate, 5); // Ensure the fetched rate matches the simulated one
    println!("Yield rate fetching simulated successfully");
}

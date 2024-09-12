use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Token, Transfer};

// Declare your program ID
declare_id!("AooWjGFEpbgt8K4ZYcjXMFxcCcynMCLurHpPeJ5Gsnvq");

#[program]
mod yield_optimizer {
    use super::*;

    // Initialize user funds with a PDA
    pub fn initialize_user_funds(ctx: Context<InitializeUserFunds>) -> Result<()> {
        let user_funds = &mut ctx.accounts.user_funds;
        user_funds.owner = *ctx.accounts.user.key;
        user_funds.balances = Vec::new();
        user_funds.current_protocol = Pubkey::default();
        user_funds.last_reallocation = Clock::get()?.unix_timestamp;
        Ok(())
    }

    // Optimize yield across protocols
    pub fn optimize_yield(ctx: Context<OptimizeYield>, new_protocol: Pubkey, asset_mint: Pubkey, amount: u64) -> Result<()> {
        ctx.accounts.guard.start()?;  // Start reentrancy guard
        
        let now = Clock::get()?.unix_timestamp;
        require!(now - ctx.accounts.user_funds.last_reallocation >= MIN_REALLOCATION_PERIOD, YieldOptimizerError::ReallocationTooFrequent);

        let current_protocol = &ctx.accounts.current_protocol;
        let new_protocol = &ctx.accounts.new_protocol;

        // Fetch yield rates from an oracle
        let current_yield_rate = fetch_yield_rate_from_oracle(current_protocol.key())?;
        emit!(YieldRateFetched {
            protocol: current_protocol.key(),
            rate: current_yield_rate,
        });

        let new_yield_rate = fetch_yield_rate_from_oracle(new_protocol.key())?;
        emit!(YieldRateFetched {
            protocol: new_protocol.key(),
            rate: new_yield_rate,
        });

        // Compare and reallocate if beneficial
        if new_yield_rate > current_yield_rate {
            msg!("New protocol has a higher yield rate. Reallocating funds...");

            let fee_rate = ctx.accounts.governance.fee_rate;
            let net_amount = collect_fees(amount, fee_rate)?;

            // Withdraw from current protocol
            let cpi_withdraw_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                WithdrawFunds {
                    user_token_account: ctx.accounts.user_token_account.to_account_info(),  // Pass by reference
                    token_program: ctx.accounts.token_program.to_account_info(),  // Pass by reference
                }
            );
            withdraw_from_protocol(cpi_withdraw_ctx, amount)?;
            emit!(FundsWithdrawn {
                user: *ctx.accounts.user.key,
                protocol: current_protocol.key(),
                amount,
            });

            // Deposit into new protocol
            let cpi_deposit_ctx = CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                DepositFunds {
                    user_token_account: ctx.accounts.user_token_account.to_account_info(),  // Pass by reference
                    token_program: ctx.accounts.token_program.to_account_info(),  // Pass by reference
                }
            );
            deposit_to_protocol(cpi_deposit_ctx, net_amount)?;
            emit!(FundsDeposited {
                user: *ctx.accounts.user.key,
                protocol: new_protocol.key(),
                amount: net_amount,
            });

            // Emit an event to track the reallocation
            emit!(FundsReallocated {
                user: *ctx.accounts.user.key,
                from_protocol: ctx.accounts.current_protocol.key(),
                to_protocol: ctx.accounts.new_protocol.key(),
                amount: net_amount,
                timestamp: now,
            });

            // Update user funds with the new protocol and reallocation time
            ctx.accounts.user_funds.current_protocol = new_protocol.key();
            ctx.accounts.user_funds.last_reallocation = now;
        } else {
            return Err(YieldOptimizerError::LowerYieldRate.into());
        }

        ctx.accounts.guard.end();  // End reentrancy guard
        Ok(())
    }

    // Update governance parameters
    pub fn update_governance(ctx: Context<Governance>, new_fee_rate: u64) -> Result<()> {
        ctx.accounts.governance.fee_rate = new_fee_rate;
        Ok(())
    }
}

// Constants for your program
const MIN_REALLOCATION_PERIOD: i64 = 3600; // Minimum 1-hour gap between reallocations
const MAX_ASSETS: usize = 10; // Maximum number of assets a user can hold

// Enums for multiple protocols
#[derive(Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum Protocol {
    Raydium,
    Serum,
    Solend,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PubkeyAmount {
    pub pubkey: Pubkey,
    pub amount: u64,
}


// PDA account for user funds (multi-asset support)
#[account]
pub struct UserFunds {
    pub owner: Pubkey,               // The owner of the funds
 //   pub balances: Vec<(Pubkey, u64)>, // A list of token mints and their balances
    pub balances: Vec<PubkeyAmount>, 
    pub current_protocol: Pubkey,    // The protocol currently holding the funds
    pub last_reallocation: i64,      // Last reallocation timestamp
}

// Governance Account
#[account]
pub struct GovernanceAccount {
    pub authority: Pubkey, // The authority controlling governance
    pub fee_rate: u64,     // Fee rate in basis points (100 = 1%)
}

// Reentrancy Guard Account
#[account]
pub struct ReentrancyGuard {
    in_function: bool,
}

impl ReentrancyGuard {
    pub fn start(&mut self) -> Result<()> {
        if self.in_function {
            return Err(YieldOptimizerError::ReentrancyAttempt.into());
        }
        self.in_function = true;
        Ok(())
    }

    pub fn end(&mut self) {
        self.in_function = false;
    }
}

// Initialize user funds account
#[derive(Accounts)]
pub struct InitializeUserFunds<'info> {
    #[account(
        init,
        payer = user,
        space = 8 + 32 + (32 + 8) * MAX_ASSETS + 32 + 8, // Adjust space calculation
        seeds = [b"user-funds", user.key().as_ref()],
        bump
    )]
    pub user_funds: Account<'info, UserFunds>,
    #[account(init, payer = user, space = 8 + 1, seeds = [b"guard", user.key().as_ref()], bump)]
    pub guard: Account<'info, ReentrancyGuard>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Optimize yield context with accounts
#[derive(Accounts)]
pub struct OptimizeYield<'info> {
    #[account(mut, seeds = [b"user-funds", user.key().as_ref()], bump)]
    pub user_funds: Account<'info, UserFunds>,

    #[account(mut, seeds = [b"guard", user.key().as_ref()], bump)]
    pub guard: Account<'info, ReentrancyGuard>,

    #[account(mut)]
    pub user_token_account: AccountInfo<'info>, // Changed to AccountInfo

    #[account(
        constraint = {
            let token_account: TokenAccount = TokenAccount::try_deserialize(&mut &user_token_account.data.borrow()[..])?;
            token_account.mint == expected_mint.key() && token_account.owner == user.key()
        }
    )]
    pub expected_mint: AccountInfo<'info>,

    pub token_program: AccountInfo<'info>, // Changed to AccountInfo
    pub current_protocol: AccountInfo<'info>,
    pub new_protocol: AccountInfo<'info>,
    pub governance: Account<'info, GovernanceAccount>,
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// Governance context with authority
#[derive(Accounts)]
pub struct Governance<'info> {
    #[account(mut, has_one = authority)]
    pub governance: Account<'info, GovernanceAccount>,
    pub authority: Signer<'info>,
}

// Custom error definitions
#[error_code]
pub enum YieldOptimizerError {
    #[msg("Insufficient funds in the user account.")]
    InsufficientFunds,

    #[msg("Yield rate is lower in the new protocol.")]
    LowerYieldRate,

    #[msg("Reallocation too frequent, try again later.")]
    ReallocationTooFrequent,

    #[msg("Failed to withdraw from the current protocol.")]
    WithdrawalFailed,

    #[msg("Failed to deposit to the new protocol.")]
    DepositFailed,

    #[msg("Reentrancy attempt detected.")]
    ReentrancyAttempt,

    #[msg("Unsupported protocol.")]
    UnsupportedProtocol,

    #[msg("Unauthorized access.")]
    UnauthorizedAccess,
}

// Event to emit during funds reallocation
#[event]
pub struct FundsReallocated {
    pub user: Pubkey,
    pub from_protocol: Pubkey,
    pub to_protocol: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

// Event to emit when a yield rate is fetched
#[event]
pub struct YieldRateFetched {
    pub protocol: Pubkey,
    pub rate: u64,
}

// Event to emit when funds are withdrawn
#[event]
pub struct FundsWithdrawn {
    pub user: Pubkey,
    pub protocol: Pubkey,
    pub amount: u64,
}

// Event to emit when funds are deposited
#[event]
pub struct FundsDeposited {
    pub user: Pubkey,
    pub protocol: Pubkey,
    pub amount: u64,
}

// Simulated yield rate fetcher from an oracle (replace with actual logic)
fn fetch_yield_rate_from_oracle(_protocol: Pubkey) -> Result<u64> {
    // Simulate fetching yield rate from an oracle
    Ok(5) // Placeholder example (5% yield)
}

// Collects fees and returns the net amount after fees
fn collect_fees(amount: u64, fee_rate: u64) -> Result<u64> {
    let fee = (amount * fee_rate) / 10000; // Assuming fee_rate is in basis points (100 = 1%)
    Ok(amount - fee)
}

// Simulate protocol withdrawal
fn withdraw_from_protocol<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, WithdrawFunds<'info>>, 
    amount: u64
) -> Result<()> {
    msg!("Withdrawing {} tokens from the protocol...", amount);
    // Logic for withdrawal from external protocol via CPI
    Ok(())
}

// Simulate protocol deposit
fn deposit_to_protocol<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, DepositFunds<'info>>, 
    amount: u64
) -> Result<()> {
    msg!("Depositing {} tokens into the protocol...", amount);
    // Logic for deposit to external protocol via CPI
    Ok(())
}

// Withdraw Funds CPI context
#[derive(Accounts)]
pub struct WithdrawFunds<'info> {
    #[account(mut)]
    pub user_token_account: AccountInfo<'info>, // Changed to AccountInfo
    pub token_program: AccountInfo<'info>, // Changed to AccountInfo
}

// Deposit Funds CPI context
#[derive(Accounts)]
pub struct DepositFunds<'info> {
    #[account(mut)]
    pub user_token_account: AccountInfo<'info>, // Changed to AccountInfo
    pub token_program: AccountInfo<'info>, // Changed to AccountInfo
}

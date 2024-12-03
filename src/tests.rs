use super::*;
use solana_program::{
    instruction::{AccountMeta, Instruction, InstructionError},
    system_instruction,
    clock::Clock,
    sysvar::clock::ID as CLOCK_ID,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transaction::TransactionError,
    hash::Hash,
};
use std::mem;
use borsh::{BorshSerialize, BorshDeserialize};

async fn setup_test_context(initial_time: i64, slot: u64) -> (ProgramTestContext, Pubkey) {
    let mut program_test = ProgramTest::default();
    let program_id = Pubkey::new_unique();
    
    program_test.add_program("mr_fresh", program_id, processor!(process_instruction));
    let context = program_test.start_with_context().await;
    
    let clock = Clock {
        slot,
        epoch_start_timestamp: initial_time,
        epoch: 0,
        leader_schedule_epoch: 0,
        unix_timestamp: initial_time,
    };
    context.set_sysvar(&clock);
    
    println!("Debug: Test context setup with time {} and slot {}", initial_time, slot);
    (context, program_id)
}

async fn create_test_state(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    program_id: &Pubkey,
    _initial_time: i64,
) -> Result<Keypair, BanksClientError> {
    let state_account = Keypair::new();
    println!("Debug: Creating state account: {}", state_account.pubkey());
    
    let rent = banks_client.get_rent().await?;
    let account_size = mem::size_of::<MrFreshState>();
    let lamports = rent.minimum_balance(account_size);

    let create_account_ix = system_instruction::create_account(
        &payer.pubkey(),
        &state_account.pubkey(),
        lamports,
        account_size as u64,
        program_id,
    );

    let transaction = Transaction::new_signed_with_payer(
        &[create_account_ix],
        Some(&payer.pubkey()),
        &[payer, &state_account],
        *recent_blockhash,
    );

    banks_client.process_transaction(transaction).await?;

    let instruction_data = MrFreshInstruction::Initialize {
        mining_difficulty: 1000,
        energy_burst_duration: 100,
    };
    
    let mut buffer = Vec::new();
    instruction_data.serialize(&mut buffer).unwrap();

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(state_account.pubkey(), false),
            AccountMeta::new_readonly(CLOCK_ID, false),
        ],
        data: buffer,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        *recent_blockhash,
    );

    banks_client.process_transaction(transaction).await?;
    Ok(state_account)
}

fn calculate_expected_reward(initial_time: i64, current_time: i64, mining_difficulty: u64) -> u64 {
    let time_since_start = current_time.saturating_sub(initial_time);
    let halving_epoch = time_since_start / HALVING_INTERVAL;
    
    let mut current_base_reward = INITIAL_BASE_REWARD;
    for _ in 0..halving_epoch {
        current_base_reward = current_base_reward.saturating_div(2);
    }
    
    current_base_reward.saturating_div(mining_difficulty)
}

fn create_mine_instruction(
    program_id: &Pubkey,
    state_account: &Keypair,
    miner: &Keypair,
) -> Instruction {
    println!("Debug: Creating mine instruction");
    println!("Debug: State account: {}", state_account.pubkey());
    println!("Debug: Miner account: {}", miner.pubkey());
    
    let mut buffer = Vec::new();
    let instruction_data = MrFreshInstruction::Mine;
    instruction_data.serialize(&mut buffer).unwrap();
    
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(state_account.pubkey(), false),
            AccountMeta::new(miner.pubkey(), true),
            AccountMeta::new_readonly(CLOCK_ID, false),
        ],
        data: buffer,
    }
}

async fn process_mining_transaction(
    banks_client: &mut BanksClient,
    instruction: Instruction,
    payer: &Keypair,
    miner: &Keypair,
    recent_blockhash: Hash,
) -> Result<(), BanksClientError> {
    println!("Debug: Processing mining transaction");
    println!("Debug: Payer: {}", payer.pubkey());
    println!("Debug: Miner: {}", miner.pubkey());
    
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer, miner],
        recent_blockhash,
    );
    
    banks_client.process_transaction(transaction).await
}

async fn verify_mining_result(
    banks_client: &mut BanksClient,
    state_account: &Keypair,
    expected_reward: Option<u64>,
) -> Result<MrFreshState, BanksClientError> {
    println!("Debug: Verifying mining result");
    let account = banks_client.get_account(state_account.pubkey()).await?.unwrap();
    let state = MrFreshState::try_from_slice(&account.data).unwrap();
    
    println!("Debug: Current state:");
    println!("Debug: Total supply: {}", state.total_supply);
    println!("Debug: Last mining timestamp: {}", state.last_mining_timestamp);
    println!("Debug: Total transactions: {}", state.total_transactions);
    
    if let Some(expected) = expected_reward {
        assert_eq!(
            state.total_supply,
            expected,
            "Mining reward mismatch. Expected: {}, Got: {}",
            expected,
            state.total_supply
        );
    }
    
    Ok(state)
}

#[tokio::test]
async fn test_initialization() {
    println!("\n=== Running Initialization Test ===");
    let initial_time = 0;
    let (mut context, program_id) = setup_test_context(initial_time, 0).await;
    
    let result = create_test_state(
        &mut context.banks_client,
        &context.payer,
        &context.last_blockhash,
        &program_id,
        initial_time,
    ).await;
    
    assert!(result.is_ok(), "Failed to initialize state");
    
    if let Ok(state_account) = result {
        let state = verify_mining_result(&mut context.banks_client, &state_account, Some(0))
            .await
            .unwrap();
        assert_eq!(state.total_supply, 0);
        assert_eq!(state.total_transactions, 0);
        assert_eq!(state.last_mining_timestamp, 0);
        assert_eq!(state.initialization_timestamp, initial_time);
    }
}

#[tokio::test]
async fn test_mining_cooldown() {
    println!("\n=== Running Mining Cooldown Test ===");
    let initial_time = 1000;
    let (mut context, program_id) = setup_test_context(initial_time, 1).await;
    let payer = context.payer.insecure_clone();
    let miner = Keypair::new();

    let state_account = create_test_state(
        &mut context.banks_client,
        &payer,
        &context.last_blockhash,
        &program_id,
        initial_time,
    ).await.unwrap();

    // First mining attempt
    println!("Debug: Attempting first mine operation");
    let mine_instruction = create_mine_instruction(&program_id, &state_account, &miner);
    let result = process_mining_transaction(
        &mut context.banks_client,
        mine_instruction,
        &payer,
        &miner,
        context.last_blockhash,
    ).await;

    assert!(result.is_ok(), "First mining attempt failed");
    
    // Try mining during cooldown period
    let cooldown_time = initial_time + (MINING_COOLDOWN - 600);
    context.set_sysvar(&Clock {
        slot: 2,
        epoch_start_timestamp: initial_time,
        epoch: 0,
        leader_schedule_epoch: 0,
        unix_timestamp: cooldown_time,
    });
    
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    
    println!("Debug: Attempting mining during cooldown");
    let mine_instruction = create_mine_instruction(&program_id, &state_account, &miner);
    let result = process_mining_transaction(
        &mut context.banks_client,
        mine_instruction,
        &payer,
        &miner,
        context.last_blockhash,
    ).await;

    assert!(
        matches!(
            result,
            Err(BanksClientError::TransactionError(
                TransactionError::InstructionError(_, InstructionError::Custom(err))
            )) if err == FreshError::CooldownActive as u32
        ),
        "Expected cooldown error, got: {:?}",
        result
    );
}

#[tokio::test]
async fn test_halving() {
    println!("\n=== Running Halving Test ===");
    let initial_time = 0;
    let mining_difficulty = 1000;
    let (mut context, program_id) = setup_test_context(initial_time, 1).await;
    let payer = context.payer.insecure_clone();
    let miner = Keypair::new();

    let state_account = create_test_state(
        &mut context.banks_client,
        &payer,
        &context.last_blockhash,
        &program_id,
        initial_time,
    ).await.unwrap();

    // Test mining before first halving
    let initial_reward = calculate_expected_reward(initial_time, initial_time, mining_difficulty);
    let mine_instruction = create_mine_instruction(&program_id, &state_account, &miner);
    let result = process_mining_transaction(
        &mut context.banks_client,
        mine_instruction,
        &payer,
        &miner,
        context.last_blockhash,
    ).await;
    assert!(result.is_ok(), "Initial mining attempt failed");

    // Test mining after first halving
    let time_after_halving = initial_time + HALVING_INTERVAL + 1;
    context.set_sysvar(&Clock {
        slot: 2,
        epoch_start_timestamp: initial_time,
        epoch: 0,
        leader_schedule_epoch: 0,
        unix_timestamp: time_after_halving,
    });
    
    context.last_blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    
    let halved_reward = calculate_expected_reward(initial_time, time_after_halving, mining_difficulty);
    assert_eq!(halved_reward, initial_reward / 2, "Halving calculation incorrect");

    let mine_instruction = create_mine_instruction(&program_id, &state_account, &miner);
    let result = process_mining_transaction(
        &mut context.banks_client,
        mine_instruction,
        &payer,
        &miner,
        context.last_blockhash,
    ).await;
    assert!(result.is_ok(), "Mining after halving failed");
}
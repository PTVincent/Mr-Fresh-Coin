#[cfg(test)]
mod tests;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::Clock,
    sysvar::{Sysvar, SysvarId},
};
use borsh::{BorshDeserialize, BorshSerialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FreshError {
    #[error("Cooldown is still active")]
    CooldownActive,
    #[error("Oh no! Poop discovered!")]
    PoopDiscovered,
    #[error("Invalid instruction data")]
    InvalidInstruction,
    #[error("Mining difficulty too low")]
    DifficultyTooLow,
    #[error("Maximum supply reached")]
    MaxSupplyReached,
}

impl From<FreshError> for ProgramError {
    fn from(e: FreshError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// Original constants
pub const MINING_COOLDOWN: i64 = 1800;        // 30 minutes
pub const ENERGY_BURST_BONUS: u64 = 150;      // 50% bonus
pub const LUCKY_PURR_CHANCE: u64 = 100;       // 1% chance
pub const LUCKY_PURR_BONUS: u64 = 110;        // 10% bonus
pub const MIN_DIFFICULTY: u64 = 100;          // Minimum mining difficulty

// New supply and halving constants
pub const HALVING_INTERVAL: i64 = 31_536_000;  // 365 days in seconds
pub const MAX_SUPPLY: u64 = 50_000_000_000_000_000;  // 50 million with 9 decimals
pub const INITIAL_BASE_REWARD: u64 = 10_000_000;     // Initial mining reward

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MrFreshState {
    pub total_supply: u64,
    pub mining_difficulty: u64,
    pub last_mining_timestamp: i64,
    pub total_miners: u64,
    pub total_transactions: u64,
    pub last_energy_burst_slot: u64,
    pub energy_burst_duration: u64,
    pub initialization_timestamp: i64,    // New field for tracking program start
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum MrFreshInstruction {
    Initialize {
        mining_difficulty: u64,
        energy_burst_duration: u64,
    },
    Mine,
    UpdateDifficulty {
        new_difficulty: u64,
    },
}

entrypoint!(process_instruction);

fn calculate_mining_reward(state: &MrFreshState, current_time: i64) -> Result<u64, ProgramError> {
    // Check if max supply reached
    if state.total_supply >= MAX_SUPPLY {
        msg!("Maximum supply of 50 million FRESH tokens reached!");
        return Err(FreshError::MaxSupplyReached.into());
    }

    // Calculate time since initialization
    let time_since_start = current_time.saturating_sub(state.initialization_timestamp);
    let halving_epoch = time_since_start / HALVING_INTERVAL;
    
    // Calculate current base reward with halving
    let mut current_base_reward = INITIAL_BASE_REWARD;
    for _ in 0..halving_epoch {
        current_base_reward = current_base_reward.saturating_div(2);
    }

    // If base reward has been reduced to zero due to halvings, return error
    if current_base_reward == 0 {
        msg!("Mining rewards have reached minimum threshold");
        return Err(FreshError::MaxSupplyReached.into());
    }

    // Calculate final reward based on difficulty
    let reward = current_base_reward.saturating_div(state.mining_difficulty);
    
    msg!("Debug: Reward calculation:");
    msg!("  Time since start: {} seconds", time_since_start);
    msg!("  Current halving epoch: {}", halving_epoch);
    msg!("  Current base reward: {}", current_base_reward);
    msg!("  Final reward after difficulty: {}", reward);
    
    Ok(reward)
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Processing instruction: {:?}", instruction_data);
    
    let instruction = MrFreshInstruction::try_from_slice(instruction_data)
        .map_err(|_| FreshError::InvalidInstruction)?;

    match instruction {
        MrFreshInstruction::Initialize { mining_difficulty, energy_burst_duration } => {
            if mining_difficulty < MIN_DIFFICULTY {
                return Err(FreshError::DifficultyTooLow.into());
            }
            process_initialize(program_id, accounts, mining_difficulty, energy_burst_duration)
        }
        MrFreshInstruction::Mine => {
            process_mining(program_id, accounts)
        }
        MrFreshInstruction::UpdateDifficulty { new_difficulty } => {
            if new_difficulty < MIN_DIFFICULTY {
                return Err(FreshError::DifficultyTooLow.into());
            }
            process_update_difficulty(program_id, accounts, new_difficulty)
        }
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    mining_difficulty: u64,
    energy_burst_duration: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let state_account = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;

    if state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let clock = Clock::from_account_info(clock_sysvar)?;
    
    let state = MrFreshState {
        total_supply: 0,
        mining_difficulty,
        last_mining_timestamp: 0,
        total_miners: 0,
        total_transactions: 0,
        last_energy_burst_slot: 0,
        energy_burst_duration,
        initialization_timestamp: clock.unix_timestamp,
    };

    state.serialize(&mut &mut state_account.data.borrow_mut()[..])?;
    msg!("🐱 Mr. Fresh token initialized successfully!");
    Ok(())
}

fn is_energy_burst_active(clock: &Clock, state: &MrFreshState) -> bool {
    let slot_since_last = clock.slot.saturating_sub(state.last_energy_burst_slot);
    let mod_check = clock.slot % 41 == 0;
    let duration_check = state.last_energy_burst_slot == 0 || slot_since_last >= state.energy_burst_duration;
    let is_active = duration_check && mod_check;
    
    msg!("Debug: Energy burst check details:");
    msg!("  Current slot: {}", clock.slot);
    msg!("  Last burst slot: {}", state.last_energy_burst_slot);
    msg!("  Slots since last: {}", slot_since_last);
    msg!("  Duration threshold: {}", state.energy_burst_duration);
    msg!("  Modulo check (slot % 41 == 0): {}", mod_check);
    msg!("  Duration check result: {}", duration_check);
    msg!("  Final is_active result: {}", is_active);
    
    is_active
}

fn process_mining(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let state_account = next_account_info(account_iter)?;
    let _miner_account = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;

    if state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if clock_sysvar.key != &Clock::id() {
        msg!("Expected Clock sysvar");
        return Err(ProgramError::InvalidArgument);
    }

    let mut state = MrFreshState::try_from_slice(&state_account.data.borrow())?;
    let clock = Clock::from_account_info(clock_sysvar)?;
    let current_time = clock.unix_timestamp;

    // Check cooldown period
    let time_since_last = current_time.saturating_sub(state.last_mining_timestamp);
    if state.last_mining_timestamp != 0 && time_since_last < MINING_COOLDOWN {
        let remaining_time = MINING_COOLDOWN - time_since_last;
        msg!("😴 Shhh... Mr. Fresh is taking a proper cat nap!");
        msg!("He needs {:.1} more minutes of sleep!", remaining_time as f64 / 60.0);
        return Err(FreshError::CooldownActive.into());
    }

    let slot = clock.slot;
    if slot != 0 && slot % 10 == 0 && slot < 1000 {
        msg!("🙀 Oh no! Mr. Fresh found 💩 in the food! Mining failed!");
        return Err(FreshError::PoopDiscovered.into());
    }

    // Calculate base reward with halving
    let mut reward = calculate_mining_reward(&state, current_time)?;

    // Apply bonus mechanisms
    if is_energy_burst_active(&clock, &state) {
        msg!("⚡ Mr. Fresh is full of energy! Bonus rewards active!");
        let pre_bonus_reward = reward;
        reward = reward.saturating_mul(ENERGY_BURST_BONUS).saturating_div(100);
        msg!("  Pre-bonus reward: {}", pre_bonus_reward);
        msg!("  After energy burst bonus: {}", reward);
        state.last_energy_burst_slot = slot;
    }

    if slot % LUCKY_PURR_CHANCE == 0 {
        msg!("😺 *purrrrrr* Mr. Fresh is extra happy! Lucky bonus!");
        let pre_purr_reward = reward;
        reward = reward.saturating_mul(LUCKY_PURR_BONUS).saturating_div(100);
        msg!("  Pre-purr reward: {}", pre_purr_reward);
        msg!("  After lucky purr bonus: {}", reward);
    }

    // Ensure reward wouldn't exceed max supply
    if state.total_supply.saturating_add(reward) > MAX_SUPPLY {
        reward = MAX_SUPPLY.saturating_sub(state.total_supply);
    }

    // Update state
    state.last_mining_timestamp = current_time;
    state.total_supply = state.total_supply.saturating_add(reward);
    state.total_transactions = state.total_transactions.saturating_add(1);

    state.serialize(&mut &mut state_account.data.borrow_mut()[..])?;
    msg!("🐱 Mining successful! Earned {} FRESH tokens!", reward);
    Ok(())
}

fn process_update_difficulty(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_difficulty: u64,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let state_account = next_account_info(account_iter)?;

    if state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut state = MrFreshState::try_from_slice(&state_account.data.borrow())?;
    msg!("Debug: Updating difficulty from {} to {}", state.mining_difficulty, new_difficulty);
    state.mining_difficulty = new_difficulty;
    state.serialize(&mut &mut state_account.data.borrow_mut()[..])?;

    msg!("🐱 Mining difficulty updated to: {}", new_difficulty);
    Ok(())
}
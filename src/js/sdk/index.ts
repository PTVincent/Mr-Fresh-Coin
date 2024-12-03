import {
    Connection,
    PublicKey,
    Transaction,
    SystemProgram,
    TransactionInstruction,
    Keypair,
    sendAndConfirmTransaction,
  } from '@solana/web3.js';
  import { Buffer } from 'buffer';
  
  export class MrFreshSDK {
    private connection: Connection;
    private programId: PublicKey;
  
    constructor(
      connection: Connection,
      programId: string | PublicKey
    ) {
      this.connection = connection;
      this.programId = typeof programId === 'string' ? new PublicKey(programId) : programId;
    }
  
    async initialize(
      payer: Keypair,
      stateAccount: Keypair,
      miningDifficulty: number = 1000,
      energyBurstDuration: number = 100
    ): Promise<string> {
      const data = Buffer.from([
        0, // Initialize instruction
        ...new Uint8Array(new Uint32Array([miningDifficulty]).buffer),
        ...new Uint8Array(new Uint32Array([energyBurstDuration]).buffer),
      ]);
  
      const instruction = new TransactionInstruction({
        keys: [{ pubkey: stateAccount.publicKey, isSigner: false, isWritable: true }],
        programId: this.programId,
        data,
      });
  
      const transaction = new Transaction().add(instruction);
      
      return await sendAndConfirmTransaction(
        this.connection,
        transaction,
        [payer, stateAccount],
        { commitment: 'confirmed' }
      );
    }
  
    async mine(
      payer: Keypair,
      stateAccount: PublicKey,
      minerAccount: Keypair
    ): Promise<string> {
      const data = Buffer.from([1]); // Mine instruction
  
      const instruction = new TransactionInstruction({
        keys: [
          { pubkey: stateAccount, isSigner: false, isWritable: true },
          { pubkey: minerAccount.publicKey, isSigner: true, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: this.programId,
        data,
      });
  
      const transaction = new Transaction().add(instruction);
      
      return await sendAndConfirmTransaction(
        this.connection,
        transaction,
        [payer, minerAccount],
        { commitment: 'confirmed' }
      );
    }
  
    async updateDifficulty(
      payer: Keypair,
      stateAccount: PublicKey,
      newDifficulty: number
    ): Promise<string> {
      const data = Buffer.from([
        2, // UpdateDifficulty instruction
        ...new Uint8Array(new Uint32Array([newDifficulty]).buffer),
      ]);
  
      const instruction = new TransactionInstruction({
        keys: [{ pubkey: stateAccount, isSigner: false, isWritable: true }],
        programId: this.programId,
        data,
      });
  
      const transaction = new Transaction().add(instruction);
      
      return await sendAndConfirmTransaction(
        this.connection,
        transaction,
        [payer],
        { commitment: 'confirmed' }
      );
    }
  
    async getState(stateAccount: PublicKey): Promise<{
      totalSupply: number;
      miningDifficulty: number;
      lastMiningTimestamp: number;
      totalMiners: number;
      totalTransactions: number;
      lastEnergyBurstSlot: number;
      energyBurstDuration: number;
    }> {
      const accountInfo = await this.connection.getAccountInfo(stateAccount);
      if (!accountInfo) {
        throw new Error('State account not found');
      }
  
      // Parse the state data according to your Rust structure
      const data = accountInfo.data;
      return {
        totalSupply: Number(data.readBigUInt64LE(0)),
        miningDifficulty: Number(data.readBigUInt64LE(8)),
        lastMiningTimestamp: Number(data.readBigInt64LE(16)),
        totalMiners: Number(data.readBigUInt64LE(24)),
        totalTransactions: Number(data.readBigUInt64LE(32)),
        lastEnergyBurstSlot: Number(data.readBigUInt64LE(40)),
        energyBurstDuration: Number(data.readBigUInt64LE(48)),
      };
    }
  }
import { program } from 'commander';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { MrFreshSDK } from '../sdk';
import fs from 'fs';

program
  .version('0.1.0')
  .description('Mr. Fresh (FRESH) CLI');

program
  .command('init')
  .description('Initialize Mr. Fresh state')
  .requiredOption('-k, --keypair <path>', 'Keypair file path')
  .option('-d, --difficulty <number>', 'Initial mining difficulty', '1000')
  .option('-b, --burst-duration <number>', 'Energy burst duration', '100')
  .action(async (options) => {
    const connection = new Connection('http://localhost:8899', 'confirmed');
    const payerKeypair = Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(fs.readFileSync(options.keypair, 'utf-8')))
    );
    
    const sdk = new MrFreshSDK(connection, process.env.PROGRAM_ID!);
    const stateAccount = new Keypair();
    
    try {
      const tx = await sdk.initialize(
        payerKeypair,
        stateAccount,
        parseInt(options.difficulty),
        parseInt(options.burstDuration)
      );
      console.log('Initialization successful!');
      console.log('Transaction:', tx);
      console.log('State Account:', stateAccount.publicKey.toBase58());
    } catch (error) {
      console.error('Error:', error);
    }
  });

program
  .command('mine')
  .description('Mine FRESH tokens')
  .requiredOption('-k, --keypair <path>', 'Keypair file path')
  .requiredOption('-s, --state <pubkey>', 'State account public key')
  .action(async (options) => {
    const connection = new Connection('http://localhost:8899', 'confirmed');
    const payerKeypair = Keypair.fromSecretKey(
      new Uint8Array(JSON.parse(fs.readFileSync(options.keypair, 'utf-8')))
    );
    
    const sdk = new MrFreshSDK(connection, process.env.PROGRAM_ID!);
    const minerKeypair = new Keypair();
    
    try {
      const tx = await sdk.mine(
        payerKeypair,
        new PublicKey(options.state),
        minerKeypair
      );
      console.log('Mining successful!');
      console.log('Transaction:', tx);
    } catch (error) {
      console.error('Error:', error);
    }
  });

program
  .command('state')
  .description('Get current state')
  .requiredOption('-s, --state <pubkey>', 'State account public key')
  .action(async (options) => {
    const connection = new Connection('http://localhost:8899', 'confirmed');
    const sdk = new MrFreshSDK(connection, process.env.PROGRAM_ID!);
    
    try {
      const state = await sdk.getState(new PublicKey(options.state));
      console.log('Current State:');
      console.log(JSON.stringify(state, null, 2));
    } catch (error) {
      console.error('Error:', error);
    }
  });

program.parse(process.argv);
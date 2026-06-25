import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StakeWeather } from "../target/types/stake_weather";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import fs from "fs";
import dotenv from "dotenv";
dotenv.config();

async function main() {
  const connection = new anchor.web3.Connection(
    process.env.RPC_URL || "https://api.devnet.solana.com"
  );

  const keypairFile = fs.readFileSync(process.env.CREATOR_KEYPAIR!);
  const keypair = anchor.web3.Keypair.fromSecretKey(
    Buffer.from(JSON.parse(keypairFile.toString()))
  );

  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {});
  anchor.setProvider(provider);

  const idl = JSON.parse(fs.readFileSync("../target/idl/stake_weather.json").toString());
  const programId = new PublicKey("Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5");
  const program = new Program<StakeWeather>(idl, provider);

  const [betPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bet"), keypair.publicKey.toBuffer()],
    programId
  );
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), keypair.publicKey.toBuffer()],
    programId
  );

   const tx = await program.methods
    .createBet(
      0, //mumbai 0, delhi 1 and blr 2
      250, // 30 * 10
      true, // below threshold
      new anchor.BN(Math.floor(Date.now() / 1000) + 60), // deadline
      new anchor.BN(1_000_000) //0.001 sol staked
    )
    .accounts({
      creator: keypair.publicKey,
    })
    .rpc();

  console.log("create_bet tx:", tx);
  console.log("bet PDA:", betPda.toBase58());
  console.log("vault PDA:", vaultPda.toBase58());
}

main();
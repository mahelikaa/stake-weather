import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StakeWeather } from "../target/types/stake_weather";
import { PublicKey } from "@solana/web3.js";
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
    const program = new Program<StakeWeather>(idl, provider);

    const programId = new PublicKey("Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5");

    const [betPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("bet"), keypair.publicKey.toBuffer()],
        programId
    );
    const [vaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), keypair.publicKey.toBuffer()],
        programId
    );

    const tx = await program.methods
        .cancelBet()
        .accountsPartial({
            bet: betPda,
            vault: vaultPda,
            creator: keypair.publicKey,
        })
        .rpc();

    console.log("cancel_bet tx:", tx);
}

main();
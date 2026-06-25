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

    const creatorFile = fs.readFileSync(process.env.CREATOR_KEYPAIR!);
    const creator = anchor.web3.Keypair.fromSecretKey(
        Buffer.from(JSON.parse(creatorFile.toString()))
    );

    const challengerFile = fs.readFileSync(process.env.CHALLENGER_KEYPAIR!);
    const challenger = anchor.web3.Keypair.fromSecretKey(
        Buffer.from(JSON.parse(challengerFile.toString()))
    );

    const wallet = new anchor.Wallet(challenger);
    const provider = new anchor.AnchorProvider(connection, wallet, {});
    anchor.setProvider(provider);

    const idl = JSON.parse(fs.readFileSync("../target/idl/stake_weather.json").toString());
    const program = new Program<StakeWeather>(idl, provider);

    const programId = new PublicKey("Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5");
    const [betPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("bet"), creator.publicKey.toBuffer()],
        programId
    );
    const [vaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), creator.publicKey.toBuffer()],
        programId
    );

    const tx = await program.methods
        .joinBet()
        .accountsPartial({
            bet: betPda,
            vault: vaultPda,
            creator: creator.publicKey,
            challenger: challenger.publicKey,
        })
        .rpc();

    console.log("join_bet tx:", tx);
}

main();

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

    const wallet = new anchor.Wallet(creator);
    const provider = new anchor.AnchorProvider(connection, wallet, {});
    anchor.setProvider(provider);

    const idl = JSON.parse(fs.readFileSync("../target/idl/stake_weather.json").toString());
    const program = new Program<StakeWeather>(idl, provider);

    const programId = new PublicKey("Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5");

    const ORACLE_ACCOUNTS: Record<number, string> = {
        0: "2X3Qp3wDjFVV9mjtvBgCavGruBA84fQo9v99CXTEdUH2",
        1: "9k5hPcG3hvjz9TneBgzuN89yWXywvx2ZYgkGWhQdxHRM",
        2: "B2PP4x15qQstEFMR9S6nMci6vFZnkKgh7Hf5qzaRMM5G",
    };

    const CITY_NAMES: Record<number, string> = { 0: "Mumbai", 1: "Delhi", 2: "Bangalore" };

    const [betPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("bet"), creator.publicKey.toBuffer()],
        programId
    );
    const [vaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), creator.publicKey.toBuffer()],
        programId
    );

    const bet = await program.account.bet.fetch(betPda);
    const city = bet.city as number;
    const oracleQuote = new PublicKey(ORACLE_ACCOUNTS[city]);

    const oracleInfo = await connection.getAccountInfo(oracleQuote);
    console.log(`City: ${CITY_NAMES[city]}`);
    console.log(`Oracle account: ${oracleQuote.toBase58()}`);
    console.log(`Oracle owner: ${oracleInfo?.owner.toBase58()}`);

    const tx = await program.methods
        .settleBet()
        .accountsPartial({
            bet: betPda,
            vault: vaultPda,
            creator: creator.publicKey,
            challenger: challenger.publicKey,
            oracle: oracleQuote,
            caller: creator.publicKey,
        })
        .rpc({ commitment: "confirmed" });

    console.log(`settle_bet tx: ${tx}`);

    const txDetails = await program.provider.connection.getTransaction(tx, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
    });
    const logs = txDetails?.meta?.logMessages || [];
    logs.filter(l => l.includes("Program log:")).forEach(l => console.log(l.replace("Program log: ", "")));
}

main();

import * as sb from "@switchboard-xyz/on-demand";
import { CrossbarClient } from "@switchboard-xyz/common";
import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
import { StakeWeather } from "../target/types/stake_weather";
import fs from "fs";
import dotenv from "dotenv";
dotenv.config();

const FEED_HASHES: Record<number, string> = {
    0: "0x13716abd2e719f652c0f4a037acff7c945d62bc96ebb6b4224e21928d88b69b0",
    1: "0x8d63297658eabedc0e9137800ffa979b1703f7a9b6300b4cf788b120b26e7c79",
    2: "0xadbfc5c82de48a0c4c3feb97ae9c8175c3fae5a894b2316161f6242ce80fe874",
};

const ORACLE_ACCOUNTS: Record<number, string> = {
    0: "2X3Qp3wDjFVV9mjtvBgCavGruBA84fQo9v99CXTEdUH2",
    1: "9k5hPcG3hvjz9TneBgzuN89yWXywvx2ZYgkGWhQdxHRM",
    2: "B2PP4x15qQstEFMR9S6nMci6vFZnkKgh7Hf5qzaRMM5G",
};

const CITY_NAMES: Record<number, string> = { 0: "Mumbai", 1: "Delhi", 2: "Bangalore" };

function parseTemperature(data: Buffer, feedHashHex: string): number | null {
    if (data.length < 42) return null;
    const dataLen = data.readUInt16LE(40);
    if (data.length < 42 + dataLen) return null;
    const quoteBytes = data.slice(42, 42 + dataLen);
    if (quoteBytes.length < 46) return null;
    const feedsBytes = quoteBytes.slice(46);
    const FEED_SIZE = 49;
    const feedHashBytes = Buffer.from(feedHashHex.replace("0x", ""), "hex");
    for (let i = 0; i < Math.floor(feedsBytes.length / FEED_SIZE); i++) {
        const offset = i * FEED_SIZE;
        const feedId = feedsBytes.slice(offset, offset + 32);
        if (feedId.equals(feedHashBytes)) {
            const lo = feedsBytes.readBigUInt64LE(offset + 32);
            const hi = feedsBytes.readBigInt64LE(offset + 40);
            const raw = (hi << 64n) | lo;
            return Number(raw / 100_000_000_000_000_000n);
        }
    }
    return null;
}

async function main() {
    const rpcUrl = process.env.RPC_URL || "https://api.devnet.solana.com";
    const connection = new Connection(rpcUrl, "confirmed");

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

    const bet = await program.account.bet.fetch(betPda);
    const city = bet.city as number;
    const feedHash = FEED_HASHES[city];
    const oraclePubkey = new PublicKey(ORACLE_ACCOUNTS[city]);
    console.log(`City: ${CITY_NAMES[city]} | Feed hash: ${feedHash}`);

    const queue = await sb.getDefaultDevnetQueue(rpcUrl);
    const crossbar = new CrossbarClient("https://crossbar.switchboard.xyz");
    const updateIxs = await queue.fetchManagedUpdateIxs(crossbar, [feedHash], {
        payer: keypair.publicKey,
    });

    const tx = await sb.asV0Tx({
        connection,
        ixs: updateIxs,
        signers: [keypair],
        computeUnitPrice: 200_000,
        computeUnitLimitMultiple: 1.3,
    });

    const sig = await connection.sendTransaction(tx, { skipPreflight: true });
    console.log("Feed updated tx:", sig);

    await connection.confirmTransaction(sig, "confirmed");
    const oracleInfo = await connection.getAccountInfo(oraclePubkey);
    if (oracleInfo) {
        const temp = parseTemperature(Buffer.from(oracleInfo.data), feedHash);
        if (temp !== null) {
            console.log(`Temperature (from Solana oracle account ${oraclePubkey.toBase58()}): ${Math.floor(temp / 10)}.${Math.abs(temp % 10)}°C`);
            console.log(`Oracle account owner: ${oracleInfo.owner.toBase58()} (Switchboard program)`);
        }
    }
}

main();

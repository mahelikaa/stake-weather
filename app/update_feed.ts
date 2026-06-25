import * as sb from "@switchboard-xyz/on-demand";
import { CrossbarClient } from "@switchboard-xyz/common";
import * as anchor from "@coral-xyz/anchor";
import { Connection } from "@solana/web3.js";
import fs from "fs";
import dotenv from "dotenv";
dotenv.config();

const MUMBAI_FEED_HASH = "0x13716abd2e719f652c0f4a037acff7c945d62bc96ebb6b4224e21928d88b69b0";

async function main() {
    const rpcUrl = process.env.RPC_URL || "https://api.devnet.solana.com";
    const connection = new Connection(rpcUrl);

    const keypairFile = fs.readFileSync(process.env.CREATOR_KEYPAIR!);
    const keypair = anchor.web3.Keypair.fromSecretKey(
        Buffer.from(JSON.parse(keypairFile.toString()))
    );

    const queue = await sb.getDefaultDevnetQueue(rpcUrl);
    const crossbar = new CrossbarClient("https://crossbar.switchboard.xyz");

    const updateIxs = await queue.fetchManagedUpdateIxs(crossbar, [MUMBAI_FEED_HASH], {
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
}

main();
import * as sb from "@switchboard-xyz/on-demand";
import { OracleJob } from "@switchboard-xyz/common";
import * as anchor from "@coral-xyz/anchor";
import fs from "fs";

const CITIES = [
    { name: "mumbai-temperature", lat: 19.0760, lon: 72.8777 },
    { name: "delhi-temperature", lat: 28.6139, lon: 77.2090 },
    { name: "bangalore-temperature", lat: 12.9716, lon: 77.5946 },
];

async function main() {
    const connection = new anchor.web3.Connection(
        "https://devnet.helius-rpc.com/?api-key=630f3384-4d68-4e76-ba30-26304a39682a"
    );

    const keypairFile = fs.readFileSync("/Users/mahelikaa/.config/solana/id.json");
    const keypair = anchor.web3.Keypair.fromSecretKey(
        Buffer.from(JSON.parse(keypairFile.toString()))
    );

    const wallet = new anchor.Wallet(keypair);
    const sbProgram = await sb.AnchorUtils.loadProgramFromConnection(connection, wallet);
    const queue = await sb.Queue.loadDefault(sbProgram);

    for (const city of CITIES) {
        const jobs: OracleJob[] = [
            OracleJob.fromObject({
                tasks: [
                    { httpTask: { url: `https://api.open-meteo.com/v1/forecast?latitude=${city.lat}&longitude=${city.lon}&current_weather=true` } },
                    { jsonParseTask: { path: "$.current_weather.temperature" } },
                ],
            }),
        ];

        const [pullFeed, feedKeypair] = sb.PullFeed.generate(sbProgram);
        const initIx = await pullFeed.initIx({
            name: city.name,
            queue: queue.pubkey,
            maxVariance: 1.0,
            minResponses: 1,
            minSampleSize: 1,
            maxStaleness: 300,
            jobs: jobs,
            payer: keypair.publicKey,
        });

        const tx = await sb.asV0Tx({
            connection,
            ixs: [initIx],
            signers: [keypair, feedKeypair],
            computeUnitPrice: 20_000,
            computeUnitLimitMultiple: 1.3,
        });

        const sig = await connection.sendTransaction(tx);
        console.log(`${city.name} feed pubkey: ${pullFeed.pubkey.toBase58()}`);
        console.log(`tx: ${sig}`);
    }
}

main();
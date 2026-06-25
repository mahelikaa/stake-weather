# StakeWeather

A Solana program where two players lock SOL and bet on whether a city's temperature will be above or below a threshold by a deadline. Temperature data is fetched from Open-Meteo via Switchboard On-Demand oracles.

Built to learn Switchboard custom oracle feeds on Solana devnet.

## How it works

1. Creator creates a bet — picks a city, a temperature threshold, a direction (above/below), a deadline, and locks SOL
2. Challenger joins the bet — takes the opposite side and locks the same amount of SOL
3. Anyone calls `update_feed` after the deadline to fetch the latest temperature from Switchboard
4. Anyone calls `settle_bet` — the program reads the oracle, determines the winner, and pays out the vault

## Cities supported

- Mumbai (0)
- Delhi (1)
- Bangalore (2)

## Setup

```bash
git clone <your-repo>
cd stake-weather
npm install
cd app
npm install
cp .env.example .env
```

Fill in `.env`:

```
RPC_URL=https://devnet.helius-rpc.com/?api-key=YOUR_HELIUS_API_KEY
CREATOR_KEYPAIR=/path/to/creator-keypair.json
CHALLENGER_KEYPAIR=/path/to/challenger-keypair.json
```

## Running

```bash
cd app

# 1. Creator creates a bet
npx ts-node create_bet.ts

# 2. Challenger joins
npx ts-node join_bet.ts

# 3. Wait for the deadline, then update the feed
npx ts-node update_feed.ts

# 4. Settle the bet
npx ts-node settle_bet.ts

# Cancel a bet before anyone joins (creator only)
npx ts-node cancel_bet.ts
```

## Program

Deployed on Solana devnet: `Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5`

## Tech

- [Anchor](https://www.anchor-lang.com/) 0.32.1
- [Switchboard On-Demand](https://docs.switchboard.xyz/) 0.13.0
- [Open-Meteo](https://open-meteo.com/) for weather data
- Helius RPC for devnet

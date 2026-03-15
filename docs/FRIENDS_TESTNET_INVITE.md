# Friends Testnet Invitation Templates

Message templates for inviting friends to test the Aztibase testnet.
Copy, personalize, and send via WhatsApp/Telegram/Discord/email.

---

## Template 1: Quick Pitch (WhatsApp/Telegram)

```
Hey [name]! I've been building a blockchain from scratch in Rust —
Aztibase Network. It's live on testnet now with 3 validators running.

Want to help me break it? You can:
- Run a node on your PC (takes 2 mins to set up)
- Use the Chrome wallet extension to send test tokens
- Just hit the explorer and poke around

No crypto knowledge needed, just curiosity.
Here's the setup guide: [link to VALIDATOR_ONBOARDING.md or GitHub release]

Explorer: https://explorer.aztibase.com
```

## Template 2: Technical Friend (Developer)

```
Hey [name] — I'm looking for a few people to stress-test my L1 chain.

Aztibase is a DAG-based BFT blockchain written in Rust. I've got
a 3-node validator testnet running with ~15 batches/sec throughput,
sub-second finality, and a block sync catch-up protocol for full nodes.

What I need:
- Someone to run a full node and see if it catches up cleanly
- Hit the RPC with some traffic (JSON-RPC over HTTP)
- Try breaking consensus by restarting nodes at bad times

It's a single binary — download, run `bash start.sh`, done.

GitHub release (Windows): https://github.com/aztibase/aztibase-node/releases/tag/v0.1.2
Explorer: https://explorer.aztibase.com
RPC docs: 48 methods, all documented
```

## Template 3: Non-Technical Friend

```
Hey [name]! Remember that blockchain project I mentioned?
It's actually running now — you can see it live.

Check out the explorer: https://explorer.aztibase.com

If you want to try the wallet:
1. Download this Chrome extension: [wallet zip link]
2. Unzip it, go to chrome://extensions, enable Developer mode
3. Click "Load unpacked" and select the folder
4. You'll get a wallet address — I'll send you some test tokens

It's all testnet (fake money), so nothing to lose. Just want
to see if the UX makes sense to a normal person.
```

## Template 4: Group Chat Drop

```
Side project update: my blockchain is live on testnet.

3 validators running, DAG consensus, sub-second finality.
Built it from scratch in Rust — no forks, no frameworks.

Explorer: https://explorer.aztibase.com
GitHub: https://github.com/aztibase/aztibase-node

If anyone wants to run a node or test the wallet, DM me.
Takes about 2 minutes to set up on Windows.
```

## Template 5: Email (Formal)

```
Subject: Aztibase Testnet — Looking for Testers

Hi [name],

I've been building an L1 blockchain called Aztibase Network.
It's now live on testnet and I'm looking for a few people to
help validate the node software before mainnet.

What the testnet needs:
- Full node operators (download binary, run, see if it syncs)
- Wallet testers (Chrome extension, send/receive test tokens)
- RPC users (developers who want to hit the JSON-RPC API)

Everything is pre-packaged — a single binary with auto-keygen.
Setup takes under 5 minutes. Detailed guide attached.

Links:
- Explorer: https://explorer.aztibase.com
- GitHub Release: https://github.com/aztibase/aztibase-node/releases/tag/v0.1.2
- Onboarding Guide: [attach VALIDATOR_ONBOARDING.md]

Let me know if you're interested — happy to walk you through it.

Best,
[your name]
```

---

## Coordinator Script Walkthrough

Step-by-step for onboarding a friend onto the testnet.

### Before they start

1. Send them the appropriate template above
2. Make sure the testnet is running: `bash start-testnet-public.sh`
3. Verify public RPC is reachable:
   ```bash
   curl -s https://rpc.aztibase.com -X POST \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"aztb_blockHeight","params":[],"id":1}'
   ```

### Walking them through setup

**Option A — Just the explorer (zero setup)**
1. Send them https://explorer.aztibase.com
2. Point out: block height increasing, validator list, recent batches
3. Done — they can see the chain is live

**Option B — Chrome wallet (2 minutes)**
1. Send them the wallet zip from the GitHub release
2. Walk them through: unzip → chrome://extensions → Developer mode → Load unpacked
3. They click the extension, get a wallet address
4. You send them tokens:
   ```bash
   curl -s https://rpc.aztibase.com -X POST \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"aztb_sendTransaction","params":[{
       "from": "0xYOUR_VALIDATOR_ADDRESS",
       "to": "0xTHEIR_ADDRESS",
       "value": "1000000",
       "gas": "21000"
     }],"id":1}'
   ```
5. They check their balance in the wallet — should show up within seconds

**Option C — Full node (5 minutes)**
1. Send them the fullnode release: `aztibase-fullnode-v0.1.2-windows-x64.tar.gz`
2. They extract and run `bash start.sh`
3. Node connects to boot nodes and starts catching up
4. Monitor their progress: `curl http://THEIR_IP:9947/health`
5. Once synced, their node follows live batches via gossip

**Option D — Validator node (5 minutes + staking)**
1. Send them the validator release: `aztibase-validator-v0.1.2-windows-x64.tar.gz`
2. They extract and run `bash start.sh` (auto-generates keys)
3. Node syncs as full node first
4. You send them enough tokens for staking (50M AZTB minimum)
5. They stake via the dashboard or CLI
6. At next epoch boundary, they join the active validator set

### Troubleshooting

| Issue | Fix |
|-------|-----|
| Node won't start | Check `node.log` for errors. Common: port already in use |
| Can't connect to boot nodes | Firewall blocking port 30336. Try: `netstat -an \| findstr 30336` |
| Wallet not loading | Must use Chrome. Firefox doesn't support MV3 extensions the same way |
| Balance shows 0 | Wait 5-10 seconds after sending tokens, then refresh |
| Node stuck syncing | Check if testnet validators are running: `curl https://rpc.aztibase.com/health` |

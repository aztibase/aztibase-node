"""
Aztibase Guard — Telegram bot for chain monitoring and queries.
Connects to an Aztibase node via JSON-RPC and WebSocket.
"""

import asyncio
import json
import logging
import os
import time

import aiohttp
from telegram import Update
from telegram.ext import (
    Application,
    CommandHandler,
    ContextTypes,
    MessageHandler,
    filters,
)

logging.basicConfig(
    format="%(asctime)s [%(levelname)s] %(message)s", level=logging.INFO
)
log = logging.getLogger("guard")

BOT_TOKEN = os.environ.get(
    "GUARD_BOT_TOKEN", "8766344587:AAEgcSi2HveW_wJSCtXYkz8u3CyrfyUVdGw"
)
RPC_URL = os.environ.get("GUARD_RPC_URL", "http://102.209.21.247:9944")
WS_URL = os.environ.get("GUARD_WS_URL", "ws://102.209.21.247:9944/ws")
OWNER_USERNAME = "Wantdadolla"
OWNER_ID = int(os.environ.get("GUARD_OWNER_ID", "0"))

# ── RPC helpers ───────────────────────────────────────────────────────

_rpc_id = 0


async def rpc_call(method: str, params=None) -> dict:
    global _rpc_id
    _rpc_id += 1
    payload = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params or [],
        "id": _rpc_id,
    }
    async with aiohttp.ClientSession() as session:
        async with session.post(
            RPC_URL, json=payload, timeout=aiohttp.ClientTimeout(total=10)
        ) as resp:
            data = await resp.json()
            if "error" in data and data["error"]:
                raise RuntimeError(data["error"].get("message", "RPC error"))
            return data.get("result")


def fmt_hex_balance(hex_str: str) -> str:
    if hex_str and hex_str.startswith("0x"):
        return f"{int(hex_str, 16):,}"
    return str(hex_str)


def short_hash(h: str) -> str:
    if h and len(h) > 12:
        return h[:10] + "..."
    return str(h)


# ── Authorization ─────────────────────────────────────────────────────


def is_authorized(update: Update) -> bool:
    user = update.effective_user
    if not user:
        return False
    if OWNER_ID and user.id == OWNER_ID:
        return True
    if user.username and user.username.lower() == OWNER_USERNAME.lower():
        return True
    return False


async def check_auth(update: Update) -> bool:
    if is_authorized(update):
        return True
    await update.message.reply_text("Unauthorized.")
    return False


# ── Commands ──────────────────────────────────────────────────────────


async def cmd_start(update: Update, context: ContextTypes.DEFAULT_TYPE):
    user = update.effective_user
    uid = user.id if user else "?"
    uname = user.username if user else "?"
    log.info(f"/start from user_id={uid} username={uname}")
    await update.message.reply_text(
        f"Aztibase Guard online.\n"
        f"Your user ID: {uid}\n"
        f"Your username: @{uname}\n\n"
        f"Commands:\n"
        f"/status — chain overview\n"
        f"/health — sentinel health\n"
        f"/validators — active validators\n"
        f"/block [n] — block info\n"
        f"/balance <addr> — account balance\n"
        f"/epochs [n] — recent epoch summaries\n"
        f"/metrics — prometheus metrics snapshot\n"
        f"/help — this message"
    )


async def cmd_status(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        info, height, gas, health = await asyncio.gather(
            rpc_call("aztb_nodeInfo"),
            rpc_call("aztb_blockNumber"),
            rpc_call("aztb_gasPrice"),
            rpc_call("aztb_getChainHealth"),
        )
        block = int(height, 16) if isinstance(height, str) else height
        fee = fmt_hex_balance(gas) if isinstance(gas, str) else gas

        health_line = ""
        if health and isinstance(health, dict) and "score" in health:
            score = health["score"]
            level = health.get("level", "?")
            health_line = f"Health: {score:.3f} ({level})\n"

        version = info.get("version", "?") if info else "?"

        msg = (
            f"Aztibase Network\n"
            f"Version: {version}\n"
            f"Block: {block:,}\n"
            f"Base fee: {fee}\n"
            f"{health_line}"
        )
        await update.message.reply_text(msg)
    except Exception as e:
        await update.message.reply_text(f"RPC error: {e}")


async def cmd_health(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        health = await rpc_call("aztb_getChainHealth")
        if not health or not isinstance(health, dict):
            await update.message.reply_text("Sentinel not active.")
            return

        score = health.get("score", 0)
        level = health.get("level", "?")
        batch = health.get("batch_height", "?")
        features = health.get("features", [])
        names = health.get("feature_names", [])

        lines = [f"Sentinel Health: {score:.3f} ({level})", f"Batch: {batch}", ""]
        for name, val in zip(names[:8], features[:8]):
            lines.append(f"  {name}: {val:.2f}")
        if len(names) > 8:
            lines.append(f"  ... +{len(names) - 8} more features")

        actions = await rpc_call("aztb_getSentinelActions", [3])
        if actions:
            lines.append("\nRecent actions:")
            for a in actions[:3]:
                kind = a.get("kind", {})
                ktype = kind.get("type", "?")
                dry = " [DRY]" if a.get("dry_run") else ""
                lines.append(f"  {ktype}{dry} (score={a.get('score', '?'):.3f})")

        await update.message.reply_text("\n".join(lines))
    except Exception as e:
        await update.message.reply_text(f"RPC error: {e}")


async def cmd_validators(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        validators = await rpc_call("aztb_getActiveValidators")
        if not validators:
            await update.message.reply_text("No active validators.")
            return

        lines = [f"Active validators: {len(validators)}", ""]
        for v in validators[:20]:
            vid = v.get("validator_id", "?")
            stake = v.get("effective_stake", v.get("self_stake", "?"))
            if isinstance(vid, str) and len(vid) > 12:
                vid = vid[:10] + "..."
            lines.append(f"  {vid} — stake: {stake:,}" if isinstance(stake, int) else f"  {vid} — stake: {stake}")

        await update.message.reply_text("\n".join(lines))
    except Exception as e:
        await update.message.reply_text(f"RPC error: {e}")


async def cmd_block(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        args = context.args
        if args:
            num = int(args[0])
        else:
            height = await rpc_call("aztb_blockNumber")
            num = int(height, 16) if isinstance(height, str) else height

        block = await rpc_call("aztb_getBlockByNumber", [num])
        if not block:
            await update.message.reply_text(f"Block {num} not found.")
            return

        txs = block.get("transactions", [])
        bhash = short_hash(block.get("hash", "?"))
        state_root = short_hash(block.get("stateRoot", "?"))

        msg = (
            f"Block #{num:,}\n"
            f"Hash: {bhash}\n"
            f"State root: {state_root}\n"
            f"Transactions: {len(txs)}"
        )
        await update.message.reply_text(msg)
    except ValueError:
        await update.message.reply_text("Usage: /block [number]")
    except Exception as e:
        await update.message.reply_text(f"RPC error: {e}")


async def cmd_balance(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        args = context.args
        if not args:
            await update.message.reply_text("Usage: /balance <0x address>")
            return
        addr = args[0]
        if not addr.startswith("0x"):
            addr = "0x" + addr

        balance = await rpc_call("aztb_getBalance", [addr])
        nonce = await rpc_call("aztb_getNonce", [addr])
        acct_type = await rpc_call("aztb_getAccountType", [addr])

        bal_str = fmt_hex_balance(balance) if isinstance(balance, str) else str(balance)

        msg = (
            f"Account: {short_hash(addr)}\n"
            f"Type: {acct_type}\n"
            f"Balance: {bal_str} AZTB\n"
            f"Nonce: {nonce}"
        )
        await update.message.reply_text(msg)
    except Exception as e:
        await update.message.reply_text(f"RPC error: {e}")


async def cmd_epochs(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        args = context.args
        limit = int(args[0]) if args else 5
        limit = min(limit, 20)

        summaries = await rpc_call("aztb_getEpochSummaries", [limit])
        if not summaries:
            await update.message.reply_text("No epoch summaries yet.")
            return

        lines = [f"Recent epoch summaries ({len(summaries)}):"]
        for s in summaries:
            epoch = s.get("epoch", "?")
            profiles = s.get("validator_profiles", [])
            avg_health = s.get("avg_health_score", 0)
            lines.append(
                f"  Epoch {epoch}: {len(profiles)} validators, "
                f"health={avg_health:.3f}"
            )

        await update.message.reply_text("\n".join(lines))
    except Exception as e:
        await update.message.reply_text(f"RPC error: {e}")


async def cmd_metrics(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(
                f"{RPC_URL}/metrics/json",
                timeout=aiohttp.ClientTimeout(total=10),
            ) as resp:
                data = await resp.json()

        lines = []
        for category, values in data.items():
            lines.append(f"{category}:")
            if isinstance(values, dict):
                for k, v in values.items():
                    lines.append(f"  {k}: {v}")
            else:
                lines.append(f"  {values}")

        msg = "\n".join(lines)
        if len(msg) > 4000:
            msg = msg[:4000] + "\n..."
        await update.message.reply_text(msg)
    except Exception as e:
        await update.message.reply_text(f"Metrics error: {e}")


async def cmd_help(update: Update, context: ContextTypes.DEFAULT_TYPE):
    await cmd_start(update, context)


async def handle_text(update: Update, context: ContextTypes.DEFAULT_TYPE):
    if not await check_auth(update):
        return
    text = update.message.text.strip().lower()

    if any(w in text for w in ["status", "how", "chain", "what's up"]):
        await cmd_status(update, context)
    elif any(w in text for w in ["health", "sentinel", "safe", "score"]):
        await cmd_health(update, context)
    elif any(w in text for w in ["validator", "who"]):
        await cmd_validators(update, context)
    elif any(w in text for w in ["block", "height", "latest"]):
        await cmd_block(update, context)
    elif any(w in text for w in ["epoch", "profile", "summary"]):
        await cmd_epochs(update, context)
    elif any(w in text for w in ["metric", "stats"]):
        await cmd_metrics(update, context)
    else:
        await update.message.reply_text(
            "I didn't understand that. Try /help for commands, "
            "or ask about: status, health, validators, blocks, epochs, metrics."
        )


# ── WebSocket alert listener ─────────────────────────────────────────


async def ws_alert_loop(app: Application):
    """Subscribe to chainHealth WebSocket topic and push alerts to owner."""
    await asyncio.sleep(5)
    bot = app.bot

    owner_id = OWNER_ID
    if not owner_id:
        log.warning(
            "GUARD_OWNER_ID not set — alerts disabled. "
            "Send /start to the bot and set your user ID."
        )
        return

    last_level = "normal"
    retry_delay = 5

    while True:
        try:
            async with aiohttp.ClientSession() as session:
                async with session.ws_connect(WS_URL) as ws:
                    sub_msg = json.dumps(
                        {
                            "jsonrpc": "2.0",
                            "method": "aztb_subscribe",
                            "params": ["chainHealth"],
                            "id": 1,
                        }
                    )
                    await ws.send_str(sub_msg)
                    log.info("WebSocket connected — subscribed to chainHealth")
                    retry_delay = 5

                    async for msg in ws:
                        if msg.type == aiohttp.WSMsgType.TEXT:
                            data = json.loads(msg.data)
                            result = (
                                data.get("params", {}).get("result")
                                or data.get("result")
                            )
                            if not result or not isinstance(result, dict):
                                continue

                            level = result.get("level", "normal")
                            score = result.get("score", 0)

                            if level != last_level and level in (
                                "warning",
                                "critical",
                            ):
                                emoji = "!!" if level == "critical" else "!"
                                alert = (
                                    f"{emoji} Chain health: {level.upper()}\n"
                                    f"Score: {score:.3f}\n"
                                    f"Batch: {result.get('batch_height', '?')}"
                                )
                                try:
                                    await bot.send_message(
                                        chat_id=owner_id, text=alert
                                    )
                                except Exception as e:
                                    log.error(f"Failed to send alert: {e}")

                            if level == "normal" and last_level != "normal":
                                try:
                                    await bot.send_message(
                                        chat_id=owner_id,
                                        text=f"Chain health restored to NORMAL (score={score:.3f})",
                                    )
                                except Exception as e:
                                    log.error(f"Failed to send recovery: {e}")

                            last_level = level

                        elif msg.type in (
                            aiohttp.WSMsgType.CLOSED,
                            aiohttp.WSMsgType.ERROR,
                        ):
                            break

        except Exception as e:
            log.warning(f"WebSocket error: {e}, reconnecting in {retry_delay}s")

        await asyncio.sleep(retry_delay)
        retry_delay = min(retry_delay * 2, 60)


# ── Main ──────────────────────────────────────────────────────────────


def main():
    async def post_init(application: Application):
        asyncio.create_task(ws_alert_loop(application))

    app = (
        Application.builder()
        .token(BOT_TOKEN)
        .post_init(post_init)
        .build()
    )

    app.add_handler(CommandHandler("start", cmd_start))
    app.add_handler(CommandHandler("status", cmd_status))
    app.add_handler(CommandHandler("health", cmd_health))
    app.add_handler(CommandHandler("validators", cmd_validators))
    app.add_handler(CommandHandler("block", cmd_block))
    app.add_handler(CommandHandler("balance", cmd_balance))
    app.add_handler(CommandHandler("epochs", cmd_epochs))
    app.add_handler(CommandHandler("metrics", cmd_metrics))
    app.add_handler(CommandHandler("help", cmd_help))
    app.add_handler(MessageHandler(filters.TEXT & ~filters.COMMAND, handle_text))

    log.info(f"Aztibase Guard bot starting — RPC: {RPC_URL}")
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()

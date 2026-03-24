"""
Aztibase Guard Game — backend API for provably fair dice & coin flip.
Serves the Mini App frontend and handles bets via REST API.
Balances are in-memory (testnet play money from faucet).
"""

import hashlib
import hmac
import json
import os
import secrets
import time
from pathlib import Path

from aiohttp import web

HOST = os.environ.get("GAME_HOST", "0.0.0.0")
PORT = int(os.environ.get("GAME_PORT", "8088"))
BOT_TOKEN = os.environ.get(
    "GUARD_BOT_TOKEN", "8766344587:AAEgcSi2HveW_wJSCtXYkz8u3CyrfyUVdGw"
)
FAUCET_AMOUNT = 10_000
FAUCET_COOLDOWN = 60
HOUSE_EDGE = 0.02
STATIC_DIR = Path(__file__).parent / "static"

# ── In-memory state ───────────────────────────────────────────────────

balances: dict[int, int] = {}
faucet_last: dict[int, float] = {}
bet_history: dict[int, list] = {}
server_seeds: dict[int, str] = {}
nonces: dict[int, int] = {}


def get_balance(user_id: int) -> int:
    return balances.get(user_id, 0)


def credit(user_id: int, amount: int):
    balances[user_id] = get_balance(user_id) + amount


def debit(user_id: int, amount: int) -> bool:
    bal = get_balance(user_id)
    if bal < amount:
        return False
    balances[user_id] = bal - amount
    return True


# ── Provably fair ─────────────────────────────────────────────────────


def generate_server_seed(user_id: int) -> str:
    seed = secrets.token_hex(32)
    server_seeds[user_id] = seed
    nonces[user_id] = 0
    return hashlib.sha256(seed.encode()).hexdigest()


def get_seed_hash(user_id: int) -> str:
    seed = server_seeds.get(user_id)
    if not seed:
        return generate_server_seed(user_id)
    return hashlib.sha256(seed.encode()).hexdigest()


def roll_result(user_id: int, client_seed: str) -> tuple[float, dict]:
    """Generate a provably fair result in [0, 1). Returns (result, proof)."""
    s_seed = server_seeds.get(user_id)
    if not s_seed:
        generate_server_seed(user_id)
        s_seed = server_seeds[user_id]

    nonce = nonces.get(user_id, 0)
    nonces[user_id] = nonce + 1

    combined = f"{s_seed}:{client_seed}:{nonce}"
    h = hmac.new(s_seed.encode(), combined.encode(), hashlib.sha256).hexdigest()
    result = int(h[:8], 16) / 0xFFFFFFFF

    proof = {
        "server_seed_hash": hashlib.sha256(s_seed.encode()).hexdigest(),
        "client_seed": client_seed,
        "nonce": nonce,
        "hmac": h[:16],
    }
    return result, proof


# ── Telegram auth validation ──────────────────────────────────────────


def validate_init_data(init_data: str) -> dict | None:
    """Validate Telegram Mini App init data. Returns user dict or None."""
    if not init_data:
        return None

    params = dict(p.split("=", 1) for p in init_data.split("&") if "=" in p)
    check_hash = params.pop("hash", None)
    if not check_hash:
        return None

    data_check = "\n".join(f"{k}={v}" for k, v in sorted(params.items()))
    secret = hmac.new(b"WebAppData", BOT_TOKEN.encode(), hashlib.sha256).digest()
    computed = hmac.new(secret, data_check.encode(), hashlib.sha256).hexdigest()

    if not hmac.compare_digest(computed, check_hash):
        return None

    user_str = params.get("user")
    if not user_str:
        return None

    try:
        import urllib.parse
        return json.loads(urllib.parse.unquote(user_str))
    except (json.JSONDecodeError, Exception):
        return None


def get_user_id(request: web.Request) -> int | None:
    """Extract user ID from Telegram init data or fallback header."""
    init_data = request.headers.get("X-Telegram-Init-Data", "")
    user = validate_init_data(init_data)
    if user:
        return user.get("id")

    # Fallback for testing without Telegram
    test_id = request.headers.get("X-Test-User-Id")
    if test_id and test_id.isdigit():
        return int(test_id)

    return None


# ── API routes ────────────────────────────────────────────────────────


async def api_status(request: web.Request):
    uid = get_user_id(request)
    if not uid:
        return web.json_response({"error": "unauthorized"}, status=401)

    if uid not in server_seeds:
        generate_server_seed(uid)

    return web.json_response({
        "balance": get_balance(uid),
        "seed_hash": get_seed_hash(uid),
        "history": (bet_history.get(uid) or [])[-10:],
    })


async def api_faucet(request: web.Request):
    uid = get_user_id(request)
    if not uid:
        return web.json_response({"error": "unauthorized"}, status=401)

    now = time.time()
    last = faucet_last.get(uid, 0)
    if now - last < FAUCET_COOLDOWN:
        remaining = int(FAUCET_COOLDOWN - (now - last))
        return web.json_response({
            "error": f"Wait {remaining}s for next faucet drip",
        }, status=429)

    credit(uid, FAUCET_AMOUNT)
    faucet_last[uid] = now

    if uid not in server_seeds:
        generate_server_seed(uid)

    return web.json_response({
        "balance": get_balance(uid),
        "amount": FAUCET_AMOUNT,
    })


async def api_flip(request: web.Request):
    uid = get_user_id(request)
    if not uid:
        return web.json_response({"error": "unauthorized"}, status=401)

    try:
        body = await request.json()
    except Exception:
        return web.json_response({"error": "invalid json"}, status=400)

    amount = body.get("amount", 0)
    side = body.get("side", "").lower()
    client_seed = body.get("client_seed", secrets.token_hex(8))

    if side not in ("heads", "tails"):
        return web.json_response({"error": "side must be heads or tails"}, status=400)
    if not isinstance(amount, int) or amount < 1:
        return web.json_response({"error": "amount must be >= 1"}, status=400)
    if amount > 100_000:
        return web.json_response({"error": "max bet: 100,000"}, status=400)
    if not debit(uid, amount):
        return web.json_response({"error": "insufficient balance"}, status=400)

    result, proof = roll_result(uid, client_seed)
    coin = "heads" if result < 0.5 else "tails"
    won = coin == side
    payout = int(amount * (2 - HOUSE_EDGE)) if won else 0

    if won:
        credit(uid, payout)

    record = {
        "game": "flip",
        "bet": amount,
        "side": side,
        "result": coin,
        "won": won,
        "payout": payout,
        "proof": proof,
        "time": int(time.time()),
    }
    bet_history.setdefault(uid, []).append(record)
    if len(bet_history[uid]) > 50:
        bet_history[uid] = bet_history[uid][-50:]

    return web.json_response({
        "result": coin,
        "won": won,
        "payout": payout,
        "balance": get_balance(uid),
        "proof": proof,
    })


async def api_dice(request: web.Request):
    uid = get_user_id(request)
    if not uid:
        return web.json_response({"error": "unauthorized"}, status=401)

    try:
        body = await request.json()
    except Exception:
        return web.json_response({"error": "invalid json"}, status=400)

    amount = body.get("amount", 0)
    target = body.get("target", 50)
    direction = body.get("direction", "over").lower()
    client_seed = body.get("client_seed", secrets.token_hex(8))

    if direction not in ("over", "under"):
        return web.json_response({"error": "direction must be over or under"}, status=400)
    if not isinstance(target, int) or target < 2 or target > 98:
        return web.json_response({"error": "target must be 2-98"}, status=400)
    if not isinstance(amount, int) or amount < 1:
        return web.json_response({"error": "amount must be >= 1"}, status=400)
    if amount > 100_000:
        return web.json_response({"error": "max bet: 100,000"}, status=400)
    if not debit(uid, amount):
        return web.json_response({"error": "insufficient balance"}, status=400)

    result, proof = roll_result(uid, client_seed)
    roll = int(result * 100) + 1  # 1-100

    if direction == "over":
        win_chance = (100 - target) / 100
        won = roll > target
    else:
        win_chance = target / 100
        won = roll < target

    multiplier = (1 - HOUSE_EDGE) / win_chance if win_chance > 0 else 0
    payout = int(amount * multiplier) if won else 0

    if won:
        credit(uid, payout)

    record = {
        "game": "dice",
        "bet": amount,
        "target": target,
        "direction": direction,
        "roll": roll,
        "won": won,
        "payout": payout,
        "multiplier": round(multiplier, 2),
        "proof": proof,
        "time": int(time.time()),
    }
    bet_history.setdefault(uid, []).append(record)
    if len(bet_history[uid]) > 50:
        bet_history[uid] = bet_history[uid][-50:]

    return web.json_response({
        "roll": roll,
        "won": won,
        "payout": payout,
        "multiplier": round(multiplier, 2),
        "balance": get_balance(uid),
        "proof": proof,
    })


async def api_history(request: web.Request):
    uid = get_user_id(request)
    if not uid:
        return web.json_response({"error": "unauthorized"}, status=401)

    return web.json_response({
        "history": (bet_history.get(uid) or [])[-20:],
    })


async def api_rotate_seed(request: web.Request):
    """Reveal current server seed and generate a new one."""
    uid = get_user_id(request)
    if not uid:
        return web.json_response({"error": "unauthorized"}, status=401)

    old_seed = server_seeds.get(uid, "")
    new_hash = generate_server_seed(uid)

    return web.json_response({
        "revealed_seed": old_seed,
        "new_seed_hash": new_hash,
    })


# ── App setup ─────────────────────────────────────────────────────────


def create_app() -> web.Application:
    app = web.Application()

    app.router.add_get("/api/status", api_status)
    app.router.add_post("/api/faucet", api_faucet)
    app.router.add_post("/api/flip", api_flip)
    app.router.add_post("/api/dice", api_dice)
    app.router.add_get("/api/history", api_history)
    app.router.add_post("/api/rotate-seed", api_rotate_seed)

    if STATIC_DIR.exists():
        app.router.add_static("/", STATIC_DIR, name="static", show_index=True)

    return app


if __name__ == "__main__":
    print(f"Aztibase Guard Game starting on {HOST}:{PORT}")
    web.run_app(create_app(), host=HOST, port=PORT)

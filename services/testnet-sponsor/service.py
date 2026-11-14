from __future__ import annotations

import base64
import fcntl
import hashlib
import json
import os
import re
import struct
import threading
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import requests
from cryptography.hazmat.primitives.serialization import (
    Encoding,
    PublicFormat,
    load_pem_private_key,
)
from flask import jsonify, request

_ACCOUNT_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]{1,63}$")
_B58_ALPHABET = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
_B58_INDEX = {char: index for index, char in enumerate(_B58_ALPHABET)}
_DEFAULT_TARGET = 2_000_000_000_000_000_000_000_000
_DEFAULT_RESERVE = 1_000_000_000_000_000_000_000_000


class SponsorError(RuntimeError):
    def __init__(self, message: str, status_code: int = 400, code: str = "sponsor_error"):
        super().__init__(message)
        self.status_code = status_code
        self.code = code


def _b58decode(value: str) -> bytes:
    raw = value.encode("ascii")
    number = 0
    for char in raw:
        if char not in _B58_INDEX:
            raise SponsorError("Invalid base58 value", 500, "sponsor_config_invalid")
        number = number * 58 + _B58_INDEX[char]
    decoded = b"" if number == 0 else number.to_bytes((number.bit_length() + 7) // 8, "big")
    pad = len(raw) - len(raw.lstrip(b"1"))
    return b"\x00" * pad + decoded


def _b58encode(value: bytes) -> str:
    number = int.from_bytes(value, "big")
    out = bytearray()
    while number:
        number, remainder = divmod(number, 58)
        out.append(_B58_ALPHABET[remainder])
    pad = len(value) - len(value.lstrip(b"\x00"))
    return (b"1" * pad + bytes(reversed(out))).decode("ascii")


def _borsh_string(value: str) -> bytes:
    encoded = value.encode("utf-8")
    return struct.pack("<I", len(encoded)) + encoded


def _u128(value: int) -> bytes:
    if value < 0 or value >= 1 << 128:
        raise SponsorError("u128 value out of range", 500, "sponsor_internal")
    return value.to_bytes(16, "little")


def _encode_transfer_transaction(
    signer_id: str,
    public_key: str,
    nonce: int,
    receiver_id: str,
    block_hash: str,
    amount_yocto: int,
) -> bytes:
    key_type, _, key_value = public_key.partition(":")
    if key_type != "ed25519" or not key_value:
        raise SponsorError("Sponsor must use an ed25519 key", 500, "sponsor_config_invalid")
    public_key_bytes = _b58decode(key_value)
    if len(public_key_bytes) != 32:
        raise SponsorError("Sponsor public key must be 32 bytes", 500, "sponsor_config_invalid")
    block_hash_bytes = _b58decode(block_hash)
    if len(block_hash_bytes) != 32:
        raise SponsorError("RPC returned an invalid block hash", 502, "rpc_invalid")

    return b"".join(
        (
            _borsh_string(signer_id),
            b"\x00",  # PublicKey::ED25519
            public_key_bytes,
            struct.pack("<Q", nonce),
            _borsh_string(receiver_id),
            block_hash_bytes,
            struct.pack("<I", 1),  # one Action
            b"\x03",  # Action::Transfer
            _u128(amount_yocto),
        )
    )


def _encode_signed_transaction(transaction: bytes, signature: bytes) -> bytes:
    if len(signature) != 64:
        raise SponsorError("Invalid ed25519 signature length", 500, "sponsor_internal")
    return transaction + b"\x00" + signature  # Signature::ED25519


@dataclass(frozen=True)
class SponsorConfig:
    enabled: bool
    network_id: str
    rpc_url: str
    sponsor_account_id: str
    sponsor_public_key: str
    private_key_path: str
    target_yocto: int
    min_reserve_yocto: int
    living_nft_id: str
    nuve_ft_id: str
    ledger_path: str
    allowed_origins: tuple[str, ...]

    @classmethod
    def from_env(cls) -> "SponsorConfig":
        origins = tuple(
            item.strip()
            for item in os.getenv(
                "NUVI_HACKATHON_SPONSOR_ALLOWED_ORIGINS",
                "https://nuvelor.gg,https://www.nuvelor.gg,https://app.nuvelor.gg",
            ).split(",")
            if item.strip()
        )
        return cls(
            enabled=os.getenv("NUVI_HACKATHON_SPONSOR_ENABLED", "0") == "1",
            network_id=os.getenv("NUVI_HACKATHON_SPONSOR_NETWORK", "testnet").strip(),
            rpc_url=os.getenv(
                "NUVI_HACKATHON_SPONSOR_RPC", "https://rpc.testnet.fastnear.com"
            ).strip(),
            sponsor_account_id=os.getenv(
                "NUVI_HACKATHON_SPONSOR_ACCOUNT_ID",
                "sponsor-hack26.nvlborc70517034832.testnet",
            ).strip(),
            sponsor_public_key=os.getenv("NUVI_HACKATHON_SPONSOR_PUBLIC_KEY", "").strip(),
            private_key_path=os.getenv(
                "NUVI_HACKATHON_SPONSOR_PRIVATE_KEY_PATH",
                "/run/secrets/nuvi-hackathon-sponsor-ed25519.pem",
            ).strip(),
            target_yocto=int(
                os.getenv("NUVI_HACKATHON_SPONSOR_TARGET_YOCTO", str(_DEFAULT_TARGET))
            ),
            min_reserve_yocto=int(
                os.getenv(
                    "NUVI_HACKATHON_SPONSOR_MIN_RESERVE_YOCTO",
                    str(_DEFAULT_RESERVE),
                )
            ),
            living_nft_id=os.getenv(
                "NUVI_HACKATHON_LIVING_NFT_ID",
                "nuvi-hack26.nvlborc70517034832.testnet",
            ).strip(),
            nuve_ft_id=os.getenv(
                "NUVI_HACKATHON_NUVE_FT_ID",
                "nuve-hack26.nvlborc70517034832.testnet",
            ).strip(),
            ledger_path=os.getenv(
                "NUVI_HACKATHON_SPONSOR_LEDGER",
                "/var/lib/nuvi-sponsor/claims.jsonl",
            ).strip(),
            allowed_origins=origins,
        )


class NearRpc:
    def __init__(self, url: str, timeout: float = 8.0):
        self.url = url
        self.timeout = timeout
        self._counter = 0

    def call(self, method: str, params: Any) -> Any:
        self._counter += 1
        response = requests.post(
            self.url,
            json={"jsonrpc": "2.0", "id": self._counter, "method": method, "params": params},
            timeout=self.timeout,
        )
        response.raise_for_status()
        payload = response.json()
        if payload.get("error"):
            error = payload["error"]
            raise SponsorError(
                str(error.get("data") or error.get("message") or error)[:800],
                502,
                "rpc_error",
            )
        return payload.get("result")

    def account_state(self, account_id: str) -> dict[str, Any]:
        result = self.call(
            "query",
            {
                "request_type": "view_account",
                "finality": "final",
                "account_id": account_id,
            },
        )
        if not isinstance(result, dict) or "amount" not in result:
            raise SponsorError("Unable to read account balance", 502, "rpc_invalid")
        return result

    def access_key(self, account_id: str, public_key: str) -> dict[str, Any]:
        result = self.call(
            "query",
            {
                "request_type": "view_access_key",
                "finality": "final",
                "account_id": account_id,
                "public_key": public_key,
            },
        )
        if not isinstance(result, dict):
            raise SponsorError("Unable to read sponsor access key", 502, "rpc_invalid")
        return result

    def view(self, contract_id: str, method_name: str, args: dict[str, Any]) -> Any:
        result = self.call(
            "query",
            {
                "request_type": "call_function",
                "finality": "final",
                "account_id": contract_id,
                "method_name": method_name,
                "args_base64": base64.b64encode(
                    json.dumps(args, separators=(",", ":")).encode("utf-8")
                ).decode("ascii"),
            },
        )
        raw = bytes(result.get("result") or [])
        return json.loads(raw.decode("utf-8"))

    def broadcast(self, signed_transaction: bytes) -> dict[str, Any]:
        result = self.call(
            "broadcast_tx_commit",
            [base64.b64encode(signed_transaction).decode("ascii")],
        )
        if not isinstance(result, dict):
            raise SponsorError("Invalid transaction response", 502, "rpc_invalid")
        return result


class SponsorLedger:
    def __init__(self, path: str):
        self.path = Path(path)

    def _open(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        return self.path.open("a+", encoding="utf-8")

    def find(self, account_id: str) -> dict[str, Any] | None:
        with self._open() as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            handle.seek(0)
            found = None
            for line in handle:
                try:
                    item = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if item.get("account_id") == account_id:
                    found = item
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            return found

    def append_if_absent(self, record: dict[str, Any]) -> bool:
        with self._open() as handle:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX)
            handle.seek(0)
            for line in handle:
                try:
                    item = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if item.get("account_id") == record["account_id"]:
                    fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
                    return False
            handle.seek(0, os.SEEK_END)
            handle.write(json.dumps(record, separators=(",", ":")) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)
            return True


class NuviTestnetSponsor:
    def __init__(self, config: SponsorConfig):
        self.config = config
        self.rpc = NearRpc(config.rpc_url)
        self.ledger = SponsorLedger(config.ledger_path)
        self._lock = threading.Lock()
        self._private_key = None
        self._validate_config()

    def _validate_config(self) -> None:
        if not self.config.enabled:
            return
        if self.config.network_id != "testnet":
            raise SponsorError("Sponsor is Testnet-only", 500, "sponsor_config_invalid")
        if not self.config.sponsor_account_id.endswith(".testnet"):
            raise SponsorError("Sponsor account must be Testnet", 500, "sponsor_config_invalid")
        if self.config.target_yocto <= 0 or self.config.min_reserve_yocto <= 0:
            raise SponsorError("Sponsor limits must be positive", 500, "sponsor_config_invalid")
        if not self.config.sponsor_public_key.startswith("ed25519:"):
            raise SponsorError("Sponsor public key is required", 500, "sponsor_config_invalid")

        key_path = Path(self.config.private_key_path)
        key = load_pem_private_key(key_path.read_bytes(), password=None)
        public_bytes = key.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
        derived = "ed25519:" + _b58encode(public_bytes)
        if derived != self.config.sponsor_public_key:
            raise SponsorError("Sponsor key mismatch", 500, "sponsor_config_invalid")
        self._private_key = key

    @staticmethod
    def _validate_account_id(account_id: str) -> str:
        value = str(account_id or "").strip().lower()
        if not _ACCOUNT_RE.fullmatch(value) or not value.endswith(".testnet"):
            raise SponsorError("A valid NEAR Testnet account is required", 400, "invalid_account")
        return value

    def status(self) -> dict[str, Any]:
        if not self.config.enabled:
            return {"enabled": False, "network_id": "testnet"}
        state = self.rpc.account_state(self.config.sponsor_account_id)
        return {
            "enabled": True,
            "network_id": self.config.network_id,
            "target_yocto": str(self.config.target_yocto),
            "sponsor_balance_yocto": str(state["amount"]),
            "reserve_floor_yocto": str(self.config.min_reserve_yocto),
        }

    def _send_transfer(self, receiver_id: str, amount_yocto: int) -> str:
        key = self.rpc.access_key(
            self.config.sponsor_account_id, self.config.sponsor_public_key
        )
        nonce = int(key["nonce"]) + 1
        block_hash = str(key["block_hash"])
        transaction = _encode_transfer_transaction(
            self.config.sponsor_account_id,
            self.config.sponsor_public_key,
            nonce,
            receiver_id,
            block_hash,
            amount_yocto,
        )
        digest = hashlib.sha256(transaction).digest()
        signature = self._private_key.sign(digest)
        signed = _encode_signed_transaction(transaction, signature)
        outcome = self.rpc.broadcast(signed)
        transaction_hash = (
            (outcome.get("transaction") or {}).get("hash")
            or (outcome.get("transaction_outcome") or {}).get("id")
        )
        if not transaction_hash:
            raise SponsorError("Sponsor transfer returned no transaction hash", 502, "rpc_invalid")
        return str(transaction_hash)

    def fund(self, account_id: str) -> dict[str, Any]:
        if not self.config.enabled:
            raise SponsorError("Testnet sponsor is disabled", 503, "sponsor_disabled")
        account_id = self._validate_account_id(account_id)

        with self._lock:
            prior = self.ledger.find(account_id)
            current = self.rpc.account_state(account_id)
            current_amount = int(current["amount"])

            if prior:
                return {
                    "status": "already_sponsored",
                    "account_id": account_id,
                    "target_yocto": str(self.config.target_yocto),
                    "balance_yocto": str(current_amount),
                    "tx_hash": prior.get("tx_hash"),
                    "grant_yocto": prior.get("grant_yocto", "0"),
                }

            if current_amount >= self.config.target_yocto:
                return {
                    "status": "already_funded",
                    "account_id": account_id,
                    "target_yocto": str(self.config.target_yocto),
                    "balance_yocto": str(current_amount),
                    "tx_hash": None,
                    "grant_yocto": "0",
                }

            nft_used = bool(
                self.rpc.view(
                    self.config.living_nft_id,
                    "starter_claim_used",
                    {"account_id": account_id},
                )
            )
            nuve_used = bool(
                self.rpc.view(
                    self.config.nuve_ft_id,
                    "has_claimed_starter",
                    {"account_id": account_id},
                )
            )
            if nft_used or nuve_used:
                raise SponsorError(
                    "Automatic funding is available before the Starter claim only",
                    409,
                    "demo_already_started",
                )

            sponsor_state = self.rpc.account_state(self.config.sponsor_account_id)
            sponsor_amount = int(sponsor_state["amount"])
            grant = self.config.target_yocto - current_amount
            if sponsor_amount - grant < self.config.min_reserve_yocto:
                raise SponsorError(
                    "Hackathon sponsor reserve is temporarily exhausted",
                    503,
                    "sponsor_reserve_exhausted",
                )

            tx_hash = self._send_transfer(account_id, grant)
            final_amount = current_amount
            for _ in range(12):
                time.sleep(0.5)
                final_amount = int(self.rpc.account_state(account_id)["amount"])
                if final_amount >= self.config.target_yocto:
                    break
            if final_amount < self.config.target_yocto:
                raise SponsorError(
                    "Sponsor transfer was submitted but final balance is not visible yet",
                    202,
                    "funding_pending",
                )

            record = {
                "schema": "nuvi-testnet-sponsor-claim-v1",
                "account_id": account_id,
                "grant_yocto": str(grant),
                "balance_before_yocto": str(current_amount),
                "balance_after_yocto": str(final_amount),
                "target_yocto": str(self.config.target_yocto),
                "tx_hash": tx_hash,
                "created_at_utc": datetime.now(timezone.utc).isoformat(),
            }
            if not self.ledger.append_if_absent(record):
                raise SponsorError("Sponsor claim raced with another request", 409, "claim_race")
            return {"status": "funded", **record}


def register_hackathon_sponsor(app):
    config = SponsorConfig.from_env()
    sponsor = NuviTestnetSponsor(config)

    def _origin_allowed() -> bool:
        origin = request.headers.get("Origin", "")
        return not origin or origin in config.allowed_origins

    @app.route("/api/hackathon/nuvi/testnet-fund/status", methods=["GET"])
    def nuvi_testnet_fund_status():
        try:
            return jsonify(sponsor.status()), 200
        except SponsorError as error:
            return jsonify({"error": error.code, "message": str(error)}), error.status_code

    @app.route("/api/hackathon/nuvi/testnet-fund", methods=["POST"])
    def nuvi_testnet_fund():
        if not _origin_allowed():
            return jsonify({"error": "origin_not_allowed", "message": "Origin is not allowed"}), 403
        payload = request.get_json(silent=True) or {}
        try:
            result = sponsor.fund(payload.get("account_id", ""))
            status_code = 202 if result.get("status") == "funding_pending" else 200
            return jsonify(result), status_code
        except SponsorError as error:
            return jsonify({"error": error.code, "message": str(error)}), error.status_code

    return sponsor

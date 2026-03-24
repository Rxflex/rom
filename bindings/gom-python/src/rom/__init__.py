from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any, Dict, Optional

try:
    if os.environ.get("ROM_FORCE_CLI_BRIDGE") == "1":
        raise ImportError("Native ROM bridge disabled by environment.")
    from . import _native
except ImportError:
    _native = None

_NativeRomRuntime = getattr(_native, "NativeRomRuntime", None) if _native is not None else None
_COOKIE_ATTRIBUTE_NAMES = {
    "domain",
    "path",
    "expires",
    "max-age",
    "samesite",
    "secure",
    "httponly",
}


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[4]


def _bridge_command() -> tuple[list[str], str]:
    bridge_bin = os.environ.get("ROM_BRIDGE_BIN")
    cwd = os.environ.get("ROM_BRIDGE_CWD", str(_repo_root()))
    if bridge_bin:
        return [bridge_bin], cwd

    return ["cargo", "run", "--quiet", "-p", "rom-runtime", "--bin", "rom_bridge"], cwd


def _run_bridge(command: str, payload: Dict[str, Any]) -> Any:
    if _native is not None:
        response = json.loads(_native.execute_bridge(json.dumps({"command": command, **payload})))
        if not response.get("ok"):
            raise RuntimeError(response.get("error") or "ROM native bridge failed.")
        return response

    args, cwd = _bridge_command()
    process = subprocess.run(
        args,
        input=json.dumps({"command": command, **payload}),
        text=True,
        capture_output=True,
        cwd=cwd,
        check=False,
    )

    if not process.stdout.strip():
        raise RuntimeError(process.stderr.strip() or "ROM bridge produced no output.")

    response = json.loads(process.stdout)
    if process.returncode != 0 or not response.get("ok"):
        raise RuntimeError(response.get("error") or process.stderr.strip() or "ROM bridge failed.")

    return response


def _normalize_runtime_config(config: Optional[Dict[str, Any]]) -> Dict[str, Any]:
    normalized = dict(config or {})
    cookie_store = _normalize_cookie_store_input(
        normalized.get("cookie_store", normalized.get("cookies")),
        normalized.get("href"),
    )
    local_storage = _normalize_storage_input(
        normalized.get("local_storage", normalized.get("localStorage"))
    )
    session_storage = _normalize_storage_input(
        normalized.get("session_storage", normalized.get("sessionStorage"))
    )
    normalized.pop("cookies", None)
    normalized.pop("localStorage", None)
    normalized.pop("sessionStorage", None)
    if cookie_store is not None:
        normalized["cookie_store"] = cookie_store
    if local_storage is not None:
        normalized["local_storage"] = local_storage
    if session_storage is not None:
        normalized["session_storage"] = session_storage
    return normalized


def _normalize_cookie_store_input(value: Any, href: Optional[str]) -> Optional[str]:
    if value in (None, ""):
        return None

    if isinstance(value, str):
        trimmed = value.strip()
        if not trimmed:
            return None
        if _looks_like_serialized_cookie_store(trimmed):
            return trimmed
        return _serialize_cookie_entries(_parse_cookie_header_string(trimmed, href))

    if isinstance(value, list):
        return _serialize_cookie_entries(_normalize_cookie_entries(value, href))

    if isinstance(value, dict):
        return _serialize_cookie_entries(_normalize_cookie_entries(value, href))

    return None


def _looks_like_serialized_cookie_store(value: str) -> bool:
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return False
    return isinstance(parsed, list)


def _normalize_storage_input(value: Any) -> Optional[str]:
    if value in (None, ""):
        return None

    if isinstance(value, str):
        trimmed = value.strip()
        if not trimmed or not _looks_like_serialized_storage(trimmed):
            return None
        return _serialize_storage_entries(json.loads(trimmed))

    if isinstance(value, (list, dict)):
        return _serialize_storage_entries(value)

    return None


def _looks_like_serialized_storage(value: str) -> bool:
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return False
    return isinstance(parsed, (list, dict))


def _normalize_cookie_entries(value: Any, href: Optional[str]) -> list[Dict[str, Any]]:
    if isinstance(value, list):
        return [entry for entry in (_normalize_cookie_entry_object(item, href) for item in value) if entry]

    return [_create_cookie_entry(name, entry_value, href) for name, entry_value in value.items()]


def _normalize_cookie_entry_object(entry: Any, href: Optional[str]) -> Optional[Dict[str, Any]]:
    if not isinstance(entry, dict):
        return None

    url = _safe_cookie_url(href)
    path = entry.get("path") if isinstance(entry.get("path"), str) and entry["path"].startswith("/") else "/"
    domain = (
        entry["domain"].strip().lstrip(".").lower()
        if isinstance(entry.get("domain"), str) and entry["domain"].strip()
        else url.hostname.lower()
    )

    return {
        "name": str(entry.get("name", "")),
        "value": str(entry.get("value", "")),
        "domain": domain,
        "hostOnly": entry.get("hostOnly", "domain" not in entry),
        "path": path,
        "secure": bool(entry.get("secure", url.scheme == "https")),
        "httpOnly": bool(entry.get("httpOnly", False)),
        "sameSite": _normalize_same_site(entry.get("sameSite")),
        "expiresAt": _normalize_expires_at(entry.get("expiresAt")),
    }


def _parse_cookie_header_string(value: str, href: Optional[str]) -> list[Dict[str, Any]]:
    entries: list[Dict[str, Any]] = []
    for part in (segment.strip() for segment in value.split(";")):
        if not part:
            continue
        separator = part.find("=")
        if separator <= 0:
            continue
        name = part[:separator].strip()
        if not name or name.lower() in _COOKIE_ATTRIBUTE_NAMES:
            continue
        entries.append(_create_cookie_entry(name, part[separator + 1 :], href))
    return entries


def _create_cookie_entry(name: Any, value: Any, href: Optional[str]) -> Dict[str, Any]:
    url = _safe_cookie_url(href)
    return {
        "name": str(name),
        "value": str(value),
        "domain": url.hostname.lower(),
        "hostOnly": True,
        "path": "/",
        "secure": url.scheme == "https",
        "httpOnly": False,
        "sameSite": "Lax",
        "expiresAt": None,
    }


def _safe_cookie_url(href: Optional[str]):
    from urllib.parse import urlparse

    parsed = urlparse(href or "https://rom.local/")
    if not parsed.scheme or not parsed.hostname:
        parsed = urlparse("https://rom.local/")
    return parsed


def _normalize_same_site(value: Any) -> str:
    normalized = str(value or "Lax").lower()
    if normalized == "strict":
        return "Strict"
    if normalized == "none":
        return "None"
    return "Lax"


def _normalize_expires_at(value: Any) -> Optional[float]:
    if value in (None, ""):
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _serialize_cookie_entries(entries: list[Dict[str, Any]]) -> Optional[str]:
    filtered = [entry for entry in entries if entry.get("name")]
    if not filtered:
        return None
    return json.dumps(filtered)


def _serialize_storage_entries(value: Any) -> Optional[str]:
    if isinstance(value, list):
        entries = {
            str(entry[0]): str(entry[1])
            for entry in value
            if isinstance(entry, (list, tuple)) and len(entry) >= 2
        }
        return json.dumps(entries)

    if isinstance(value, dict):
        return json.dumps({str(key): str(entry_value) for key, entry_value in value.items()})

    return None


class RomRuntime:
    def __init__(self, config: Optional[Dict[str, Any]] = None) -> None:
        self.config = _normalize_runtime_config(config)
        self._native_runtime = None
        if _NativeRomRuntime is not None:
            self._native_runtime = _NativeRomRuntime(json.dumps(self.config))

    def _apply_cookie_store(self) -> None:
        if self._native_runtime is None:
            return

        cookie_store = self._native_runtime.export_cookie_store()
        if isinstance(cookie_store, str):
            self.config = {**self.config, "cookie_store": cookie_store}

        if hasattr(self._native_runtime, "export_local_storage"):
            local_storage = self._native_runtime.export_local_storage()
            if isinstance(local_storage, str):
                self.config = {**self.config, "local_storage": local_storage}

        if hasattr(self._native_runtime, "export_session_storage"):
            session_storage = self._native_runtime.export_session_storage()
            if isinstance(session_storage, str):
                self.config = {**self.config, "session_storage": session_storage}

    def _run_native(self, command: str, payload: Dict[str, Any]) -> Any:
        if self._native_runtime is None:
            return None

        if command == "eval":
            result = self._native_runtime.eval(payload["script"])
        elif command == "eval-async":
            result = self._native_runtime.eval_async(payload["script"])
        elif command == "surface-snapshot":
            result = json.loads(self._native_runtime.surface_snapshot_json())
        elif command == "fingerprint-probe":
            result = json.loads(self._native_runtime.fingerprint_probe_json())
        elif command == "fingerprint-js-harness":
            result = json.loads(self._native_runtime.fingerprint_js_harness_json())
        elif command == "fingerprint-js-version":
            result = self._native_runtime.fingerprint_js_version()
        else:
            raise RuntimeError(f"Unsupported ROM native command: {command}")

        self._apply_cookie_store()
        return result

    def _run(self, command: str, payload: Dict[str, Any]) -> Any:
        if self._native_runtime is not None:
            return self._run_native(command, payload)

        response = _run_bridge(command, {"config": self.config, **payload})
        state = response.get("state")
        if isinstance(state, dict):
            next_config = dict(self.config)
            if isinstance(state.get("cookie_store"), str):
                next_config["cookie_store"] = state["cookie_store"]
            if isinstance(state.get("local_storage"), str):
                next_config["local_storage"] = state["local_storage"]
            if isinstance(state.get("session_storage"), str):
                next_config["session_storage"] = state["session_storage"]
            self.config = next_config
        return response.get("result")

    def eval(self, script: str) -> str:
        return self._run("eval", {"script": script})

    def eval_async(self, script: str) -> str:
        return self._run("eval-async", {"script": script})

    def eval_json(self, script: str, *, async_mode: bool = True) -> Any:
        value = self.eval_async(script) if async_mode else self.eval(script)
        return json.loads(value)

    def surface_snapshot(self) -> Dict[str, Any]:
        return self._run("surface-snapshot", {})

    def fingerprint_probe(self) -> Dict[str, Any]:
        return self._run("fingerprint-probe", {})

    def run_fingerprintjs_harness(self) -> Dict[str, Any]:
        return self._run("fingerprint-js-harness", {})

    def fingerprintjs_version(self) -> str:
        return self._run("fingerprint-js-version", {})


def create_runtime(config: Optional[Dict[str, Any]] = None) -> RomRuntime:
    return RomRuntime(config=config)


def has_native_binding() -> bool:
    return _native is not None


__all__ = ["RomRuntime", "create_runtime", "has_native_binding"]

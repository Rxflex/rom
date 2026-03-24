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


class RomRuntime:
    def __init__(self, config: Optional[Dict[str, Any]] = None) -> None:
        self.config = config or {}
        self._native_runtime = None
        if _NativeRomRuntime is not None:
            self._native_runtime = _NativeRomRuntime(json.dumps(self.config))

    def _apply_cookie_store(self) -> None:
        if self._native_runtime is None:
            return

        cookie_store = self._native_runtime.export_cookie_store()
        if isinstance(cookie_store, str):
            self.config = {**self.config, "cookie_store": cookie_store}

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
        if isinstance(state, dict) and isinstance(state.get("cookie_store"), str):
            self.config = {**self.config, "cookie_store": state["cookie_store"]}
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

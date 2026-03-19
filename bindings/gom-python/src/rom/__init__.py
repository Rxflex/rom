from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any, Dict, Optional


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[4]


def _bridge_command() -> tuple[list[str], str]:
    bridge_bin = os.environ.get("ROM_BRIDGE_BIN")
    cwd = os.environ.get("ROM_BRIDGE_CWD", str(_repo_root()))
    if bridge_bin:
        return [bridge_bin], cwd

    return ["cargo", "run", "--quiet", "-p", "rom-runtime", "--bin", "rom_bridge"], cwd


def _run_bridge(command: str, payload: Dict[str, Any]) -> Any:
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

    return response.get("result")


class RomRuntime:
    def __init__(self, config: Optional[Dict[str, Any]] = None) -> None:
        self.config = config or {}

    def eval(self, script: str) -> str:
        return _run_bridge("eval", {"config": self.config, "script": script})

    def eval_async(self, script: str) -> str:
        return _run_bridge("eval-async", {"config": self.config, "script": script})

    def eval_json(self, script: str, *, async_mode: bool = True) -> Any:
        value = self.eval_async(script) if async_mode else self.eval(script)
        return json.loads(value)

    def surface_snapshot(self) -> Dict[str, Any]:
        return _run_bridge("surface-snapshot", {"config": self.config})

    def fingerprint_probe(self) -> Dict[str, Any]:
        return _run_bridge("fingerprint-probe", {"config": self.config})

    def run_fingerprintjs_harness(self) -> Dict[str, Any]:
        return _run_bridge("fingerprint-js-harness", {"config": self.config})

    def fingerprintjs_version(self) -> str:
        return _run_bridge("fingerprint-js-version", {"config": self.config})


def create_runtime(config: Optional[Dict[str, Any]] = None) -> RomRuntime:
    return RomRuntime(config=config)


__all__ = ["RomRuntime", "create_runtime"]

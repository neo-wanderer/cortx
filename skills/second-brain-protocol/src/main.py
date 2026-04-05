#!/usr/bin/env python3
"""LibreFang skill: second-brain-protocol — thin passthrough to the cortx CLI."""
import json
import shlex
import subprocess
import sys


def run_cortx(args: list[str]) -> str:
    """Execute a cortx CLI command and return its output."""
    result = subprocess.run(
        ["cortx"] + args,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or f"cortx exited with code {result.returncode}")
    return result.stdout.strip()


def cortx(command: str) -> str:
    """Execute an arbitrary cortx command."""
    return run_cortx(shlex.split(command))


def cortx_schema(entity_type: str | None = None) -> str:
    """Get schema for a specific type, or list all types."""
    if entity_type:
        return run_cortx(["schema", "show", entity_type, "--format", "json"])
    return run_cortx(["schema", "types", "--format", "json"])


def main():
    payload = json.loads(sys.stdin.read())
    tool_name = payload["tool"]
    input_data = payload["input"]

    try:
        if tool_name == "cortx":
            result = cortx(input_data["command"])
        elif tool_name == "cortx_schema":
            result = cortx_schema(input_data.get("entity_type"))
        else:
            print(json.dumps({"error": f"Unknown tool: {tool_name}"}))
            return

        print(json.dumps({"result": result}))
    except Exception as e:
        print(json.dumps({"error": str(e)}))


if __name__ == "__main__":
    main()

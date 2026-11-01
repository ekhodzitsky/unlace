#!/usr/bin/env python3
"""Reject all Cargo dependencies, including optional, build, dev and target ones."""
import json
import subprocess

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--offline", "--locked", "--format-version", "1",
]))
assert len(metadata["packages"]) == 1, "third-party or extra workspace package found"
package = metadata["packages"][0]
assert not package["dependencies"], "Cargo dependencies are prohibited in every category"
print(json.dumps({"ok": True, "packages": 1, "dependencies": 0}))

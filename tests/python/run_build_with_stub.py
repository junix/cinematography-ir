"""Runs a generated build.py against the bpy stub and prints a JSON summary."""
import json
import os
import runpy
import sys

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(here, "bpy_stub"))

import bpy  # noqa: E402  (the stub)

script = sys.argv[1]
runpy.run_path(script, run_name="__main__")
summary = bpy.call_counts()
print(json.dumps(summary))

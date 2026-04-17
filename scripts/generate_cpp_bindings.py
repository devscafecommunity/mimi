#!/usr/bin/env python3
"""
FlatBuffers C++ code generation for MiMi protocol schemas.
Generates C++ header files from .fbs schema files.
"""

import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
PROTO_DIR = SCRIPT_DIR / "proto"
CPP_OUT_DIR = SCRIPT_DIR / "proto" / "generated" / "cpp"
SCHEMAS = [
    "proto/schema.fbs",
    "proto/metadata.fbs",
]

def main():
    CPP_OUT_DIR.mkdir(parents=True, exist_ok=True)
    
    for schema in SCHEMAS:
        schema_path = SCRIPT_DIR / schema
        if not schema_path.exists():
            print(f"ERROR: Schema file not found: {schema_path}", file=sys.stderr)
            return 1
        
        print(f"Generating C++ bindings for {schema}...")
        cmd = [
            "flatc",
            "--cpp",
            f"--cpp-std c++17",
            "-o", str(CPP_OUT_DIR),
            "-I", str(PROTO_DIR),
            str(schema_path),
        ]
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"ERROR: flatc compilation failed for {schema}", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            return 1
        
        print(f"✓ Generated C++ headers for {schema}")
    
    print(f"\nC++ bindings generated to: {CPP_OUT_DIR}")
    return 0

if __name__ == "__main__":
    sys.exit(main())

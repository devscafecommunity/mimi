#!/usr/bin/env python3
"""
Simple FlatBuffers schema validator.
Checks basic syntax without requiring the full flatc compiler.
"""

import re
import sys
from pathlib import Path

def validate_schema(filepath):
    """Basic schema validation."""
    content = Path(filepath).read_text()
    
    errors = []
    
    if "namespace" not in content:
        errors.append("Missing 'namespace' declaration")
    
    if "root_type" not in content:
        errors.append("Missing 'root_type' declaration")
    
    table_matches = re.findall(r'table (\w+)', content)
    if not table_matches:
        errors.append("No 'table' definitions found")
    
    enum_matches = re.findall(r'enum (\w+)', content)
    if not enum_matches:
        errors.append("No 'enum' definitions found")
    
    if re.search(r'required\);', content) and not re.search(r'required;', content):
        errors.append("Improper 'required' syntax found")
    
    unmatched_braces = content.count('{') - content.count('}')
    if unmatched_braces != 0:
        errors.append(f"Unmatched braces: +{unmatched_braces}")
    
    return errors

def main():
    schemas = [
        "proto/schema.fbs",
        "proto/metadata.fbs",
    ]
    
    all_errors = {}
    for schema in schemas:
        path = Path(schema)
        if not path.exists():
            print(f"ERROR: Schema not found: {schema}")
            return 1
        
        print(f"Validating {schema}...")
        errors = validate_schema(path)
        if errors:
            all_errors[schema] = errors
        else:
            print(f"  ✓ {schema} syntax OK")
    
    if all_errors:
        print("\nValidation errors found:")
        for schema, errors in all_errors.items():
            print(f"\n{schema}:")
            for err in errors:
                print(f"  - {err}")
        return 1
    
    print("\nAll schemas valid!")
    return 0

if __name__ == "__main__":
    sys.exit(main())

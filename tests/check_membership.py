"""Reject duplicate member IDs or domains before JSON parsing can hide them."""

import json
from collections import Counter
from pathlib import Path


membership_path = Path(__file__).resolve().parents[1] / "resources/membership.json"
members = json.loads(membership_path.read_text(encoding="utf-8"), object_pairs_hook=list)

ids = [member_id for member_id, _ in members]
domains = [dict(fields)["domain"] for _, fields in members]
for label, values in (("member ID", ids), ("domain", domains)):
    duplicates = sorted(value for value, count in Counter(values).items() if count > 1)
    if duplicates:
        raise SystemExit(f"Duplicate {label} in {membership_path}: {', '.join(duplicates)}")

print(f"Validated {len(members)} unique member IDs and domains")

# Fixture Cases

Each case contains:

- `left.txt`
- `right.txt`
- `case.json`

`case.json` records expected diff stats, expected warnings, and the ignore
mode used by the fixture-driven tests.

## Schema

```json
{
  "name": "human-readable case name",
  "ignore": "none | whitespace | whitespace-and-case",
  "expected": {
    "additions": 0,
    "deletions": 0,
    "unchanged": 0,
    "truncated": false
  },
  "expected_warnings": 0
}
```

- `expected_warnings` (optional, default 0): number of UTF-8 lossy-conversion
  warnings expected. Only set for fixtures with invalid-UTF-8 input files.

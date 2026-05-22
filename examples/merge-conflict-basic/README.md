# Merge Conflict Basic Example

These files scaffold the future `dt merge` workflow. The current CLI accepts the
arguments but merge resolution is not implemented yet.

```bash
cargo run -p diff-cli -- merge \
  --base examples/merge-conflict-basic/base.txt \
  --ours examples/merge-conflict-basic/ours.txt \
  --theirs examples/merge-conflict-basic/theirs.txt \
  --out /tmp/dt-merge-output.txt
```

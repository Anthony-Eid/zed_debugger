# Debugger walkthrough

Use this document to exercise Markdown rendering while reviewing the examples.

## Step through a score

1. Open `main.py` and place a breakpoint in `ScoreRecord.weighted`.
2. Run **Python: run examples** from the debugger.
3. Inspect `self.value`, `self.attempts`, and `penalty`.
4. Step out and compare the weighted score with the original value.

> A score of 80 on a second attempt has a weighted value of 76.

| Scenario | Input | Expected result |
| --- | --- | --- |
| Passing boundary | 60 | PASS |
| Below boundary | 59 | FAIL |
| Empty gradebook | No students | Average 0 |
| Many retries | 30 attempts | Weighted score 0 |

```python
record = ScoreRecord("Bob", "Math", 80, attempts=2)
assert record.weighted() == 76.0
```

## Review checklist

- [ ] Inspect a scalar, a list, and a nested record.
- [ ] Step into a helper and back to its caller.
- [ ] Compare edits across Python, PHP, C, and Rust.
- [ ] Check global provenance for source, configuration, and documentation edits.

See [the project guide](../README.md) for commands and prerequisites.

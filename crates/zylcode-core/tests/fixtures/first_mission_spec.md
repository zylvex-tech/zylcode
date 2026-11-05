# Mission specification — First Mission (`statslib` mean coverage)

## Intent

Give the `statslib` library honest test coverage for `mean()`, repair the defect
that coverage exposes, and leave the suite green — with every step recorded as
evidence. This is the end-to-end mission defined by Master Transformation
Prompt §36 (phase T3 of `docs/governance/ZYLCODE_TRANSFORMATION_PLAN.md`).

## Requirements

1. The baseline suite must pass in a freshly prepared workspace before any change.
2. New coverage for `mean([1, 2, 3])` must be added as a real test file.
3. Running the suite after that change must exhibit the real defect:
   non-zero exit, `FAIL:` and `AssertionError` in the captured output.
4. The root cause must be diagnosed from the captured output, not assumed.
5. The repair must change `statslib/core.py` and nothing else.
6. The same suite command must pass after the repair.
7. Every command's exit code and output must be captured, hashed, persisted,
   and linked from the evidence graph.

```json
{
  "intent": "Add mean() coverage to statslib, repair the defect the coverage exposes, and leave the suite green with full evidence.",
  "requirements": [
    "Baseline suite passes before any modification",
    "test_mean.py asserts mean([1, 2, 3]) == 2.0",
    "Suite fails after the new coverage, with FAIL: and AssertionError in output",
    "statslib/core.py is repaired so mean() divides by len(values)",
    "The same suite command passes after the repair"
  ],
  "verify_command": [
    "python",
    "-m",
    "unittest",
    "discover",
    "-v",
    "-s",
    ".",
    "-p",
    "test_*.py"
  ],
  "operations": [
    {
      "op": "write",
      "path": "test_mean.py",
      "content": "import unittest\n\nfrom statslib import mean\n\n\nclass TestMean(unittest.TestCase):\n    def test_mean_basic(self):\n        self.assertEqual(mean([1, 2, 3]), 2.0)\n\n\nif __name__ == \"__main__\":\n    unittest.main()\n"
    },
    {
      "op": "run",
      "id": "after_mean_coverage",
      "expect": "fail",
      "signature": ["FAIL:", "AssertionError"]
    },
    {
      "op": "replace",
      "path": "statslib/core.py",
      "old": "    return sum(values) / (len(values) - 1)",
      "new": "    return sum(values) / len(values)"
    },
    {
      "op": "run",
      "id": "after_repair",
      "expect": "pass",
      "signature": ["OK"]
    },
    {
      "op": "commit",
      "message": "repair: divide mean() by len(values)"
    }
  ]
}
```

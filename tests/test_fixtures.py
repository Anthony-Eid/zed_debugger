"""Keep editor fixtures easy to parse, even when launched from another directory."""

import csv
from datetime import date, datetime
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class FixtureTests(unittest.TestCase):
    def test_account_columns_match_the_header(self):
        with (ROOT / "src/test.csv").open(newline="") as source:
            rows = list(csv.DictReader(source))
        self.assertEqual(len(rows), 20)
        self.assertEqual(len({row["id"] for row in rows}), len(rows))
        for row in rows:
            with self.subTest(account=row["id"]):
                self.assertNotIn(None, row)
                self.assertIn(row["is_active"], {"true", "false"})
                signed_up = date.fromisoformat(row["signup_date"])
                last_login = datetime.fromisoformat(row["last_login"].replace("Z", "+00:00"))
                self.assertLessEqual(signed_up, last_login.date())
                self.assertGreaterEqual(float(row["account_balance"]), 0)

    def test_project_scores_allow_missing_values(self):
        with (ROOT / "src/project_scores.csv").open(newline="") as source:
            rows = list(csv.DictReader(source))
        self.assertEqual(len(rows), 62)
        for rank, row in enumerate(rows, start=1):
            with self.subTest(project=row["project"]):
                self.assertNotIn(None, row)
                self.assertEqual(int(row["rank"]), rank)
                self.assertGreater(int(row["review_count"]), 0)
                self.assertTrue(0 <= float(row["average_total"]) <= 15)
                for category in (
                    "innovation", "technical_difficulty", "design", "impact", "presentation"
                ):
                    if row[category]:
                        self.assertTrue(0 <= float(row[category]) <= 3)


if __name__ == "__main__":
    unittest.main()

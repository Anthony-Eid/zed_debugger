"""Boundary cases for the examples; requires only the Python standard library."""

import contextlib
import importlib
import io
import unittest

import main as examples


class ScoreTests(unittest.TestCase):
    def test_passing_boundary(self):
        self.assertFalse(examples.ScoreRecord("Alice", "Math", 59).is_passing())
        self.assertTrue(examples.ScoreRecord("Alice", "Math", 60).is_passing())

    def test_invalid_records(self):
        for value, attempts in [(-1, 1), (101, 1), (80, 0)]:
            with self.subTest(value=value, attempts=attempts):
                with self.assertRaises(ValueError):
                    examples.ScoreRecord("Alice", "Math", value, attempts)

    def test_retry_penalty_never_makes_scores_negative(self):
        self.assertEqual(examples.ScoreRecord("Bob", "Art", 80, 30).weighted(), 0)

    def test_record_lists_are_independent(self):
        first = examples.StudentProfile("Alice", 20)
        second = examples.StudentProfile("Bob", 21)
        first.add_record(examples.ScoreRecord("Alice", "Math", 90))
        self.assertEqual(second.records, [])

    def test_empty_gradebook(self):
        book = examples.GradeBook("Autumn")
        self.assertEqual(book.class_average(), 0.0)
        self.assertIsNone(book.top_student())

    def test_class_average_weights_students_equally(self):
        alice = examples.StudentProfile("Alice", 20)
        alice.add_record(examples.ScoreRecord("Alice", "Math", 100))
        bob = examples.StudentProfile("Bob", 21)
        bob.add_record(examples.ScoreRecord("Bob", "Math", 60))
        bob.add_record(examples.ScoreRecord("Bob", "Art", 40))
        book = examples.GradeBook("Autumn", [alice, bob])
        self.assertEqual(book.class_average(), 75.0)


class HelperTests(unittest.TestCase):
    def test_import_has_no_output(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            importlib.reload(examples)
        self.assertEqual(output.getvalue(), "")

    def test_empty_inputs(self):
        self.assertEqual(examples.calculate_average([]), 0.0)
        self.assertEqual(examples.build_user_profile(" Alice ", 20, [])["pass_rate"], 0)
        self.assertEqual(examples.flatten([]), [])
        self.assertEqual(examples.transpose([]), [])
        self.assertEqual(examples.summarize_payload([])["grand_total"], 0)

    def test_ragged_matrix_is_rejected(self):
        with self.assertRaises(ValueError):
            examples.transpose([[1, 2], [3]])

    def test_window_boundaries(self):
        self.assertEqual(list(examples.rolling_window([1, 2], 3)), [])
        for size in [0, -1]:
            with self.subTest(size=size), self.assertRaises(ValueError):
                list(examples.rolling_window([1, 2], size))

    def test_payload_is_repeatable(self):
        self.assertEqual(examples.make_big_payload(), examples.make_big_payload())
        self.assertEqual(
            examples.summarize_payload(examples.make_big_payload()),
            {"rows": 64, "even": 32, "odd": 32, "grand_total": 13056},
        )

    def test_introductory_examples(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(examples.run_tests(), 0)


if __name__ == "__main__":
    unittest.main()

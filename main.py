"""Small, deterministic examples for stepping through Python in a debugger."""

import math
import random
import string
from collections import defaultdict
from dataclasses import dataclass, field


PASSING_SCORE = 60
SAMPLE_SEED = 2026


@dataclass
class ScoreRecord:
    student: str
    subject: str
    value: int
    attempts: int = 1
    notes: list[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        if not 0 <= self.value <= 100:
            raise ValueError("score must be between 0 and 100")
        if self.attempts < 1:
            raise ValueError("attempts must be positive")

    def is_passing(self) -> bool:
        return self.value >= PASSING_SCORE

    def weighted(self) -> float:
        penalty = min((self.attempts - 1) * 0.05, 1.0)
        return self.value * (1.0 - penalty)

    def describe(self) -> str:
        status = "PASS" if self.is_passing() else "FAIL"
        return (
            f"{self.student}/{self.subject}: {self.value} "
            f"({status}, weighted={self.weighted():.2f})"
        )


@dataclass
class StudentProfile:
    name: str
    age: int
    records: list[ScoreRecord] = field(default_factory=list)
    metadata: dict[str, str] = field(default_factory=dict)

    def add_record(self, record: ScoreRecord) -> None:
        self.records.append(record)

    def passing_records(self) -> list[ScoreRecord]:
        return [r for r in self.records if r.is_passing()]

    def failing_records(self) -> list[ScoreRecord]:
        return [r for r in self.records if not r.is_passing()]

    def pass_rate(self) -> float:
        if not self.records:
            return 0.0
        return len(self.passing_records()) / len(self.records)

    def average(self) -> float:
        return calculate_average([record.value for record in self.records])

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "age": self.age,
            "record_count": len(self.records),
            "pass_rate": self.pass_rate(),
            "average": self.average(),
            "metadata": self.metadata,
        }


@dataclass
class GradeBook:
    term: str
    profiles: list[StudentProfile] = field(default_factory=list)

    def enroll(self, profile: StudentProfile) -> None:
        self.profiles.append(profile)

    def class_average(self) -> float:
        """Give each enrolled student equal weight, regardless of record count."""
        return calculate_average([profile.average() for profile in self.profiles])

    def top_student(self) -> StudentProfile | None:
        return max(self.profiles, key=lambda p: p.average(), default=None)


# Helpers


def calculate_average(numbers: list[int | float]) -> float:
    """Return zero for an empty sequence, otherwise its arithmetic mean."""
    return sum(numbers) / len(numbers) if numbers else 0.0


def build_user_profile(name, age, scores):
    """Normalize a name and partition scores without modifying the input."""
    buckets = defaultdict(list)
    for score in scores:
        buckets["passing" if score >= PASSING_SCORE else "failing"].append(score)
    passing_scores = buckets["passing"]
    failing_scores = buckets["failing"]
    profile = {
        "name": name.strip().title(),
        "age": age,
        "scores": list(scores),
        "passing_scores": passing_scores,
        "failing_scores": failing_scores,
        "score_count": len(scores),
        "pass_rate": len(passing_scores) / len(scores) if scores else 0,
    }
    profile["status"] = "active" if profile["pass_rate"] >= 0.5 else "needs-review"
    profile["checksum"] = sum(scores)
    return profile


def fibonacci(n):
    """Return the nth Fibonacci number using constant auxiliary space."""
    if n <= 0:
        return 0
    previous, current = 0, 1
    for _ in range(n):
        previous, current = current, previous + current
    return previous


def flatten(nested_list):
    """Flatten arbitrarily nested lists using an explicit stack."""
    result = []
    stack = list(reversed(nested_list))
    while stack:
        item = stack.pop()
        if isinstance(item, list):
            stack.extend(reversed(item))
        else:
            result.append(item)
    return result


def generate_matrix(rows, cols):
    """Generate a deterministic matrix of products."""
    return [[(r + 1) * (c + 1) for c in range(cols)] for r in range(rows)]


def transpose(matrix):
    """Transpose a matrix of equal-length rows."""
    if not matrix:
        return []
    if any(len(row) != len(matrix[0]) for row in matrix):
        raise ValueError("matrix rows must have equal lengths")
    return [list(col) for col in zip(*matrix)]


def matrix_diagonal_sum(matrix):
    """Sum the main diagonal of a square-ish matrix."""
    total = 0
    for i, row in enumerate(matrix):
        if i < len(row):
            total += row[i]
    return total


def rolling_window(values, size):
    """Yield rolling windows of a fixed size over a sequence."""
    if size <= 0:
        raise ValueError("window size must be positive")
    for start in range(len(values) - size + 1):
        yield values[start : start + size]


# Introductory tests


def test_average():
    nums = [4, 8, 15, 16, 23, 42]
    avg = calculate_average(nums)
    expected = sum(nums) / len(nums)
    assert avg == expected, f"Expected {expected}, got {avg}"


def test_user_profile_pass_rate():
    scores = [80, 45, 90, 55, 70, 100, 61, 12]
    profile = build_user_profile("  Alice Debugger  ", 25, scores)
    assert profile["pass_rate"] == 0.625, profile["pass_rate"]
    assert len(profile["passing_scores"]) == 5
    assert profile["name"] == "Alice Debugger"


def test_fibonacci():
    expected = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]
    for i, exp in enumerate(expected):
        result = fibonacci(i)
        assert result == exp, f"fibonacci({i}): expected {exp}, got {result}"


def test_flatten_deeply_nested():
    nested = [[1], [2, [3, [4, [5, [6]]]]], 7, [[8, 9]], 10]
    result = flatten(nested)
    assert result == [1, 2, 3, 4, 5, 6, 7, 8, 9, 10], result


def test_matrix_roundtrip():
    matrix = generate_matrix(3, 4)
    twice = transpose(transpose(matrix))
    assert matrix == twice


def test_diagonal_sum():
    matrix = generate_matrix(4, 4)
    assert matrix_diagonal_sum(matrix) == 1 + 4 + 9 + 16


def test_rolling_window():
    windows = list(rolling_window([1, 2, 3, 4], 2))
    assert windows == [[1, 2], [2, 3], [3, 4]]


def test_score_record_dataclass():
    record = ScoreRecord("Bob", "Math", 80, attempts=2)
    assert record.is_passing()
    assert abs(record.weighted() - 76.0) < 1e-9


def test_student_profile_dataclass():
    profile = StudentProfile("Carol", 22)
    profile.add_record(ScoreRecord("Carol", "Math", 90))
    profile.add_record(ScoreRecord("Carol", "Art", 40))
    assert profile.pass_rate() == 0.5
    assert profile.average() == 65.0


def test_gradebook():
    book = GradeBook("Fall")
    alice = StudentProfile("Alice", 20)
    alice.add_record(ScoreRecord("Alice", "Math", 100))
    bob = StudentProfile("Bob", 21)
    bob.add_record(ScoreRecord("Bob", "Math", 60))
    book.enroll(alice)
    book.enroll(bob)
    assert book.class_average() == 80.0
    assert book.top_student().name == "Alice"


def run_tests():
    tests = [
        ("test_average", test_average),
        ("test_user_profile_pass_rate", test_user_profile_pass_rate),
        ("test_fibonacci", test_fibonacci),
        ("test_flatten_deeply_nested", test_flatten_deeply_nested),
        ("test_matrix_roundtrip", test_matrix_roundtrip),
        ("test_diagonal_sum", test_diagonal_sum),
        ("test_rolling_window", test_rolling_window),
        ("test_score_record_dataclass", test_score_record_dataclass),
        ("test_student_profile_dataclass", test_student_profile_dataclass),
        ("test_gradebook", test_gradebook),
    ]
    passed = 0
    failed = 0
    for index, (name, fn) in enumerate(tests, start=1):
        try:
            fn()
            print(f"  PASS {index:02d}/{len(tests):02d} {name}")
            passed += 1
        except AssertionError as e:
            print(f"  FAIL {index:02d}/{len(tests):02d} {name}: {e}")
            failed += 1
    print(f"\nResults: {passed} passed, {failed} failed out of {len(tests)} tests")
    return failed


# Deterministic sample data


def make_big_payload(rows=64):
    """Generate repeatable nested values to inspect in a variables panel."""
    rng = random.Random(SAMPLE_SEED)
    payload = []
    for i in range(rows):
        payload.append(
            {
                "row": i,
                "label": "".join(rng.choice(string.ascii_lowercase) for _ in range(8)),
                "parity": "even" if i % 2 == 0 else "odd",
                "modulo": {"three": i % 3, "five": i % 5, "seven": i % 7},
                "values": [i + offset for offset in range(6)],
                "sqrt": round(math.sqrt(i), 4),
                "flags": {
                    "large": i > 40,
                    "tiny": i < 4,
                    "multiple_of_four": i % 4 == 0,
                },
            }
        )
    return payload


def summarize_payload(payload):
    counts = defaultdict(int)
    grand_total = 0
    for row in payload:
        counts[row["parity"]] += 1
        grand_total += sum(row["values"])
    return {
        "rows": len(payload),
        "even": counts["even"],
        "odd": counts["odd"],
        "grand_total": grand_total,
    }


def print_payload_summary():
    summary = summarize_payload(make_big_payload())
    print(
        "Payload summary: "
        f"rows={summary['rows']} even={summary['even']} "
        f"odd={summary['odd']} total={summary['grand_total']}"
    )


if __name__ == "__main__":
    print_payload_summary()
    raise SystemExit(bool(run_tests()))

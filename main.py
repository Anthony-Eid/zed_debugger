import os

current_pid = os.getpid()
print(f"Current process ID: {current_pid}")


# --- Helper functions with subtle bugs ---


def calculate_average(numbers):
    """Bug: off-by-one in the divisor."""
    total = 0
    for n in numbers:
        total += n
    return total / (len(numbers) + 1)  # off-by-one!


def build_user_profile(name, age, scores):
    """Bug: accidentally shadows the filtered list."""
    profile = {"name": name, "age": age}
    passing_scores = [s for s in scores if s >= 60]
    # Oops – reassigning to the unfiltered list
    passing_scores = scores
    profile["passing_scores"] = passing_scores
    profile["pass_rate"] = len(passing_scores) / len(scores)
    return profile


def fibonacci(n):
    """Bug: wrong base-case value makes every result off by 1."""
    if n <= 0:
        return 0
    if n == 1:
        return 2  # should be 1!
    return fibonacci(n - 1) + fibonacci(n - 2)


def flatten(nested_list):
    """Bug: forgets to recurse deeper than two levels."""
    result = []
    for item in nested_list:
        if isinstance(item, list):
            result.extend(item)  # only flattens one level
        else:
            result.append(item)
    return result


# --- Tests ---


def test_average():
    nums = [10, 20, 30]
    avg = calculate_average(nums)
    assert avg == 20.0, f"Expected 20.0, got {avg}"


def test_user_profile_pass_rate():
    scores = [80, 45, 90, 55, 70]
    profile = build_user_profile("Alice", 25, scores)
    # 3 out of 5 scores are >= 60, so pass_rate should be 0.6
    assert profile["pass_rate"] == 0.6, (
        f"Expected pass_rate 0.6, got {profile['pass_rate']}"
    )
    assert len(profile["passing_scores"]) == 3, (
        f"Expected 3 passing scores, got {len(profile['passing_scores'])}"
    )


def test_fibonacci():
    expected = [0, 1, 1, 2, 3, 5, 8, 13]
    for i, exp in enumerate(expected):
        result = fibonacci(i)
        assert result == exp, f"fibonacci({i}): expected {exp}, got {result}"


def test_flatten_deeply_nested():
    nested = [1, [2, [3, [4]], 5], 6]
    result = flatten(nested)
    assert result == [1, 2, 3, 4, 5, 6], f"Expected [1,2,3,4,5,6], got {result}"


def run_tests():
    tests = [
        ("test_average", test_average),
        ("test_user_profile_pass_rate", test_user_profile_pass_rate),
        ("test_fibonacci", test_fibonacci),
        ("test_flatten_deeply_nested", test_flatten_deeply_nested),
    ]

    passed = 0
    failed = 0
    for name, fn in tests:
        try:
            fn()
            print(f"  ✅ {name}")
            passed += 1
        except AssertionError as e:
            print(f"  ❌ {name}: {e}")
            failed += 1

    print(f"\nResults: {passed} passed, {failed} failed out of {len(tests)} tests")


if __name__ == "__main__":
    run_tests()

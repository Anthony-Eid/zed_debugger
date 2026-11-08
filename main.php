<?php

declare(strict_types=1);

// Keep this threshold aligned with the Python and C score examples.
const PASSING_SCORE = 60;

final class ScoreRecord
{
    public function __construct(
        public string $student,
        public string $subject,
        public int $value,
        public int $attempts = 1,
    ) {
        if ($value < 0 || $value > 100) {
            throw new InvalidArgumentException("score must be between 0 and 100");
        }
        if ($attempts < 1) {
            throw new InvalidArgumentException("attempts must be positive");
        }
    }

    public function isPassing(): bool
    {
        return $this->value >= PASSING_SCORE;
    }

    public function weighted(): float
    {
        $penalty = min(($this->attempts - 1) * 0.05, 1.0);
        return $this->value * (1.0 - $penalty);
    }

    public function describe(): string
    {
        $status = $this->isPassing() ? "PASS" : "FAIL";
        return sprintf(
            "%s/%s: %d (%s, weighted=%.2f)",
            $this->student,
            $this->subject,
            $this->value,
            $status,
            $this->weighted(),
        );
    }
}

final class GradeBook
{
    /** @var ScoreRecord[] */
    private array $records = [];

    public function add(ScoreRecord $record): void
    {
        $this->records[] = $record;
    }

    public function passRate(): float
    {
        if (count($this->records) === 0) {
            return 0.0;
        }
        $passing = array_filter(
            $this->records,
            static fn(ScoreRecord $record): bool => $record->isPassing(),
        );
        return count($passing) / count($this->records);
    }

    public function average(): float
    {
        if (count($this->records) === 0) {
            return 0.0;
        }
        $total = array_sum(array_map(
            static fn(ScoreRecord $record): int => $record->value,
            $this->records,
        ));
        return $total / count($this->records);
    }
}

/** @param list<int> $scores */
function buildProfile(string $name, int $age, array $scores): array
{
    $passing = array_values(array_filter(
        $scores,
        static fn(int $score): bool => $score >= PASSING_SCORE,
    ));
    $failing = array_values(array_filter(
        $scores,
        static fn(int $score): bool => $score < PASSING_SCORE,
    ));

    return [
        "name" => ucwords(strtolower(trim($name))),
        "age" => $age,
        "passing" => $passing,
        "failing" => $failing,
        "score_count" => count($scores),
        "pass_rate" =>
            count($scores) > 0 ? count($passing) / count($scores) : 0.0,
        "checksum" => array_sum($scores),
    ];
}

function generateRows(int $count): array
{
    if ($count < 0) {
        throw new InvalidArgumentException("row count cannot be negative");
    }
    $rows = [];
    for ($i = 0; $i < $count; $i++) {
        $rows[] = [
            "row" => $i,
            "square" => $i * $i,
            "parity" => $i % 2 === 0 ? "even" : "odd",
            "label" => sprintf("sample-row-%03d", $i),
        ];
    }
    return $rows;
}

$book = new GradeBook();
$records = [
    new ScoreRecord("Alice", "Math", 90, 1),
    new ScoreRecord("Bob", "Art", 40, 3),
    new ScoreRecord("Carol", "Science", 75, 2),
];

foreach ($records as $index => $record) {
    $book->add($record);
    echo sprintf("%02d %s\n", $index + 1, $record->describe());
}

echo sprintf(
    "Gradebook average=%.2f pass_rate=%.2f\n",
    $book->average(),
    $book->passRate(),
);

$profile = buildProfile("  alice debugger  ", 25, [
    80,
    45,
    90,
    55,
    70,
    100,
    61,
    12,
]);
echo "Profile name: {$profile["name"]} | pass_rate: {$profile["pass_rate"]}\n";

$rows = generateRows(4);
echo "Generated rows: " . count($rows) . "\n";
foreach ($rows as $row) {
    echo sprintf(
        "%s square=%04d parity=%s\n",
        $row["label"],
        $row["square"],
        $row["parity"],
    );
}

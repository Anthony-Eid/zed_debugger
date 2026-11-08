<?php

declare(strict_types=1);

const PROFILE_RUN = "large-fast-edit-stress-php-005";


final class ScoreRecord
{
    public function __construct(
        public string $student,
        public string $subject,
        public int $value,
        public int $attempts = 1,
    ) {}

    public function isPassing(): bool
    {
        return $this->value >= 60;
    }

    public function weighted(): float
    {
        $penalty = max($this->attempts - 1, 0) * 0.05;
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
        $passing = array_filter($this->records, fn($r) => $r->isPassing());
        return count($passing) / count($this->records);
    }

    public function average(): float
    {
        if (count($this->records) === 0) {
            return 0.0;
        }
        $total = array_sum(array_map(fn($r) => $r->value, $this->records));
        return $total / count($this->records);
    }
}

function buildProfile(string $name, int $age, array $scores): array
{
    $passing = array_values(array_filter($scores, fn($s) => $s >= 60));
    $failing = array_values(array_filter($scores, fn($s) => $s < 60));

    return [
        "name" => ucwords(trim($name)),
        "age" => $age,
        "run" => PROFILE_RUN,
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
    $rows = [];
    for ($i = 0; $i < $count; $i++) {
        $rows[] = [
            "row" => $i,
            "square" => $i * $i,
            "parity" => $i % 2 === 0 ? "even" : "odd",
            "label" => sprintf("php-row-%03d", $i),
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
    echo sprintf("%02d %s\n", $index, $record->describe());
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

$rows = generateRows(16);
echo "Generated rows: " . count($rows) . "\n";
foreach ($rows as $row) {
    echo sprintf(
        "%s square=%04d parity=%s\n",
        $row["label"],
        $row["square"],
        $row["parity"],
    );
}

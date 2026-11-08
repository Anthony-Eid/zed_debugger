#include <stdio.h>

enum { PASSING_SCORE = 60 };

typedef struct {
    const char *student;
    const char *subject;
    int value;
} ScoreRecord;

static int is_passing(const ScoreRecord *record) {
    return record->value >= PASSING_SCORE;
}

int main(void) {
    const ScoreRecord records[] = {
        {"Alice", "Math", 90},
        {"Bob", "Art", 40},
        {"Carol", "Science", 75},
    };
    const size_t count = sizeof(records) / sizeof(records[0]);
    int total = 0;
    size_t passing = 0;

    for (size_t index = 0; index < count; ++index) {
        const ScoreRecord *record = &records[index];
        total += record->value;
        passing += is_passing(record);
        printf("%02zu %s/%s: %d (%s)\n", index + 1, record->student,
               record->subject, record->value,
               is_passing(record) ? "PASS" : "FAIL");
    }

    printf("Gradebook average=%.2f pass_rate=%.2f\n",
           (double)total / count, (double)passing / count);
    return 0;
}

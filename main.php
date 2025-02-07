<?php

$test = true;

function changeTest(): void
{
    global $test;
    echo "Test: $test\n";
    $test = false;
    echo "Test: $test\n";
}

while (true) {
    sleep(10);
    break;
}

changeTest();

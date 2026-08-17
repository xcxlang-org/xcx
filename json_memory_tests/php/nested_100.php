<?php
$parent = ["val" => 0, "child" => null];

$t0 = microtime(true);

for ($i = 1; $i <= 100; $i++) {
    $parent = ["val" => $i, "child" => $parent];
}

$serialized = json_encode($parent);
$parsed = json_decode($serialized, true);

$explorer = $parsed;
for ($k = 1; $k <= 50; $k++) {
    $explorer = $explorer["child"];
}
$val_check = $explorer["val"];

$t1 = microtime(true);
$json_ms = ($t1 - $t0) * 1000.0;
echo "json_elapsed_ms: " . number_format($json_ms, 3, '.', '') . "\n";

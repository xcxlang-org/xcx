<?php
$payload_tbl = [];
for ($i = 0; $i < 50000; $i++) {
    $payload_tbl[] = ["id" => $i, "name" => "User_" . $i, "active" => true];
}

$t0 = microtime(true);

$data = $payload_tbl;
$raw_str = json_encode($data);
file_put_contents('flat_50k_temp.json', $raw_str);

$read_str = "";
if (file_exists('flat_50k_temp.json')) {
    $read_str = file_get_contents('flat_50k_temp.json');
    unlink('flat_50k_temp.json');
}

$parsed = json_decode($read_str, true);
$count = count($parsed);
$mid_elem = $parsed[25000];
$name_mid = $mid_elem["name"];

$t1 = microtime(true);
$json_ms = ($t1 - $t0) * 1000.0;
echo "json_elapsed_ms: " . number_format($json_ms, 3, '.', '') . "\n";

<?php
$payload_tbl = [];
for ($i = 0; $i < 1000; $i++) {
    $payload_tbl[] = ["id" => $i, "name" => "User_" . $i, "active" => true];
}

$t0 = microtime(true);

$tbl_json = json_decode(json_encode($payload_tbl), true);

$root_json = ["items" => [], "meta" => ["total" => 0, "processed" => false]];
$tbl_size = count($tbl_json);
for ($i = 0; $i < $tbl_size; $i++) {
    $item = $tbl_json[$i];
    $name = $item["name"];

    $profile = ["id" => 0, "username" => "", "ratings" => [], "flags" => ["valid" => false]];
    $profile["id"] = $i;
    $profile["username"] = $name;
    $profile["ratings"][] = 10;
    $profile["ratings"][] = 20;
    $profile["ratings"][] = 30;
    $profile["flags"]["valid"] = true;

    $root_json["items"][] = $profile;
}
$root_json["meta"]["total"] = $tbl_size;
$root_json["meta"]["processed"] = true;

$serialized_str = json_encode($root_json);

$parsed = json_decode($serialized_str, true);

$meta = $parsed["meta"];
$total_val = $meta["total"];

$sample_idx = (int) ($total_val / 2);
$sample = $parsed["items"][$sample_idx];
$name_check = $sample["username"];

$imported = [];
$wrapper = $parsed["items"];
for ($i = 0; $i < count($wrapper); $i++) {
    $it = $wrapper[$i];
    $imported[] = ["uid" => $it["id"], "uname" => $it["username"]];
}

$t1 = microtime(true);
$json_ms = ($t1 - $t0) * 1000.0;
echo "json_elapsed_ms: " . number_format($json_ms, 3, '.', '') . "\n";

<?php
$payload_tbl = [];
for ($i = 0; $i < 50000; $i++) {
    $payload_tbl[] = ["id" => $i, "val" => $i * 2, "active" => true];
}

$t0 = microtime(true);

$arr = json_decode(json_encode($payload_tbl), true);
$filtered_arr = [];

$size = count($arr);
for ($i = 0; $i < $size; $i++) {
    $item = $arr[$i];
    $val = $item["val"];

    if (($val % 4) === 0) {
        $item_map = ["val" => $val];
        $map_json = json_decode(json_encode($item_map), true);
        $map_json["id"] = $i;

        $filtered_arr[] = $map_json;
    }
}

$last_elem = $filtered_arr[count($filtered_arr) - 1];
$keys = array_keys($last_elem);

$final_tbl = [];
$wrapper = ["items" => $filtered_arr];

$items_to_inject = $wrapper["items"];
for ($i = 0; $i < count($items_to_inject); $i++) {
    $it = $items_to_inject[$i];
    $final_tbl[] = ["uid" => $it["id"], "uval" => $it["val"]];
}

$t1 = microtime(true);
$json_ms = ($t1 - $t0) * 1000.0;
echo "json_elapsed_ms: " . number_format($json_ms, 3, '.', '') . "\n";

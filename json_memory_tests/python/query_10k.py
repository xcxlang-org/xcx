import json
import time

payload_tbl = []
for i in range(10000):
    payload_tbl.append({"id": i, "val": i * 2, "active": True})

t0 = time.perf_counter()
arr = json.loads(json.dumps(payload_tbl))
filtered_arr = []
for i, item in enumerate(arr):
    val = item["val"]
    if val % 4 == 0:
        item_map = {"val": val}
        map_json = json.loads(json.dumps(item_map))
        map_json["id"] = i
        filtered_arr.append(map_json)

last_elem = filtered_arr[-1]
keys = list(last_elem.keys())

final_tbl = []
wrapper = {"items": filtered_arr}
items_to_inject = wrapper["items"]
for it in items_to_inject:
    final_tbl.append({"uid": it["id"], "uval": it["val"]})

t1 = time.perf_counter()
json_ms = (t1 - t0) * 1000.0
print(f"json_elapsed_ms: {json_ms:.3f}")

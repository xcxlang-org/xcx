import json
import time

payload_tbl = []
for i in range(50000):
    payload_tbl.append({"id": i, "name": f"User_{i}", "active": True})

t0 = time.perf_counter()
tbl_json = json.loads(json.dumps(payload_tbl))
root_json = {"items": [], "meta": {"total": 0, "processed": False}}
for i, item in enumerate(tbl_json):
    name = item["name"]
    profile = {"id": i, "username": name, "ratings": [10, 20, 30], "flags": {"valid": True}}
    root_json["items"].append(profile)
root_json["meta"]["total"] = len(tbl_json)
root_json["meta"]["processed"] = True

serialized_str = json.dumps(root_json)
parsed = json.loads(serialized_str)
meta = parsed["meta"]
total_val = meta["total"]
sample_idx = total_val // 2
sample = parsed["items"][sample_idx]
name_check = sample["username"]

imported = []
wrapper = parsed["items"]
for it in wrapper:
    imported.append({"uid": it["id"], "uname": it["username"]})

t1 = time.perf_counter()
json_ms = (t1 - t0) * 1000.0
print(f"json_elapsed_ms: {json_ms:.3f}")

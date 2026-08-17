import json
import time
import os

payload_tbl = []
for i in range(10000):
    payload_tbl.append({"id": i, "name": f"User_{i}", "active": True})

t0 = time.perf_counter()

data = payload_tbl
raw_str = json.dumps(data)
with open('flat_10000_temp.json', 'w') as f:
    f.write(raw_str)

read_str = ""
if os.path.exists('flat_10000_temp.json'):
    with open('flat_10000_temp.json', 'r') as f:
        read_str = f.read()
    os.remove('flat_10000_temp.json')

parsed = json.loads(read_str)
count = len(parsed)
mid_elem = parsed[5000]
name_mid = mid_elem["name"]

t1 = time.perf_counter()
json_ms = (t1 - t0) * 1000.0
print(f"json_elapsed_ms: {json_ms:.3f}")

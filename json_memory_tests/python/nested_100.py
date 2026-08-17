import json
import time

parent = {"val": 0, "child": None}
t0 = time.perf_counter()
for i in range(1, 100 + 1):
    parent = {"val": i, "child": parent}

serialized = json.dumps(parent)
parsed = json.loads(serialized)

explorer = parsed
for k in range(1, 50 + 1):
    explorer = explorer["child"]
val_check = explorer["val"]

t1 = time.perf_counter()
json_ms = (t1 - t0) * 1000.0
print(f"json_elapsed_ms: {json_ms:.3f}")

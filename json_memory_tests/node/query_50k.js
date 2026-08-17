const payload_tbl = [];
for (let i = 0; i < 50000; i++) {
    payload_tbl.push({ id: i, val: i * 2, active: true });
}

const t0 = performance.now();

const arr = JSON.parse(JSON.stringify(payload_tbl));
const filtered_arr = [];

const size = arr.length;
for (let i = 0; i < size; i++) {
    const item = arr[i];
    const val = item.val;

    if ((val % 4) === 0) {
        const item_map = new Map();
        item_map.set("val", val);

        const map_obj = Object.fromEntries(item_map);
        const map_json = JSON.parse(JSON.stringify(map_obj));
        map_json.id = i;

        filtered_arr.push(map_json);
    }
}

const last_elem = filtered_arr[filtered_arr.length - 1];
const keys = Object.keys(last_elem);

const final_tbl = [];
const wrapper = { items: filtered_arr };

const items_to_inject = wrapper.items;
for (let i = 0; i < items_to_inject.length; i++) {
    const it = items_to_inject[i];
    final_tbl.push({ uid: it.id, uval: it.val });
}

const t1 = performance.now();
const json_ms = t1 - t0;
console.log("json_elapsed_ms: " + json_ms.toFixed(3));

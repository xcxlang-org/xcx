const payload_tbl = [];
for (let i = 0; i < 10000; i++) {
    payload_tbl.push({ id: i, name: "User_" + i, active: true });
}

const t0 = performance.now();

const tbl_json = JSON.parse(JSON.stringify(payload_tbl));

const root_json = { items: [], meta: { total: 0, processed: false } };
const tbl_size = tbl_json.length;
for (let i = 0; i < tbl_size; i++) {
    const item = tbl_json[i];
    const name = item.name;

    const profile = { id: 0, username: "", ratings: [], flags: { valid: false } };
    profile.id = i;
    profile.username = name;
    profile.ratings.push(10);
    profile.ratings.push(20);
    profile.ratings.push(30);
    profile.flags.valid = true;

    root_json.items.push(profile);
}
root_json.meta.total = tbl_size;
root_json.meta.processed = true;

const serialized_str = JSON.stringify(root_json);

const parsed = JSON.parse(serialized_str);

const meta = parsed.meta;
const total_val = meta.total;

const sample_idx = Math.floor(total_val / 2);
const sample = parsed.items[sample_idx];
const name_check = sample.username;

const imported = [];
const wrapper = parsed.items;
for (let i = 0; i < wrapper.length; i++) {
    const it = wrapper[i];
    imported.push({ uid: it.id, uname: it.username });
}

const t1 = performance.now();
const json_ms = t1 - t0;
console.log("json_elapsed_ms: " + json_ms.toFixed(3));

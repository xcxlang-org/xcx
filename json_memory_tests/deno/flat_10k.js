const payload_tbl = [];
for (let i = 0; i < 10000; i++) {
    payload_tbl.push({ id: i, name: "User_" + i, active: true });
}

const t0 = performance.now();

const data = payload_tbl;
const raw_str = JSON.stringify(data);
Deno.writeTextFileSync('flat_10k_temp.json', raw_str);

let read_str = "";
try {
    read_str = Deno.readTextFileSync('flat_10k_temp.json');
    Deno.removeSync('flat_10k_temp.json');
} catch (e) { }

const parsed = JSON.parse(read_str);
const count = parsed.length;
const mid_elem = parsed[5000];
const name_mid = mid_elem.name;

const t1 = performance.now();
const json_ms = t1 - t0;
console.log("json_elapsed_ms: " + json_ms.toFixed(3));

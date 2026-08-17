const fs = require('fs');

const payload_tbl = [];
for (let i = 0; i < 50000; i++) {
    payload_tbl.push({ id: i, name: "User_" + i, active: true });
}

const t0 = performance.now();

const data = payload_tbl;
const raw_str = JSON.stringify(data);
fs.writeFileSync('flat_50k_temp.json', raw_str);

let read_str = "";
if (fs.existsSync('flat_50k_temp.json')) {
    read_str = fs.readFileSync('flat_50k_temp.json', 'utf8');
    fs.unlinkSync('flat_50k_temp.json');
}

const parsed = JSON.parse(read_str);
const count = parsed.length;
const mid_elem = parsed[25000];
const name_mid = mid_elem.name;

const t1 = performance.now();
const json_ms = t1 - t0;
console.log("json_elapsed_ms: " + json_ms.toFixed(3));

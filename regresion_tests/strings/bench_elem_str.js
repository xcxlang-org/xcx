const buf = ["", "", "", "", ""];
const t0 = performance.now();

for (let i = 0; i < 100000; i++) {
    buf[0] = buf[0] + "a";
}

const t1 = performance.now();
const ms = (t1 - t0).toFixed(3);
console.log(`Array elem str append 100k: ${ms} ms  |  len=${buf[0].length}`);
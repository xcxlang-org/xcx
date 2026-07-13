let res = "";
const t0 = performance.now();

for (let i = 0; i < 100000; i++) {
    res = res + "a" + "b";
}

const t1 = performance.now();
const ms = (t1 - t0).toFixed(3);
console.log(`General str append 100k: ${ms} ms  |  len=${res.length}`);

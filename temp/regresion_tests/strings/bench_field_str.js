const data = { log: "" };
const t0 = performance.now();

for (let i = 0; i < 100000; i++) {
    data.log = data.log + "a";
}

const t1 = performance.now();
const ms = (t1 - t0).toFixed(3);
console.log(`Field str append 100k: ${ms} ms  |  len=${data.log.length}`);

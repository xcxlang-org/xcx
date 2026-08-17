let parent = { val: 0, child: null };

const t0 = performance.now();

for (let i = 1; i <= 50; i++) {
    parent = { val: i, child: parent };
}

const serialized = JSON.stringify(parent);
const parsed = JSON.parse(serialized);

let explorer = parsed;
for (let k = 1; k <= 25; k++) {
    explorer = explorer.child;
}
const val_check = explorer.val;

const t1 = performance.now();
const json_ms = t1 - t0;
console.log("json_elapsed_ms: " + json_ms.toFixed(3));

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

function main() {
    const benchDir = 'B:\\workspace\\xcx_compiler_workspace\\xcx-benchmarks\\Benchmarks\\loop_suite\\xcx';
    let binary = 'B:\\workspace\\xcx_compiler_workspace\\target\\release\\xcx.exe';

    if (!fs.existsSync(binary)) {
        console.log(`XCX executable not found at ${binary}. Looking in PATH...`);
        binary = 'xcx';
    }

    if (!fs.existsSync(benchDir) || !fs.statSync(benchDir).isDirectory()) {
        console.error(`Benchmark directory not found: ${benchDir}`);
        process.exit(1);
    }

    let xcxFiles = fs.readdirSync(benchDir).filter(f => f.endsWith('.xcx'));
    xcxFiles.sort();

    if (xcxFiles.length === 0) {
        console.error(`No .xcx files found in ${benchDir}`);
        process.exit(1);
    }

    console.log(`Found ${xcxFiles.length} benchmark files.`);
    console.log('='.repeat(115));
    console.log(`${'Benchmark'.padEnd(35)} | ${'Runs (1-6)'.padEnd(60)} | ${'Average'}`);
    console.log('-'.repeat(115));

    const msPattern = /(\d+\.?\d*)\s*ms/;
    let totalAvg = 0.0;

    for (const fileName of xcxFiles) {
        const filePath = path.join(benchDir, fileName);

        // 1. Warmup run
        try {
            execFileSync(binary, [filePath], { stdio: 'pipe', encoding: 'utf8' });
        } catch (e) {
            console.log(`${fileName.padEnd(35)} | Warmup failed! ${e.stderr || e.message}`);
            continue;
        }

        // 2. Benchmark runs (6 times)
        const times = [];
        let success = true;
        for (let i = 0; i < 6; i++) {
            try {
                const stdout = execFileSync(binary, [filePath], { stdio: 'pipe', encoding: 'utf8' });
                const stdoutCleaned = stdout.replace(/\x1b\[K/g, '');
                let match = msPattern.exec(stdoutCleaned);
                if (match) {
                    times.push(parseFloat(match[1]));
                } else {
                    console.warn(`Warning: Couldn't parse duration from stdout of ${fileName}`);
                    success = false;
                    break;
                }
            } catch (e) {
                console.log(`Error during execution of ${fileName}: ${e.message}`);
                success = false;
                break;
            }
        }

        if (success && times.length === 6) {
            const avg = times.reduce((a, b) => a + b, 0) / 6;
            totalAvg += avg;
            const runsStr = times.map(t => `${t.toFixed(2)} ms`).join(', ');
            console.log(`${fileName.padEnd(35)} | ${runsStr.padEnd(60)} | ${avg.toFixed(2)} ms`);
        } else {
            console.log(`${fileName.padEnd(35)} | Failed to retrieve 6 valid runs.`);
        }
    }

    console.log('='.repeat(115));
    console.log(`Total Suite Execution Time (Sum of Averages): ${totalAvg.toFixed(2)} ms`);
}

main();

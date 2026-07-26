import os
import subprocess
import re
import sys

def main():
    bench_dir = r"B:\workspace\xcx_compiler_workspace\xcx-benchmarks\Benchmarks\loop_suite\xcx"
    binary = r"B:\workspace\xcx_compiler_workspace\target\release\xcx.exe"
    
    if not os.path.exists(binary):
        print(f"XCX executable not found at {binary}. Looking in PATH...")
        binary = "xcx"

    if not os.path.isdir(bench_dir):
        print(f"Benchmark directory not found: {bench_dir}")
        sys.exit(1)

    xcx_files = [f for f in os.listdir(bench_dir) if f.endswith(".xcx")]
    xcx_files.sort()

    if not xcx_files:
        print(f"No .xcx files found in {bench_dir}")
        sys.exit(1)

    print(f"Found {len(xcx_files)} benchmark files.")
    print("=" * 70)
    print(f"{'Benchmark':<35} | {'Runs (1-3)':<30} | {'Average':<12}")
    print("-" * 85)

    ms_pattern = re.compile(r"(\d+\.?\d*)\s*ms")
    total_avg = 0.0

    for file_name in xcx_files:
        file_path = os.path.join(bench_dir, file_name)
        
        # 1. Warmup run
        try:
            subprocess.run([binary, file_path], capture_output=True, text=True, check=True)
        except subprocess.CalledProcessError as e:
            print(f"{file_name:<35} | Warmup failed! {e.stderr.strip()}")
            continue
        except Exception as e:
            print(f"{file_name:<35} | Warmup failed! {e}")
            continue

        # 2. Benchmark runs (3 times)
        times = []
        for _ in range(3):
            try:
                res = subprocess.run([binary, file_path], capture_output=True, text=True, check=True)
                stdout_cleaned = res.stdout.replace("\x1b[K", "")
                match = ms_pattern.search(stdout_cleaned)
                if match:
                    times.append(float(match.group(1)))
                else:
                    # Fallback to searching stderr just in case
                    stderr_cleaned = res.stderr.replace("\x1b[K", "")
                    match_err = ms_pattern.search(stderr_cleaned)
                    if match_err:
                        times.append(float(match_err.group(1)))
                    else:
                        print(f"Warning: Couldn't parse duration from stdout/stderr of {file_name}")
            except Exception as e:
                print(f"Error during execution of {file_name}: {e}")
                break

        if len(times) == 3:
            avg = sum(times) / 3
            total_avg += avg
            runs_str = ", ".join(f"{t:.2f} ms" for t in times)
            print(f"{file_name:<35} | {runs_str:<30} | {avg:.2f} ms")
        else:
            print(f"{file_name:<35} | Failed to retrieve 3 valid runs.")

    print("=" * 85)
    print(f"Total Suite Execution Time (Sum of Averages): {total_avg:.2f} ms")

if __name__ == "__main__":
    main()

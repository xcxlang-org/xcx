import subprocess
import time
import os
import sys

try:
    import psutil
except ImportError:
    print("psutil not found, installing via pip...")
    subprocess.run([sys.executable, "-m", "pip", "install", "psutil"], check=True)
    import psutil

BIN_PATH = r"..\target\release\xcx.exe"
if not os.path.exists(BIN_PATH):
    BIN_PATH = r"target\release\xcx.exe"

import subprocess
import time
import os
import sys

try:
    import psutil
except ImportError:
    print("psutil not found, installing via pip...")
    subprocess.run([sys.executable, "-m", "pip", "install", "psutil"], check=True)
    import psutil

BIN_PATH = r"..\target\release\xcx.exe"
if not os.path.exists(BIN_PATH):
    BIN_PATH = r"target\release\xcx.exe"

def is_cmd_available(cmd):
    try:
        probe = "--version" if cmd != "bun" else "bun"
        subprocess.run([cmd, probe if cmd != "bun" else "--version"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return True
    except FileNotFoundError:
        return False

def profile_run(args):
    proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        p = psutil.Process(proc.pid)
    except psutil.NoSuchProcess:
        return None
        
    rss_samples = []
    vms_samples = []
    
    while proc.poll() is None:
        try:
            mem = p.memory_info()
            rss_samples.append(mem.rss)
            vms_samples.append(mem.vms)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            break
        time.sleep(0.002) # Sample every 2ms
        
    stdout, stderr = proc.communicate()
    
    import re
    elapsed = 0.0
    for line in stdout.splitlines():
        match = re.search(r"json_elapsed_ms:\s*(\d+\.?\d*)", line)
        if match:
            try:
                elapsed = float(match.group(1))
            except ValueError:
                pass
                    
    if not rss_samples:
        return None
        
    mb = 1024 * 1024
    
    return {
        "min_rss": min(rss_samples) / mb,
        "max_rss": max(rss_samples) / mb,
        "avg_rss": (sum(rss_samples) / len(rss_samples)) / mb,
        "max_vms": max(vms_samples) / mb,
        "elapsed_ms": elapsed,
        "stdout": stdout,
        "stderr": stderr
    }

def generate_svg_chart(results, filename, title, ylabel, value_key):
    heavy_scenarios = ["Heavy Ops 1K", "Heavy Ops 10K", "Heavy Ops 50K", "Heavy Ops 100K"]
    x_scales = [1, 10, 50, 100]
    runtimes = ["XCX (JIT)", "XCX (Interpreter)", "Node.js", "Bun", "Deno", "Python", "Ruby", "PHP"]
    colors = {
        "XCX (JIT)": "#FF6B6B",
        "XCX (Interpreter)": "#4D96FF",
        "Node.js": "#6BCB77",
        "Bun": "#FFA45B",
        "Deno": "#F472B6",
        "Python": "#38BDF8",
        "Ruby": "#BE123C",
        "PHP": "#B983FF"
    }

    max_val = 0.0
    for rt in runtimes:
        for sc in heavy_scenarios:
            match = [r for r in results if r["scenario"] == sc and r["runtime"] == rt]
            if match and match[0][value_key] is not None:
                max_val = max(max_val, match[0][value_key])
                
    if max_val == 0:
        max_val = 1.0
        
    import math
    order = 10 ** math.floor(math.log10(max_val)) if max_val > 0 else 1
    if order == 0: order = 1
    nice_max = math.ceil(max_val / (order / 2)) * (order / 2)
    if nice_max == max_val:
        nice_max += order / 2

    w, h = 800, 500
    margin_l, margin_r, margin_t, margin_b = 85, 175, 75, 75
    plot_w = w - margin_l - margin_r
    plot_h = h - margin_t - margin_b

    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" height="500" style="background-color: #1e1e1e; font-family: system-ui, -apple-system, sans-serif;">')
    
    # Title
    svg.append(f'<text x="{w/2}" y="35" text-anchor="middle" font-size="18" font-weight="bold" fill="#ffffff">{title}</text>')
    
    # Axis grids and ticks (Y-axis)
    ticks_y = 5
    for i in range(ticks_y + 1):
        val = (nice_max / ticks_y) * i
        y = (h - margin_b) - (val / nice_max) * plot_h
        svg.append(f'<line x1="{margin_l}" y1="{y}" x2="{w - margin_r}" y2="{y}" stroke="#333333" stroke-dasharray="4,4" stroke-width="1" />')
        svg.append(f'<text x="{margin_l - 10}" y="{y + 4}" text-anchor="end" font-size="12" fill="#aaaaaa">{val:.1f}</text>')

    # Axis ticks (X-axis)
    for i, scale in enumerate(x_scales):
        x = margin_l + (scale / 100.0) * plot_w
        svg.append(f'<line x1="{x}" y1="{h - margin_b}" x2="{x}" y2="{h - margin_b + 5}" stroke="#8E9297" stroke-width="1.5" />')
        svg.append(f'<text x="{x}" y="{h - margin_b + 20}" text-anchor="middle" font-size="12" fill="#aaaaaa">{scale}K</text>')

    # Labels
    svg.append(f'<text x="{margin_l + plot_w/2}" y="{h - 15}" text-anchor="middle" font-size="13" fill="#aaaaaa">Data Scale (Elements)</text>')
    svg.append(f'<text x="25" y="{h/2}" text-anchor="middle" font-size="13" fill="#aaaaaa" transform="rotate(-90 25 {h/2})">{ylabel}</text>')

    # Border lines
    svg.append(f'<line x1="{margin_l}" y1="{margin_t}" x2="{margin_l}" y2="{h - margin_b}" stroke="#aaaaaa" stroke-width="1.5" />')
    svg.append(f'<line x1="{margin_l}" y1="{h - margin_b}" x2="{w - margin_r}" y2="{h - margin_b}" stroke="#aaaaaa" stroke-width="1.5" />')

    # Draw lines and markers
    for rt in runtimes:
        points = []
        valid_points = True
        for sc in heavy_scenarios:
            match = [r for r in results if r["scenario"] == sc and r["runtime"] == rt]
            if match and match[0][value_key] is not None:
                val = match[0][value_key]
                scale = float(sc.split(" ")[-1][:-1]) # gets 1, 10, 50, 100
                px = margin_l + (scale / 100.0) * plot_w
                py = (h - margin_b) - (val / nice_max) * plot_h
                points.append((px, py))
            else:
                valid_points = False
                break
                
        if valid_points and points:
            color = colors[rt]
            path_str = f"M {points[0][0]:.1f} {points[0][1]:.1f} "
            for px, py in points[1:]:
                path_str += f"L {px:.1f} {py:.1f} "
            svg.append(f'<path d="{path_str}" fill="none" stroke="{color}" stroke-width="2.5" />')
            
            for px, py in points:
                svg.append(f'<circle cx="{px:.1f}" cy="{py:.1f}" r="4.5" fill="{color}" stroke="#1e1e1e" stroke-width="1.5" />')

    # Legend
    legend_x = w - margin_r + 15
    legend_y = margin_t + 10
    svg.append(f'<rect x="{legend_x - 5}" y="{legend_y - 15}" width="160" height="{len(runtimes)*25 + 10}" fill="#2d2d2d" stroke="#444444" rx="5" ry="5" />')
    for idx, rt in enumerate(runtimes):
        ly = legend_y + idx * 25
        color = colors[rt]
        svg.append(f'<line x1="{legend_x}" y1="{ly}" x2="{legend_x + 15}" y2="{ly}" stroke="{color}" stroke-width="2.5" />')
        svg.append(f'<circle cx="{legend_x + 7.5}" cy="{ly}" r="3.5" fill="{color}" />')
        svg.append(f'<text x="{legend_x + 25}" y="{ly + 4}" fill="#ffffff" font-size="12" font-weight="500">{rt}</text>')

    svg.append('</svg>')
    
    with open(filename, 'w') as f:
        f.write('\n'.join(svg))
    print(f"Generated chart: {os.path.basename(filename)}")

def generate_bar_chart(results, filename, title, ylabel, value_key, target_scenarios=None):
    if target_scenarios is None:
        target_scenarios = ["Scenario A (Flat 50K)", "Scenario B (Nested 100)", "Scenario C (Query 50K)"]
        
    runtimes = ["XCX (JIT)", "XCX (Interpreter)", "Node.js", "Bun", "Deno", "Python", "Ruby", "PHP"]
    colors = {
        "XCX (JIT)": "#FF6B6B",
        "XCX (Interpreter)": "#4D96FF",
        "Node.js": "#6BCB77",
        "Bun": "#FFA45B",
        "Deno": "#F472B6",
        "Python": "#38BDF8",
        "Ruby": "#BE123C",
        "PHP": "#B983FF"
    }

    max_val = 0.0
    for rt in runtimes:
        for sc in target_scenarios:
            match = [r for r in results if r["scenario"] == sc and r["runtime"] == rt]
            if match and match[0][value_key] is not None:
                max_val = max(max_val, match[0][value_key])
                
    if max_val == 0:
        max_val = 1.0
        
    import math
    order = 10 ** math.floor(math.log10(max_val)) if max_val > 0 else 1
    if order == 0: order = 1
    nice_max = math.ceil(max_val / (order / 2)) * (order / 2)
    if nice_max == max_val:
        nice_max += order / 2

    w, h = 800, 500
    margin_l, margin_r, margin_t, margin_b = 85, 175, 75, 75
    plot_w = w - margin_l - margin_r
    plot_h = h - margin_t - margin_b

    svg = []
    svg.append(f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" height="500" style="background-color: #1e1e1e; font-family: system-ui, -apple-system, sans-serif;">')
    
    # Title
    svg.append(f'<text x="{w/2}" y="35" text-anchor="middle" font-size="18" font-weight="bold" fill="#ffffff">{title}</text>')
    
    # Axis grids and ticks (Y-axis)
    ticks_y = 5
    for i in range(ticks_y + 1):
        val = (nice_max / ticks_y) * i
        y = (h - margin_b) - (val / nice_max) * plot_h
        svg.append(f'<line x1="{margin_l}" y1="{y}" x2="{w - margin_r}" y2="{y}" stroke="#333333" stroke-dasharray="4,4" stroke-width="1" />')
        svg.append(f'<text x="{margin_l - 10}" y="{y + 4}" text-anchor="end" font-size="12" fill="#aaaaaa">{val:.1f}</text>')

    # Border lines
    svg.append(f'<line x1="{margin_l}" y1="{margin_t}" x2="{margin_l}" y2="{h - margin_b}" stroke="#aaaaaa" stroke-width="1.5" />')
    svg.append(f'<line x1="{margin_l}" y1="{h - margin_b}" x2="{w - margin_r}" y2="{h - margin_b}" stroke="#aaaaaa" stroke-width="1.5" />')

    # Draw bars group by group
    group_width = plot_w / len(target_scenarios)
    bar_width = 15
    gap = 2
    num_runtimes = len(runtimes)
    total_bars_width = num_runtimes * bar_width + (num_runtimes - 1) * gap
    side_padding = (group_width - total_bars_width) / 2
    
    for g_idx, sc_name in enumerate(target_scenarios):
        # group center X
        gx = margin_l + g_idx * group_width + group_width / 2
        # tick mark
        svg.append(f'<line x1="{gx}" y1="{h - margin_b}" x2="{gx}" y2="{h - margin_b + 5}" stroke="#8E9297" stroke-width="1.5" />')
        # label
        short_name = sc_name.replace("Scenario ", "")
        svg.append(f'<text x="{gx}" y="{h - margin_b + 20}" text-anchor="middle" font-size="12" font-weight="bold" fill="#aaaaaa">{short_name}</text>')
        
        for r_idx, rt in enumerate(runtimes):
            match = [r for r in results if r["scenario"] == sc_name and r["runtime"] == rt]
            if match and match[0][value_key] is not None:
                val = match[0][value_key]
                bx = margin_l + g_idx * group_width + side_padding + r_idx * (bar_width + gap)
                bar_h = (val / nice_max) * plot_h
                by = (h - margin_b) - bar_h
                color = colors[rt]
                # draw bar
                svg.append(f'<rect x="{bx:.1f}" y="{by:.1f}" width="{bar_width}" height="{max(bar_h, 1.0):.1f}" fill="{color}" rx="2" ry="2" />')
                # value labels above bar
                if val > 0:
                    font_sz = 9 if val >= 100 else 10
                    svg.append(f'<text x="{bx + bar_width/2:.1f}" y="{by - 4:.1f}" text-anchor="middle" font-size="{font_sz}" fill="#ffffff" opacity="0.8">{val:.1f}</text>')

    # Labels
    svg.append(f'<text x="25" y="{h/2}" text-anchor="middle" font-size="13" fill="#aaaaaa" transform="rotate(-90 25 {h/2})">{ylabel}</text>')

    # Legend
    legend_x = w - margin_r + 15
    legend_y = margin_t + 10
    svg.append(f'<rect x="{legend_x - 5}" y="{legend_y - 15}" width="160" height="{len(runtimes)*25 + 10}" fill="#2d2d2d" stroke="#444444" rx="5" ry="5" />')
    for idx, rt in enumerate(runtimes):
        ly = legend_y + idx * 25
        color = colors[rt]
        svg.append(f'<rect x="{legend_x}" y="{ly - 6}" width="15" height="12" fill="{color}" rx="1" ry="1" />')
        svg.append(f'<text x="{legend_x + 25}" y="{ly + 4}" fill="#ffffff" font-size="12" font-weight="500">{rt}</text>')

    svg.append('</svg>')
    
    with open(filename, 'w') as f:
        f.write('\n'.join(svg))
    print(f"Generated chart: {os.path.basename(filename)}")

def generate_combined_dashboard(results, filename):
    file_configs = [
        ("heavy_ops_memory.svg", 0, 0),
        ("heavy_ops_duration.svg", 800, 0),
        ("scenarios_bar_memory.svg", 0, 500),
        ("scenarios_bar_duration.svg", 800, 500)
    ]
    
    dashboard_svg = []
    dashboard_svg.append('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1600 1000" width="100%" height="1000" style="background-color: #1e1e1e; font-family: system-ui, -apple-system, sans-serif;">')
    
    for fname, x, y in file_configs:
        path = os.path.join("json_memory_tests", fname)
        if not os.path.exists(path):
            print(f"Warning: {path} not found when generating dashboard.", file=sys.stderr)
            continue
        with open(path, 'r') as f:
            content = f.read()
            
        start_idx = content.find('<svg')
        if start_idx == -1:
            continue
        first_gt = content.find('>', start_idx)
        if first_gt == -1:
            continue
        end_idx = content.rfind('</svg>')
        if end_idx == -1:
            continue
            
        inner_content = content[first_gt + 1:end_idx]
        dashboard_svg.append(f'  <svg x="{x}" y="{y}" width="800" height="500" viewBox="0 0 800 500">')
        dashboard_svg.append(inner_content)
        dashboard_svg.append('  </svg>')
        
    dashboard_svg.append('</svg>')
    
    with open(filename, 'w') as f:
        f.write('\n'.join(dashboard_svg))
    print(f"Generated combined dashboard: {os.path.basename(filename)}")

def main():
    if not os.path.exists(BIN_PATH):
        print(f"XCX executable not found at {BIN_PATH}")
        sys.exit(1)

    has_bun = is_cmd_available("bun")
    has_deno = is_cmd_available("deno")
    has_php = is_cmd_available("php")
    has_node = is_cmd_available("node")
    has_python = is_cmd_available("python") or is_cmd_available("python3")
    py_cmd = "python" if is_cmd_available("python") else "python3"
    has_ruby = is_cmd_available("ruby")

    scenarios = [
        {
            "name": "Scenario A (Flat 10K)",
            "xcx": "xcx/flat_10k.xcx",
            "node": "node/flat_10k.js",
            "bun": "bun/flat_10k.js",
            "deno": "deno/flat_10k.js",
            "php": "php/flat_10k.php",
            "python": "python/flat_10k.py",
            "ruby": "ruby/flat_10k.rb",
        },
        {
            "name": "Scenario A (Flat 50K)",
            "xcx": "xcx/flat_50k.xcx",
            "node": "node/flat_50k.js",
            "bun": "bun/flat_50k.js",
            "deno": "deno/flat_50k.js",
            "php": "php/flat_50k.php",
            "python": "python/flat_50k.py",
            "ruby": "ruby/flat_50k.rb",
        },
        {
            "name": "Scenario B (Nested 50)",
            "xcx": "xcx/nested_1k.xcx",
            "node": "node/nested_50.js",
            "bun": "bun/nested_50.js",
            "deno": "deno/nested_50.js",
            "php": "php/nested_50.php",
            "python": "python/nested_50.py",
            "ruby": "ruby/nested_50.rb",
        },
        {
            "name": "Scenario B (Nested 100)",
            "xcx": "xcx/nested_5k.xcx",
            "node": "node/nested_100.js",
            "bun": "bun/nested_100.js",
            "deno": "deno/nested_100.js",
            "php": "php/nested_100.php",
            "python": "python/nested_100.py",
            "ruby": "ruby/nested_100.rb",
        },
        {
            "name": "Scenario C (Query 10K)",
            "xcx": "xcx/query_10k.xcx",
            "node": "node/query_10k.js",
            "bun": "bun/query_10k.js",
            "deno": "deno/query_10k.js",
            "php": "php/query_10k.php",
            "python": "python/query_10k.py",
            "ruby": "ruby/query_10k.rb",
        },
        {
            "name": "Scenario C (Query 50K)",
            "xcx": "xcx/query_50k.xcx",
            "node": "node/query_50k.js",
            "bun": "bun/query_50k.js",
            "deno": "deno/query_50k.js",
            "php": "php/query_50k.php",
            "python": "python/query_50k.py",
            "ruby": "ruby/query_50k.rb",
        },
        {
            "name": "Heavy Ops 1K",
            "xcx": "xcx/heavy_ops_1k.xcx",
            "node": "node/heavy_ops_1k.js",
            "bun": "bun/heavy_ops_1k.js",
            "deno": "deno/heavy_ops_1k.js",
            "php": "php/heavy_ops_1k.php",
            "python": "python/heavy_ops_1k.py",
            "ruby": "ruby/heavy_ops_1k.rb",
        },
        {
            "name": "Heavy Ops 10K",
            "xcx": "xcx/heavy_ops_10k.xcx",
            "node": "node/heavy_ops_10k.js",
            "bun": "bun/heavy_ops_10k.js",
            "deno": "deno/heavy_ops_10k.js",
            "php": "php/heavy_ops_10k.php",
            "python": "python/heavy_ops_10k.py",
            "ruby": "ruby/heavy_ops_10k.rb",
        },
        {
            "name": "Heavy Ops 50K",
            "xcx": "xcx/heavy_ops_50k.xcx",
            "node": "node/heavy_ops_50k.js",
            "bun": "bun/heavy_ops_50k.js",
            "deno": "deno/heavy_ops_50k.js",
            "php": "php/heavy_ops_50k.php",
            "python": "python/heavy_ops_50k.py",
            "ruby": "ruby/heavy_ops_50k.rb",
        },
        {
            "name": "Heavy Ops 100K",
            "xcx": "xcx/heavy_ops_100k.xcx",
            "node": "node/heavy_ops_100k.js",
            "bun": "bun/heavy_ops_100k.js",
            "deno": "deno/heavy_ops_100k.js",
            "php": "php/heavy_ops_100k.php",
            "python": "python/heavy_ops_100k.py",
            "ruby": "ruby/heavy_ops_100k.rb",
        },
    ]

    print("\nStarting cross-runtime comparative profiling...\n")
    print(f"| {'Scenario':<24} | {'Runtime':<12} | {'Min RSS (MB)':<12} | {'Max RSS (MB)':<12} | {'Avg RSS (MB)':<12} | {'Max VMS (MB)':<12} | {'Duration (ms)':<14} |")
    print(f"|{'-'*26}|{'-'*14}|{'-'*14}|{'-'*14}|{'-'*14}|{'-'*14}|{'-'*16}|")

    results_data = []

    for sc in scenarios:
        targets = []
        targets.append(("XCX (JIT)", [BIN_PATH, os.path.join("json_memory_tests", sc["xcx"])], True))
        targets.append(("XCX (Interpreter)", [BIN_PATH, os.path.join("json_memory_tests", sc["xcx"]), "--no-jit"], True))
        targets.append(("Node.js", ["node", os.path.join("json_memory_tests", sc["node"])], has_node))
        targets.append(("Bun", ["bun", os.path.join("json_memory_tests", sc["bun"])], has_bun))
        targets.append(("Deno", ["deno", "run", "--allow-read", "--allow-write", os.path.join("json_memory_tests", sc["deno"])], has_deno))
        targets.append(("Python", [py_cmd, os.path.join("json_memory_tests", sc["python"])], has_python))
        targets.append(("Ruby", ["ruby", os.path.join("json_memory_tests", sc["ruby"])], has_ruby))
        targets.append(("PHP", ["php", "-d", "memory_limit=-1", os.path.join("json_memory_tests", sc["php"])], has_php))

        for runtime_name, cmd, available in targets:
            if not available:
                print(f"| {sc['name']:<24} | {runtime_name:<12} | {'N/A':<12} | {'N/A':<12} | {'N/A':<12} | {'N/A':<12} | {'N/A':<14} |")
                continue
                
            res = profile_run(cmd)
            if res:
                print(f"| {sc['name']:<24} | {runtime_name:<12} | {res['min_rss']:<12.2f} | {res['max_rss']:<12.2f} | {res['avg_rss']:<12.2f} | {res['max_vms']:<12.2f} | {res['elapsed_ms']:<14.2f} |")
                results_data.append({
                    "scenario": sc["name"],
                    "runtime": runtime_name,
                    "max_rss": res["max_rss"],
                    "elapsed_ms": res["elapsed_ms"]
                })
            else:
                print(f"| {sc['name']:<24} | {runtime_name:<12} | {'Error':<12} | {'Error':<12} | {'Error':<12} | {'Error':<12} | {'Error':<14} |")

    # Generate charts
    dirs_to_save = ["json_memory_tests"]
    artifact_dir = r"C:\Users\s\.gemini\antigravity\brain\1b16d76f-567c-47b5-84b9-b513c9c3d5db"
    if os.path.exists(artifact_dir):
        dirs_to_save.append(artifact_dir)
        
    for d in dirs_to_save:
        try:
            generate_svg_chart(results_data, os.path.join(d, "heavy_ops_memory.svg"), "JSON Memory Scaling: Heavy Ops (Max RSS)", "Max RSS (MB)", "max_rss")
            generate_svg_chart(results_data, os.path.join(d, "heavy_ops_duration.svg"), "JSON Execution Performance: Heavy Ops (Duration)", "Duration (ms)", "elapsed_ms")
            generate_bar_chart(results_data, os.path.join(d, "scenarios_bar_memory.svg"), "JSON Memory Comparison: Scenarios A, B, C (Max RSS)", "Max RSS (MB)", "max_rss")
            generate_bar_chart(results_data, os.path.join(d, "scenarios_bar_duration.svg"), "JSON Execution Comparison: Scenarios A, B, C (Duration)", "Duration (ms)", "elapsed_ms")
            # Generate combined dashboard
            generate_combined_dashboard(results_data, os.path.join(d, "combined_dashboard.svg"))
        except Exception as e:
            print(f"Failed to generate plots in {d}: {e}", file=sys.stderr)

if __name__ == "__main__":
    main()

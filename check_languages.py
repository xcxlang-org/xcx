import os
import shutil
import subprocess

# List of target languages/environments and their corresponding executable names
LANGUAGES = {
    "R": ["Rscript", "R"],
    "Bun": ["bun"],
    "C": ["gcc", "clang"],
    "C++": ["g++", "clang++"],
    "Crystal": ["crystal"],
    "C#": ["dotnet", "csc"],
    "Erlang": ["escript", "erl"],
    "Go": ["go"],
    "Java": ["java", "javac"],
    "Lua": ["lua"],
    "LuaJIT": ["luajit"],
    "Nim": ["nim"],
    "Node.js": ["node"],
    "Perl": ["perl"],
    "PHP": ["php"],
    "PyPy": ["pypy3", "pypy"],
    "Python": ["python", "python3", "py"],
    "Ruby": ["ruby"],
    "Rust": ["rustc", "cargo"],
    "V": ["v"],
    "Zig": ["zig"],
}

# Directories to search if the executable is not in PATH
SEARCH_ROOTS = [
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\Users\s\AppData\Local\Programs",
    r"C:\Users\s\scoop",
    r"C:\ProgramData\chocolatey",
]

def find_executable_locally(names):
    # 1. Check in PATH
    for name in names:
        path = shutil.which(name)
        if path:
            return "PATH", path

    # 2. Check in common directories
    # We will search file tree up to depth 4 or 5 under target directories
    for root in SEARCH_ROOTS:
        if not os.path.exists(root):
            continue
        try:
            for dirpath, dirnames, filenames in os.walk(root):
                # Limit depth
                depth = dirpath.count(os.sep) - root.count(os.sep)
                if depth > 4:
                    # Do not descend too deep
                    del dirnames[:]
                    continue
                for name in names:
                    exe_name = f"{name}.exe"
                    if exe_name in filenames:
                        full_path = os.path.join(dirpath, exe_name)
                        return "Disk", full_path
        except Exception:
            pass
            
    return "None", None

def get_version(path):
    try:
        # Try generic version check command
        res = subprocess.run([path, "--version"], capture_output=True, text=True, timeout=2)
        if res.returncode == 0:
            return res.stdout.strip().split("\n")[0]
    except Exception:
        pass
    try:
        res = subprocess.run([path, "-v"], capture_output=True, text=True, timeout=2)
        if res.returncode == 0:
            return res.stdout.strip().split("\n")[0]
    except Exception:
        pass
    return "Executable found but version query timed out/failed."

def main():
    print("Checking toolchain installations on this Windows system...")
    print("=" * 90)
    print(f"{'Language':<12} | {'Executables':<18} | {'Status':<10} | {'Path':<46}")
    print("-" * 90)
    
    for lang, execs in LANGUAGES.items():
        loc_type, path = find_executable_locally(execs)
        if loc_type == "PATH":
            status_str = "In PATH"
            path_str = path
        elif loc_type == "Disk":
            status_str = "On Disk"
            path_str = path
        else:
            status_str = "Not Found"
            path_str = "-"
            
        execs_str = ", ".join(execs)
        print(f"{lang:<12} | {execs_str:<18} | {status_str:<10} | {path_str:<46}")
        
if __name__ == "__main__":
    main()

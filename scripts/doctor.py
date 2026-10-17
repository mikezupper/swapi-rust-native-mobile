#!/usr/bin/env python3
"""Report development capabilities without installing or changing anything."""

import argparse
import json
import os
import platform
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def command_output(args):
    try:
        result = subprocess.run(args, capture_output=True, text=True, timeout=15)
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"error": str(error)}
    output = (result.stdout or result.stderr).strip()
    return {"exit_code": result.returncode, "output": output[:2000]}


def tool(name, args=None):
    path = shutil.which(name)
    result = {"on_path": path is not None, "path": path}
    if path and args is not None:
        result["version"] = command_output([path, *args])
    return result


def android_sdk():
    candidates = [
        os.environ.get("ANDROID_HOME"),
        os.environ.get("ANDROID_SDK_ROOT"),
        str(Path.home() / "Android/Sdk"),
        str(Path.home() / "Library/Android/sdk"),
        "/opt/android-sdk",
        "/usr/lib/android-sdk",
    ]
    roots = list(dict.fromkeys(str(Path(p).expanduser()) for p in candidates if p))
    found = []
    for root in roots:
        path = Path(root)
        if not path.is_dir():
            continue
        executables = [path / "platform-tools/adb", path / "emulator/emulator"]
        executables.extend(path.glob("cmdline-tools/*/bin/sdkmanager"))
        found.append({
            "root": root,
            "executables": [str(p) for p in executables if os.access(p, os.X_OK)],
            "ndk_versions": sorted(p.name for p in (path / "ndk").glob("*") if p.is_dir()),
        })
    return {"searched": roots, "found": found}


def linux_kvm():
    import fcntl

    cpuinfo = Path("/proc/cpuinfo").read_text()
    flags = set()
    for line in cpuinfo.splitlines():
        if line.startswith("flags"):
            flags.update(line.split(":", 1)[1].split())
    result = {
        "cpu_virtualization": bool({"vmx", "svm"} & flags),
        "sse4_1": "sse4_1" in flags,
        "avx2": "avx2" in flags,
        "device_exists": Path("/dev/kvm").exists(),
    }
    try:
        fd = os.open("/dev/kvm", os.O_RDWR)
        try:
            # KVM_GET_API_VERSION queries availability; it does not create a VM.
            result["api_version"] = fcntl.ioctl(fd, 0xAE00, 0)
        finally:
            os.close(fd)
    except OSError as error:
        result["error"] = str(error)
    result["accessible"] = result.get("api_version") == 12
    return result


def report():
    system = platform.system()
    disk = shutil.disk_usage(ROOT)
    result = {
        "system": system,
        "architecture": platform.machine(),
        "disk_free_gib": round(disk.free / 1024**3, 1),
        "tools": {name: tool(name, args) for name, args in [
            ("python3", ["--version"]),
            ("git", ["--version"]),
            ("bd", ["--version"]),
            ("rustc", ["--version"]),
            ("cargo", ["--version"]),
            ("java", ["-version"]),
            ("javac", ["-version"]),
            ("adb", None),
            ("emulator", None),
            ("sdkmanager", None),
            ("qemu-system-x86_64", ["--version"]),
            ("qemu-img", ["--version"]),
            ("ssh", ["-V"]),
        ]},
        "android_sdk": android_sdk(),
        "scope": "Read-only inventory; tool presence is not a successful platform build.",
    }
    if shutil.which("rustup"):
        result["rust_targets"] = command_output(["rustup", "target", "list", "--installed"])
    if system == "Linux":
        result["kvm"] = linux_kvm()
        meminfo = {}
        for line in Path("/proc/meminfo").read_text().splitlines():
            key, value = line.split(":", 1)
            if key in {"MemTotal", "MemAvailable"}:
                meminfo[key] = round(int(value.split()[0]) / 1024**2, 1)
        result["memory_gib"] = meminfo
    elif system == "Darwin":
        result["tools"]["xcodebuild"] = tool("xcodebuild", ["-version"])
        result["tools"]["xcrun"] = tool("xcrun")
        if shutil.which("xcrun"):
            result["simctl"] = command_output(["xcrun", "--find", "simctl"])
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="Print structured output")
    args = parser.parse_args()
    data = report()
    if args.json:
        print(json.dumps(data, indent=2))
        return
    print(f"Host: {data['system']} {data['architecture']}")
    print(f"Free disk: {data['disk_free_gib']} GiB")
    if "memory_gib" in data:
        memory = data["memory_gib"]
        print(f"Memory: {memory.get('MemTotal')} GiB total, {memory.get('MemAvailable')} GiB available")
    for name, entry in data["tools"].items():
        summary = entry["path"] if entry["on_path"] else "not found on PATH"
        print(f"{name}: {summary}")
        version = entry.get("version", {})
        if version.get("error") or version.get("exit_code", 0) != 0:
            print(f"  Version check unavailable: {version}")
    roots = data["android_sdk"]["found"]
    print("Android SDK: " + (", ".join(r["root"] for r in roots) or "not found in checked locations"))
    if "kvm" in data:
        print("KVM: " + json.dumps(data["kvm"], sort_keys=True))
    print(data["scope"])


if __name__ == "__main__":
    main()

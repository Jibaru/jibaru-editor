#!/usr/bin/env python3
"""End-to-end smoke test of packaged `jb` and `jb-mcp` binaries.

    python3 scripts/smoke/smoke.py <bin-dir> --ffmpeg <ffmpeg> [--ffprobe <ffprobe>]

<bin-dir> holds `jb` and `jb-mcp` (with their bundled FFmpeg libraries, as
in a CLI package). The ffmpeg tool is only used to generate test media and
check the output; the binaries under test run with FFmpeg removed from PATH,
so they must find the bundled libraries on their own.

Checks: decode (import + render of a generated clip with audio), GPU render
(any adapter, including WARP/llvmpipe), encode (the output is probed), a
single frame render, and an MCP session over stdio (initialize, tools/list,
new_project, execute_command, get_project).
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

EXE = ".exe" if os.name == "nt" else ""


def clean_env(ffmpeg_dirs):
    """Environment with every directory containing FFmpeg removed from PATH."""
    env = dict(os.environ)
    keep = []
    for p in env.get("PATH", "").split(os.pathsep):
        if not p:
            continue
        rp = os.path.normcase(os.path.abspath(p))
        if any(rp == os.path.normcase(os.path.abspath(d)) for d in ffmpeg_dirs):
            continue
        try:
            if any(Path(p).glob("avcodec-*.dll")) or any(Path(p).glob("libavcodec.*dylib")):
                continue
        except OSError:
            pass
        keep.append(p)
    env["PATH"] = os.pathsep.join(keep)
    for var in ("DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH", "LD_LIBRARY_PATH", "FFMPEG_DIR"):
        env.pop(var, None)
    return env


def run(cmd, env=None, check=True):
    print("+", " ".join(str(c) for c in cmd), flush=True)
    p = subprocess.run([str(c) for c in cmd], env=env, capture_output=True, text=True)
    if p.stderr.strip():
        print(p.stderr.strip()[-4000:], file=sys.stderr)
    if check and p.returncode != 0:
        print(p.stdout[-4000:])
        raise SystemExit(f"command failed with exit code {p.returncode}")
    return p


def jb(bin_dir, env, *args):
    p = run([bin_dir / f"jb{EXE}", "--json", *args], env=env, check=False)
    try:
        out = json.loads(p.stdout)
    except json.JSONDecodeError:
        raise SystemExit(f"jb {args[0]}: non-JSON output (exit {p.returncode}):\n{p.stdout}")
    if not out.get("ok"):
        raise SystemExit(f"jb {args[0]} failed: {json.dumps(out, indent=2)}")
    return out["data"]


def probe(ffprobe, path):
    p = run([ffprobe, "-v", "error", "-show_streams", "-show_format", "-of", "json", path])
    return json.loads(p.stdout)


def smoke_cli(bin_dir, env, ffmpeg, ffprobe, work):
    media = work / "in.mp4"
    run([ffmpeg, "-v", "error", "-y",
         "-f", "lavfi", "-i", "testsrc2=size=320x180:rate=30",
         "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000",
         "-t", "2", "-c:v", "mpeg4", "-q:v", "4", "-c:a", "aac", "-pix_fmt", "yuv420p", media])

    project = work / "smoke.jbp"
    jb(bin_dir, env, "new", project, "--width", "640", "--height", "360", "--fps", "30")
    added = jb(bin_dir, env, "import-media", project, media)["added"]
    assert added and added[0]["width"] == 320, added
    media_id = added[0]["id"]
    video = jb(bin_dir, env, "exec", project, '{"op":"add_track","kind":"video"}')["created"]["tracks"][0]
    audio = jb(bin_dir, env, "exec", project, '{"op":"add_track","kind":"audio"}')["created"]["tracks"][0]
    for track in (video, audio):
        clip = {"op": "add_clip", "track_id": track, "clip": {
            "name": "clip", "source": {"type": "media", "media_id": media_id}, "start": 0, "end": 20_000_000}}
        jb(bin_dir, env, "exec", project, json.dumps(clip))

    out = work / "out.mp4"
    result = jb(bin_dir, env, "render", project, "-o", out)
    print("render:", json.dumps(result)[:2000])
    info = probe(ffprobe, out)
    kinds = sorted(s["codec_type"] for s in info["streams"])
    duration = float(info["format"]["duration"])
    print(f"output: {kinds}, {duration:.2f}s, {out.stat().st_size} bytes")
    assert "video" in kinds, info
    assert 1.8 <= duration <= 2.3, duration
    v = next(s for s in info["streams"] if s["codec_type"] == "video")
    assert (int(v["width"]), int(v["height"])) == (640, 360), v

    # The rendered picture must not be blank: testsrc2 is colorful.
    frame = work / "frame.png"
    jb(bin_dir, env, "frame", project, "--at", "1", "-o", frame)
    assert frame.stat().st_size > 2000, frame.stat().st_size


class Mcp:
    def __init__(self, exe, env):
        self.p = subprocess.Popen([str(exe)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env, text=True)
        self.next_id = 1

    def send(self, msg):
        self.p.stdin.write(json.dumps(msg) + "\n")
        self.p.stdin.flush()

    def request(self, method, params):
        rid = self.next_id
        self.next_id += 1
        self.send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise SystemExit("jb-mcp closed stdout")
            msg = json.loads(line)
            if msg.get("id") == rid:
                if "error" in msg:
                    raise SystemExit(f"JSON-RPC error: {msg}")
                return msg["result"]

    def call(self, tool, args):
        r = self.request("tools/call", {"name": tool, "arguments": args})
        if r.get("isError"):
            raise SystemExit(f"{tool} failed: {r}")
        return r.get("structuredContent", {})

    def close(self):
        self.p.stdin.close()
        try:
            self.p.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.p.kill()


def smoke_mcp(bin_dir, env, work):
    c = Mcp(bin_dir / f"jb-mcp{EXE}", env)
    try:
        init = c.request("initialize", {
            "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "smoke", "version": "0"}})
        assert init["serverInfo"]["name"] == "jb-mcp", init
        c.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        names = [t["name"] for t in c.request("tools/list", {})["tools"]]
        print("mcp tools:", ", ".join(names))
        for expected in ("new_project", "execute_command", "get_project", "render"):
            assert expected in names, expected
        c.call("new_project", {"path": str(work / "mcp.jbp"), "width": 320, "height": 180, "fps": "25"})
        track = c.call("execute_command", {"command": {"op": "add_track", "kind": "video"}})["created"]["tracks"][0]
        c.call("execute_command", {"command": {"op": "add_clip", "track_id": track, "clip": {
            "name": "blue", "source": {"type": "color", "color": [0, 0, 1, 1]}, "start": 0, "end": 10_000_000}}})
        c.call("get_project", {})
    finally:
        c.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("bin_dir", type=Path)
    ap.add_argument("--ffmpeg", required=True)
    ap.add_argument("--ffprobe")
    a = ap.parse_args()
    ffmpeg = shutil.which(a.ffmpeg) or a.ffmpeg
    ffprobe = a.ffprobe or str(Path(ffmpeg).with_name(f"ffprobe{EXE}"))
    env = clean_env([Path(ffmpeg).parent])
    bin_dir = a.bin_dir.resolve()
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        smoke_cli(bin_dir, env, ffmpeg, ffprobe, work)
        smoke_mcp(bin_dir, env, work)
    print("smoke test passed")


if __name__ == "__main__":
    main()

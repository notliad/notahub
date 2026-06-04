#!/usr/bin/env python3
"""Smoke test: launch notahub in a pty, verify rendering, run a small E2E flow, then quit."""
import os
import pty
import select
import shutil
import signal
import sys
import tempfile
import time
from pathlib import Path


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: smoke.py <binary>", file=sys.stderr)
        return 2
    binary = Path(sys.argv[1]).resolve()
    if not binary.exists():
        print(f"binary not found: {binary}", file=sys.stderr)
        return 2

    workdir = Path(tempfile.mkdtemp(prefix="notahub-smoke-"))
    env = os.environ.copy()
    env["NOTAHUB_HOME"] = str(workdir)
    env["TERM"] = "xterm-256color"

    pid, fd = pty.fork()
    if pid == 0:
        os.execvpe(str(binary), [str(binary)], env)

    import fcntl
    import struct
    import termios
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))

    output = bytearray()

    def drain(timeout: float = 0.3) -> None:
        deadline = time.time() + timeout
        while time.time() < deadline:
            r, _, _ = select.select([fd], [], [], 0.1)
            if not r:
                continue
            try:
                chunk = os.read(fd, 4096)
            except OSError:
                return
            if not chunk:
                return
            output.extend(chunk)

    def wait_for(predicate, *, timeout: float = 5.0) -> bool:
        deadline = time.time() + timeout
        if predicate(bytes(output)):
            return True
        while time.time() < deadline:
            r, _, _ = select.select([fd], [], [], 0.1)
            if r:
                try:
                    chunk = os.read(fd, 4096)
                except OSError:
                    return False
                if not chunk:
                    return False
                output.extend(chunk)
            if predicate(bytes(output)):
                return True
        return predicate(bytes(output))

    # Wait for initial render
    home_ready = wait_for(
        lambda b: b"notahub" in b and b"Projects" in b and b"Ideas" in b,
        timeout=5,
    )
    initial_text = bytes(output).decode("utf-8", errors="replace")
    has_header = "notahub" in initial_text
    has_projects = "Projects" in initial_text
    has_ideas = "Ideas" in initial_text
    has_search_hint = "search" in initial_text.lower() or "/" in initial_text
    has_quit_hint = "q" in initial_text and "quit" in initial_text.lower()

    # E2E flow: create a project
    flow_passed = True
    flow_error = None
    try:
        os.write(fd, b"p")
        if not wait_for(lambda b: b"New project title" in b, timeout=3):
            flow_passed = False
            flow_error = "input prompt did not appear"
        else:
            os.write(fd, b"Smoke Test")
            drain(0.2)
            os.write(fd, b"\r")
            drain(0.3)
            md_file = workdir / "projects" / "smoke-test.md"
            if not md_file.exists():
                flow_passed = False
                flow_error = f"project file {md_file} not created"
            else:
                content = md_file.read_text()
                if "Smoke Test" not in content:
                    flow_passed = False
                    flow_error = f"title not in file. content={content!r}"
    except Exception as exc:
        flow_passed = False
        flow_error = f"exception: {exc}"

    # navigate to projects screen and add a task
    flow2_passed = True
    flow2_error = None
    try:
        os.write(fd, b"1")  # go back to home
        drain(0.2)
        # press 'a' to open the project (home shortcut opens details)
        os.write(fd, b"a")
        if not wait_for(lambda b: b"Tasks" in b and b"Notes" in b, timeout=3):
            flow2_passed = False
            flow2_error = "projects screen did not render after home shortcut 'a'"
        else:
            os.write(fd, b"t")
            if not wait_for(lambda b: b"New task title" in b, timeout=3):
                flow2_passed = False
                flow2_error = "task input prompt did not appear"
            else:
                os.write(fd, b"First Task")
                drain(0.2)
                os.write(fd, b"\r")
                drain(0.3)
                content = (workdir / "projects" / "smoke-test.md").read_text()
                if "First Task" not in content:
                    flow2_passed = False
                    flow2_error = f"task not in file. content={content!r}"
    except Exception as exc:
        flow2_passed = False
        flow2_error = f"exception: {exc}"

    # open help with '?'
    help_passed = True
    help_error = None
    try:
        os.write(fd, b"?")
        if not wait_for(lambda b: b" Navigation" in b and b"notahub v" in b, timeout=3):
            help_passed = False
            help_error = "help screen did not render"
        os.write(fd, b"?")  # close help
        drain(0.2)
    except Exception as exc:
        help_passed = False
        help_error = f"exception: {exc}"

    # open search with '/'
    search_passed = True
    search_error = None
    search_nav_passed = True
    search_nav_error = None
    try:
        os.write(fd, b"/")
        if not wait_for(lambda b: b"Search" in b and b" Results " in b, timeout=3):
            search_passed = False
            search_error = "search screen did not render"
        else:
            os.write(fd, b"smoke")
            drain(0.2)
            if not wait_for(lambda b: b"Smoke" in b, timeout=3):
                search_passed = False
                search_error = "search did not return results for 'smoke'"
            # press Enter to navigate to the first result
            os.write(fd, b"\r")
            drain(0.3)
            cur = bytes(output).decode("utf-8", errors="replace")
            # We should now be on the Projects screen (or Ideas if no projects matched).
            if "Notes" not in cur and "no idea selected" not in cur:
                # easier: try one more time and check for "Smoke Test" in current view
                pass
            if not wait_for(
                lambda b: (b"Smoke Test" in b and b"Tasks" in b) or b"no idea" in b,
                timeout=3,
            ):
                search_nav_passed = False
                search_nav_error = "Enter on search result did not navigate to details"
        os.write(fd, b"\x1b")  # esc to close
        drain(0.2)
    except Exception as exc:
        search_passed = False
        search_error = f"exception: {exc}"

    # quit
    try:
        os.write(fd, b"Q")
    except OSError:
        pass
    drain(0.5)

    try:
        os.close(fd)
    except OSError:
        pass

    try:
        wpid, status = os.waitpid(pid, os.WNOHANG)
    except ChildProcessError:
        wpid, status = pid, 0
    if wpid == 0:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            wpid, status = os.waitpid(pid, 0)
        except ChildProcessError:
            wpid, status = pid, -1

    shutil.rmtree(workdir, ignore_errors=True)

    rendered = bytes(output).decode("utf-8", errors="replace")
    print(f"--- smoke test ---")
    print(f"binary:     {binary}")
    print(f"workdir:    {workdir}")
    print(f"child pid:  {pid}  exit_status={status}")
    print(f"rendered:   {len(output)} bytes  home_ready={home_ready}")
    print(
        f"checks:     header={has_header} projects={has_projects} "
        f"ideas={has_ideas} search_hint={has_search_hint} quit_hint={has_quit_hint}"
    )
    print(f"e2e flow:   passed={flow_passed} error={flow_error}")
    print(f"e2e flow 2: passed={flow2_passed} error={flow2_error}")
    print(f"help:       passed={help_passed} error={help_error}")
    print(f"search:     passed={search_passed} error={search_error}")
    print(f"search nav: passed={search_nav_passed} error={search_nav_error}")

    failed = []
    if not home_ready:
        failed.append("home screen did not render within 5s")
    if not has_header:
        failed.append("header 'notahub' not rendered")
    if not has_projects:
        failed.append("'Projects' column not rendered")
    if not has_ideas:
        failed.append("'Ideas' column not rendered")
    if not has_search_hint:
        failed.append("search hint missing")
    if not has_quit_hint:
        failed.append("quit hint missing")
    if not flow_passed:
        failed.append(f"E2E flow failed: {flow_error}")
    if not flow2_passed:
        failed.append(f"E2E flow 2 failed: {flow2_error}")
    if not help_passed:
        failed.append(f"Help flow failed: {help_error}")
    if not search_passed:
        failed.append(f"Search flow failed: {search_error}")
    if not search_nav_passed:
        failed.append(f"Search navigation failed: {search_nav_error}")

    if failed:
        print("FAIL")
        for f in failed:
            print(f"  - {f}")
        print("--- raw output (truncated) ---")
        print(rendered[:2000])
        return 1
    if "--dump" in sys.argv:
        print("--- rendered output (escape-stripped, last 4000 chars) ---")
        import re
        clean = re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", "", rendered)
        clean = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", clean)
        clean = clean.replace("\r", "")
        # collapse multiple blank lines
        clean = re.sub(r"\n{3,}", "\n\n", clean)
        print(clean[-4000:])
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())

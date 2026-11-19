#!/usr/bin/env python3
"""Run a local mixed HTTP workload against DB_MAX_CONNECTIONS=1,2,4,10.

Requires a built target/debug/braindrop-cli and Python 3. The app is launched
with `start --all`, so its queue workers and scheduler run during each sample.
This is a repeatable smoke benchmark, not a portable performance claim.
"""

import concurrent.futures
import json
import os
import random
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request


ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BINARY = os.path.join(ROOT, "target", "debug", "braindrop-cli")
PASSWORD = "benchmark-long-password"
EMAIL = "pool-benchmark@example.com"
DURATION_SECONDS = int(os.environ.get("BENCHMARK_SECONDS", "20"))
CONCURRENCY = int(os.environ.get("BENCHMARK_CONCURRENCY", "12"))


def request(base, path, method="GET", token=None, payload=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    data = json.dumps(payload).encode() if payload is not None else None
    req = urllib.request.Request(base + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()


def wait_ready(base, process):
    deadline = time.time() + 90
    while time.time() < deadline:
        if process.poll() is not None:
            raise RuntimeError("BrainDrop exited before becoming ready")
        try:
            status, _ = request(base, "/_health")
            if status == 200:
                return
        except (OSError, TimeoutError):
            time.sleep(0.5)
    raise TimeoutError("BrainDrop did not become healthy within 90 seconds")


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int((len(ordered) - 1) * fraction))]


def run_pool(pool_size):
    with tempfile.TemporaryDirectory(prefix=f"braindrop-pool-{pool_size}-") as tmp:
        db = os.path.join(tmp, "app.sqlite")
        queue = os.path.join(tmp, "queue.sqlite")
        log_path = os.path.join(tmp, "app.log")
        port = 5510 + pool_size
        env = os.environ.copy()
        env.update(
            {
                "DATABASE_URL": f"sqlite://{db}?mode=rwc",
                "QUEUE_URL": f"sqlite://{queue}?mode=rwc",
                "DB_MAX_CONNECTIONS": str(pool_size),
                "DB_MIN_CONNECTIONS": "1",
                "PORT": str(port),
                "BINDING": "127.0.0.1",
                "LOCO_ENV": "production",
                "HOST": f"http://127.0.0.1:{port}",
                "MAILER_ENABLED": "false",
                "JWT_SECRET": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
                "RUST_LOG": "warn,loco=error,sqlx=warn",
            }
        )
        subprocess.run(
            [BINARY, "db", "migrate"],
            cwd=ROOT,
            env=env,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        subprocess.run(
            [
                BINARY,
                "task",
                "admin:create",
                f"email:{EMAIL}",
                "name:PoolBenchmark",
                f"password:{PASSWORD}",
            ],
            cwd=ROOT,
            env=env,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        with open(log_path, "w", encoding="utf-8") as log:
            process = subprocess.Popen(
                [BINARY, "start", "--all"], cwd=ROOT, env=env, stdout=log, stderr=log
            )
            base = f"http://127.0.0.1:{port}"
            try:
                wait_ready(base, process)
                status, body = request(
                    base,
                    "/api/auth/login",
                    "POST",
                    payload={"email": EMAIL, "password": PASSWORD},
                )
                if status != 200:
                    raise RuntimeError(f"login failed: {status} {body[:200]!r}")
                token = json.loads(body)["token"]

                for batch in range(5):
                    items = [
                        {"title": f"Pool {pool_size} seeded item {batch * 50 + index}"}
                        for index in range(50)
                    ]
                    status, body = request(
                        base, "/api/items", "POST", token, {"items": items}
                    )
                    if status != 201:
                        raise RuntimeError(f"item seeding failed: {status} {body[:200]!r}")

                latencies = []
                errors = []
                stop_at = time.monotonic() + DURATION_SECONDS
                counter = 0
                counter_lock = threading.Lock()

                def worker():
                    local = []
                    while time.monotonic() < stop_at:
                        choice = random.random()
                        if choice < 0.55:
                            method, path, payload = "GET", "/api/items?page_size=25", None
                        elif choice < 0.85:
                            method, path, payload = "GET", "/api/stats", None
                        else:
                            nonlocal counter
                            with counter_lock:
                                counter += 1
                                item_number = counter
                            method, path = "POST", "/api/items"
                            payload = {"title": f"Pool {pool_size} workload item {item_number}"}
                        started = time.perf_counter()
                        try:
                            status, body = request(base, path, method, token, payload)
                            local.append((time.perf_counter() - started, status, body[:250]))
                        except Exception as error:  # pragma: no cover - emitted in report
                            local.append((time.perf_counter() - started, 0, str(error).encode()))
                    return local

                started = time.monotonic()
                with concurrent.futures.ThreadPoolExecutor(max_workers=CONCURRENCY) as pool:
                    for result in pool.map(lambda _: worker(), range(CONCURRENCY)):
                        for latency, status, body in result:
                            latencies.append(latency)
                            if status != 200 and status != 201:
                                errors.append((status, body.decode(errors="replace")))
                elapsed = time.monotonic() - started
                time.sleep(0.1)
                rss_kb = int(
                    subprocess.check_output(["ps", "-o", "rss=", "-p", str(process.pid)])
                    .decode()
                    .strip()
                )
                busy_errors = sum("busy" in body.lower() or "locked" in body.lower() for _, body in errors)
                timeouts = sum("timeout" in body.lower() or "timed out" in body.lower() for _, body in errors)
                print(
                    json.dumps(
                        {
                            "pool": pool_size,
                            "seconds": round(elapsed, 2),
                            "requests": len(latencies),
                            "throughput_rps": round(len(latencies) / elapsed, 2),
                            "p50_ms": round(percentile(latencies, 0.50) * 1000, 2),
                            "p95_ms": round(percentile(latencies, 0.95) * 1000, 2),
                            "p99_ms": round(percentile(latencies, 0.99) * 1000, 2),
                            "errors": len(errors),
                            "sqlite_busy_errors": busy_errors,
                            "connection_timeout_errors": timeouts,
                            "rss_mb": round(rss_kb / 1024, 1),
                            "worker_concurrency": 2,
                            "scheduler": "running via start --all",
                        },
                        sort_keys=True,
                    )
                )
                with open(log_path, encoding="utf-8") as log_file:
                    logs = log_file.read().lower()
                print(
                    json.dumps(
                        {
                            "pool": pool_size,
                            "logged_pool_timeouts": logs.count("pool timed out"),
                            "logged_sqlite_busy": logs.count("database is locked")
                            + logs.count("sqlite_busy"),
                        },
                        sort_keys=True,
                    )
                )
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


if __name__ == "__main__":
    if not os.path.isfile(BINARY):
        raise SystemExit("build target/debug/braindrop-cli first with cargo build")
    for size in (1, 2, 4, 10):
        run_pool(size)

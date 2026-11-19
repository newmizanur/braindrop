# BrainDrop benchmark and resource notes

## SQLite application pool comparison

This is the Task 009 local synthetic comparison, preserved without alteration.
It is a tuning comparison, not a capacity guarantee.

| App pool max | Throughput | p50 / p95 / p99 | Sampled RSS |
|---:|---:|---:|---:|
| 1 | 742.0 req/s | 15.63 / 21.92 / 24.48 ms | 38.4 MB |
| 2 | 1,202.9 req/s | 9.41 / 15.28 / 18.65 ms | 39.7 MB |
| 4 | 1,501.9 req/s | 7.41 / 12.70 / 16.58 ms | 37.0 MB |
| 10 | 1,508.7 req/s | 7.14 / 14.70 / 22.66 ms | 50.3 MB |

### Environment and method

- Source revision: Task 009 final source, `103c591969b52f93c8a5254f50edfc5ca23105e6`.
- OS/architecture and host hardware: local Task 009 benchmark host; exact hardware and OS identifiers were not recorded in the Task 009 report and are not reconstructed here.
- Build: local debug binary (`cargo build --bin braindrop-cli`), production Loco configuration.
- Script: [`scripts/benchmark-sqlite-pools.py`](../scripts/benchmark-sqlite-pools.py), 20 seconds per pool, 12 concurrent clients.
- Dataset: 250 seeded items; separate temporary app and queue SQLite files for each run.
- Workload: 55% item-list reads, 30% dashboard stats reads and 15% item writes. `start --all` ran worker concurrency 2 and scheduler during each sample.
- The worker and scheduler had no seeded queue jobs. No pre-remediation run exists.
- Per-request HTTP latency and total throughput were measured by the script. Process RSS was sampled once after workload using `ps`; it is sampled process RSS, not container/cgroup usage or peak RSS.
- SQLite busy errors and pool acquisition timeout errors/log messages were zero in the samples. Connection acquisition latency was not separately instrumented.
- There were no HTTP request errors. The benchmark does not provide an isolated CPU profile, peak memory, or representative production capacity estimate.

The default app pool max of 4 is supported by this comparison: max 10 added 0.5%
throughput over max 4 while increasing sampled RSS and worsening p95/p99. The
validated configuration is app min/max 1/4, queue max 2, two queue workers,
WAL, `synchronous=NORMAL`, 5000 ms busy timeout, foreign keys enabled and one
application replica.

Reproduce the local run from the repository root:

```sh
cargo build --bin braindrop-cli
python3 scripts/benchmark-sqlite-pools.py
```

`BENCHMARK_SECONDS` and `BENCHMARK_CONCURRENCY` can override duration and client
count. The script creates temporary databases and an administrator account; it
does not use external AI or notification providers.

## Deployment measurements

### Local constrained Docker check (2026-11-19)

The production image was run locally under Docker Desktop on a Darwin ARM64
development host, with one CPU, temporary bind-mounted app/queue SQLite files,
and Loco server, worker and scheduler running through `start --all`. Each
sample logged in an administrator, created 40 items and read the item list 40
times. This is a tiny deterministic smoke workload, not a sustained benchmark.

| Container memory limit | Sampled Docker memory | OOM killed | Authenticated API p50 / p95 | Requests |
|---:|---:|---:|---:|---:|
| 128 MiB | 12.69 MiB | No | 2.10 / 3.96 ms | 40 creates + 40 reads, all passed |
| 256 MiB | 12.70 MiB | No | 2.15 / 4.59 ms | 40 creates + 40 reads, all passed |

Both samples served `/_health`, ran as uid 10001, and had `/data` owned by
`10001:999`; the 128 MiB run's app+queue database files totaled about 152 KiB.
Memory was sampled once using `docker stats`; it is container memory at that
instant, not process RSS, peak memory, host usage or cgroup high-water mark.
These results show the small workload can start and complete at both limits,
but do not establish 128 or 256 MiB production reliability. They had no seeded
queue jobs, sustained concurrency, memory pressure, large database or real
provider work. There is no comparable before/after result.

### Remote deployments

No TierHive or Fly.io deployment measurements have been recorded. The remote
TierHive host has not been inspected, and no Fly app, machine or volume has
been provisioned. There are no verified remote server specifications, native
binary/archive or Docker image measurements, startup time, idle/active/peak
memory, CPU use, API latency, database size, or worker/scheduler overhead.
Native process RSS, container memory and total host memory must be measured and
reported separately when those deployments are authorized.

The prior pool test is not evidence that BrainDrop is reliable at either 128 MiB
or 256 MiB. In particular, it did not measure production container overhead or
active notification/provider workloads. A separate deployment benchmark should
use mock providers, representative authenticated reads/writes and queued jobs,
and report the full environment and measurement method before making a memory
budget claim.

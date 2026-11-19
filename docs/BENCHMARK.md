# BrainDrop benchmark and resource notes

## Remote deployment samples (2026-11-19 to 2026-11-20)

These observations are separate from the Task 009 local pool comparison below
and are point-in-time checks, not capacity tests.

### Fly.io demo machine

- Source/config: Task 010 local revision with `AI_ENABLED=true` in `fly.toml`
  (BYOK enabled, no shared OpenRouter key); production Docker image built by
  Fly from the existing Dockerfile.
- Region/shape: Singapore, one shared CPU, 256 MiB, one encrypted 1 GiB
  volume. Image size reported by Fly: 37 MB.
- Machine creation to healthy: about 19 seconds, based on Fly machine
  timestamps. A forced stop followed by an HTTPS health request cold-started
  successfully; the request returned 200 in 2.25 seconds.
- Health and frontend root returned HTTP 200. Registration returned the
  expected 403 `registration_disabled` response.
- Idle process RSS sample: 23,244 kB for `braindrop-cli`, UID 10001. This is
  process RSS, not machine/cgroup memory. No active authenticated workload was
  available in this earlier idle sample. A later transient-account workload
  (described below) verified authenticated item operations and encrypted BYOK
  settings without retaining test data.
- App DB: 122,880 bytes; queue DB: 28,672 bytes after migrations. Both passed
  SQLite `PRAGMA integrity_check` after restart/cold start. This validates
  mount persistence. Online backup and restore scripts passed against
  disposable copies (4.3 KiB archive); no user data or pending notification
  jobs were included, so this does not validate delivery-boundary recovery.
- CPU utilization, machine-level peak memory, authenticated endpoint latency,
  sustained workload, queue throughput, and actual auto-stop idle duration
  were not measured. Native Fly idle detection was verified by configuration;
  auto-start was verified through an explicit stop/cold-start test.

### TierHive Alpine VPS

The first read-only snapshot on 2026-11-20 was taken before the flat-layout
redeployment, against the owner-repaired old release tree. The host ran Alpine
3.24.4 x86_64, with 95,468 KiB RAM and 131,068 KiB swap. At the sample, 45,692
KiB RAM and 130,072 KiB swap were free/available. The non-root app process
(UID 103) had 22,476 KiB RSS, 24,840 KiB process high-water RSS since start,
50,524 KiB virtual size and three threads. The `ps` implementation on this
Alpine image did not expose per-process CPU, so CPU percentage was not measured.
The app and queue databases together occupied 160 KiB by `du`; both passed
`PRAGMA integrity_check`.

Three local health requests returned 200 in 0.7–14.2 ms; the frontend returned
200 in 1.2 ms and a client-side deep link returned 200 in 0.7 ms. These are
single-request samples, not authenticated latency or a performance test.
The post-deployment sample later that day is described below. Startup duration,
CPU, worker/scheduler overhead under load and peak-under-load memory remain
unmeasured. The observed database and idle memory do not establish reliable
capacity on this approximately 93 MiB host.

The Task 009 local Docker samples (128/256 MiB limits) and historical pool
benchmark below do not establish reliable production capacity under concurrent
writes or background jobs.

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

#### TierHive post-deployment sample (2026-11-20)

After deploying the flat runtime and restarting through OpenRC, the process
ran as UID 103 with 17,492 KiB RSS and 18,236 KiB process high-water RSS since
start. The host reported 50,228 KiB available RAM and 130,072 KiB free swap
(approximately 93.2 MiB RAM and 128 MiB swap total). The app and queue data
directory occupied 172 KiB on disk at the sample; both databases passed
`PRAGMA integrity_check`. Local health returned HTTP 200. This measurement was
idle; authenticated workload and per-process CPU were not sampled on this host.
The BusyBox `ps` lacks a CPU percentage field. No full reboot was performed.

#### Fly post-deployment authenticated workload (2026-11-20)

Using a random temporary administrator and password held only in the test
process, 10 authenticated item creates and 10 list reads succeeded. A machine
restart then passed; login, item retrieval and encrypted personal-key settings
persisted. The temporary account, items and test key were removed. The fake
key was not sent to an external provider and was not present in API responses.
Over 10 requests per operation, creates had p50/p95 latency of 199.01/276.39
ms and reads 195.98/205.93 ms. These measurements include public HTTPS and
request overhead and are small-sample observations, not capacity results.

The app process ran as UID 10001 with 23,984 KiB RSS and 41,212 KiB process
high-water RSS since start (VmSize 573,052 KiB). `/data` occupied 232 KiB
including the 16 KiB `lost+found` directory; both SQLite files passed integrity
checks. Machine shape remained one shared CPU and 256 MiB, one machine and one
1 GiB encrypted volume. Cgroup peak memory and CPU utilization were not
captured. Worker/scheduler startup is present in filtered logs; notification
recovery with a seeded pending remote job was not exercised.

Fly active and TierHive idle samples cannot establish reliable production
capacity within 128 or 256 MiB. The remote samples are short and omit sustained
concurrency, large data, queued delivery work and provider execution.

The prior pool test is not evidence that BrainDrop is reliable at either 128 MiB
or 256 MiB. In particular, it did not measure production container overhead or
active notification/provider workloads. A separate deployment benchmark should
use mock providers, representative authenticated reads/writes and queued jobs,
and report the full environment and measurement method before making a memory
budget claim.

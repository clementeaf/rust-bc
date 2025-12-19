---
name: Performance Issue
about: Report a performance degradation
title: "[Performance] "
labels: type/performance
assignees: ''

---

## Metric
What metric degraded?
- [ ] Latency (API response time)
- [ ] Throughput (TPS, transactions/sec)
- [ ] Memory usage
- [ ] CPU usage
- [ ] Disk I/O
- [ ] Other: ___

## Component
- [ ] storage
- [ ] consensus
- [ ] identity
- [ ] api
- [ ] persistence
- [ ] services

## Current Performance
```
Metric: X
Value: Y units
Measured: YYYY-MM-DD
```

## Target Performance
```
Expected: Z units
SLA: < Z units
```

## Regression Details
- **Since:** Commit hash / PR #XXX / Version v0.1.0
- **Frequency:** Always / Intermittent / Under load
- **Load:** X requests/sec, Y concurrent users, Z dataset size

## Profiling Data
```
Attach profiling results (flamegraph, perf output, etc.)
```

## Environment
- **OS:** macOS / Linux
- **CPU:** (cores, type)
- **Memory:** (GB)
- **Rust version:** 1.75.0
- **.NET version:** 8.0.0

## Reproduction Steps
1. Setup: ...
2. Run: `command`
3. Observe: ...

## Possible Cause
Any theories about the root cause?

## Screenshots
Include graphs/charts if available.

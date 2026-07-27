# Next-Core Benchmark Report

- Generated: 2026-07-27 11:58:13 +08:00
- Commit: `376c280`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=3 render_frame_revision=16 render_frame_lines=30 render_delta_lines=0 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| key-to-screen p95 | 5962 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5542 us | 16000 us | ok |
| dual-agent echo p95 | 5632 us | 33000 us | ok |
| agent startup input p95 | 20 us | 33000 us | ok |
| paste 10kb elapsed | 21 ms | 50 ms | ok |
| scrollback page p95 | 73 us | 1000 us | ok |
| viewport scroll p95 | 75 us | 1000 us | ok |
| viewport scroll under flood p95 | 374 us | 50000 us | ok |
| screen read under flood p95 | 141 us | 50000 us | ok |
| render frame p95 | 0 us | 1000 us | ok |
| render dirty frame p95 | 529 us | 1000 us | ok |
| focus switch p95 | 363 us | 100000 us | ok |
| session create p95 | 48127 us | 100000 us | ok |
| session ready p95 | 74435 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=25 bytes_per_sec=4477611.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=70844 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=70844 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5128 p50_us=5638 p95_us=5962 max_us=15687
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=30740 foreground_cwd=none root=cmd.exe root_pid=30740 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=364 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=105 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3097 min_us=1 p50_us=1 p95_us=1 max_us=110
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=29012 foreground_cwd=none root=cmd.exe root_pid=29012 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5040 p50_us=5292 p95_us=5542 max_us=15636
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=38476 foreground_cwd=none root=cmd.exe root_pid=38476 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=360 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0041
UNTERM_NEXT_CORE_BENCH_0041
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0042
UNTERM_NEXT_CORE_BENCH_0042
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0043
UNTERM_NEXT_CORE_BENCH_0043
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0044
UNTERM_NEXT_CORE_BENCH_0044
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0045
UNTERM_NEXT_CORE_BENCH_0045
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0046
UNTERM_NEXT_CORE_BENCH_0046
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0047
UNTERM_NEXT_CORE_BENCH_0047
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0048
UNTERM_NEXT_CORE_BENCH_0048
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0049
UNTERM_NEXT_CORE_BENCH_0049
```

### output flood

- Status: ok
- Args: `--bench-flood-lines 100000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=100000 bytes=1048576 elapsed_ms=24302 lines_per_sec=4114.9 bytes_per_sec=43147.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=23616 foreground_cwd=none root=cmd.exe root_pid=23616 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=174081 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=21 bytes_per_sec=480776.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=16140 foreground_cwd=none root=cmd.exe root_pid=16140 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1299 lines_per_sec=7694.9 bytes_per_sec=806870.6
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=21 min_us=56 p50_us=61 p95_us=73 max_us=114
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=6636 foreground_cwd=none root=cmd.exe root_pid=6636 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25081 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1252 lines_per_sec=7985.3 bytes_per_sec=837321.4
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=23 min_us=64 p50_us=70 p95_us=75 max_us=101
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=30936 foreground_cwd=none root=cmd.exe root_pid=30936 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25235 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=116 rows_read=3391 total_ms=676 min_us=13 p50_us=233 p95_us=374 max_us=546
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=44916 foreground_cwd=none root=cmd.exe root_pid=44916 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14571 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=235 viewport_scrolls=116
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5074 p50_us=5343 p95_us=5632 max_us=5701
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=764 combined_lines_per_sec=13084.2 combined_bytes_per_sec=1709457.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=23288 foreground_cwd=none root=cmd.exe root_pid=23288 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=153 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0011
UNTERM_NEXT_CORE_BENCH_0011
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0012
UNTERM_NEXT_CORE_BENCH_0012
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0013
UNTERM_NEXT_CORE_BENCH_0013
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0014
UNTERM_NEXT_CORE_BENCH_0014
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0015
UNTERM_NEXT_CORE_BENCH_0015
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0016
UNTERM_NEXT_CORE_BENCH_0016
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0017
UNTERM_NEXT_CORE_BENCH_0017
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0018
UNTERM_NEXT_CORE_BENCH_0018
C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0019
UNTERM_NEXT_CORE_BENCH_0019
```

### agent startup stall

- Status: ok
- Args: `--bench-agent-startup-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=126 screen_reads=126 elapsed_ms=706 input_min_us=5 input_p50_us=8 input_p95_us=20 input_max_us=45 screen_read_min_us=14 screen_read_p50_us=19 screen_read_p95_us=31 screen_read_max_us=48
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=43372 foreground_cwd=none root=cmd.exe root_pid=43372 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=127 input_bytes=383 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=129 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=114 total_ms=644 min_us=11 p50_us=96 p95_us=141 max_us=427 text_bytes=85726
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=2224 foreground_cwd=none root=cmd.exe root_pid=2224 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14534 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=117 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=458 full_lines=30 empty_deltas=1000 min_us=0 p50_us=0 p95_us=0 max_us=2 dirty_rounds=50 dirty_lines=1500 dirty_min_us=278 dirty_p50_us=382 dirty_p95_us=529 dirty_max_us=761
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
activity_process foreground=cmd.exe foreground_pid=17888 foreground_cwd=none root=cmd.exe root_pid=17888 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=463 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1204 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=248 p50_us=286 p95_us=363 max_us=16851
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
activity_process foreground=cmd.exe foreground_pid=11508 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=11508 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=9 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=6830 p50_us=10028 p95_us=48127 max_us=49594
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=62696 foreground_cwd=none root=cmd.exe root_pid=62696 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=29114 p50_us=37552 p95_us=74435 max_us=75885
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=72228 foreground_cwd=none root=cmd.exe root_pid=72228 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=25 bytes_per_sec=4477611.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
render_frame revision=1 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=70844 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=70844 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5128 p50_us=5638 p95_us=5962 max_us=15687
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
render_frame revision=364 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=30740 foreground_cwd=none root=cmd.exe root_pid=30740 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=364 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=105 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success

C:\Users\lixd2>echo KTS0041
KTS0041

C:\Users\lixd2>echo KTS0042
KTS0042

C:\Users\lixd2>echo KTS0043
KTS0043

C:\Users\lixd2>echo KTS0044
KTS0044

C:\Users\lixd2>echo KTS0045
KTS0045

C:\Users\lixd2>echo KTS0046
KTS0046

C:\Users\lixd2>echo KTS0047
KTS0047

C:\Users\lixd2>echo KTS0048
KTS0048

C:\Users\lixd2>echo KTS0049
KTS0049

C:\Users\lixd2>exit

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3097 min_us=1 p50_us=1 p95_us=1 max_us=110
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=29012 foreground_cwd=none root=cmd.exe root_pid=29012 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5040 p50_us=5292 p95_us=5542 max_us=15636
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
render_frame revision=360 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=38476 foreground_cwd=none root=cmd.exe root_pid=38476 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=360 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0041
UNTERM_NEXT_CORE_BENCH_0041

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0042
UNTERM_NEXT_CORE_BENCH_0042

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0043
UNTERM_NEXT_CORE_BENCH_0043

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0044
UNTERM_NEXT_CORE_BENCH_0044

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0045
UNTERM_NEXT_CORE_BENCH_0045

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0046
UNTERM_NEXT_CORE_BENCH_0046

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0047
UNTERM_NEXT_CORE_BENCH_0047

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0048
UNTERM_NEXT_CORE_BENCH_0048

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0049
UNTERM_NEXT_CORE_BENCH_0049

C:\Users\lixd2>exit

```

### output flood

```text
bench_flood lines=100000 bytes=1048576 elapsed_ms=24302 lines_per_sec=4114.9 bytes_per_sec=43147.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=174081 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=23616 foreground_cwd=none root=cmd.exe root_pid=23616 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=174081 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_99977
UNTERM_NEXT_CORE_FLOOD_99978
UNTERM_NEXT_CORE_FLOOD_99979
UNTERM_NEXT_CORE_FLOOD_99980
UNTERM_NEXT_CORE_FLOOD_99981
UNTERM_NEXT_CORE_FLOOD_99982
UNTERM_NEXT_CORE_FLOOD_99983
UNTERM_NEXT_CORE_FLOOD_99984
UNTERM_NEXT_CORE_FLOOD_99985
UNTERM_NEXT_CORE_FLOOD_99986
UNTERM_NEXT_CORE_FLOOD_99987
UNTERM_NEXT_CORE_FLOOD_99988
UNTERM_NEXT_CORE_FLOOD_99989
UNTERM_NEXT_CORE_FLOOD_99990
UNTERM_NEXT_CORE_FLOOD_99991
UNTERM_NEXT_CORE_FLOOD_99992
UNTERM_NEXT_CORE_FLOOD_99993
UNTERM_NEXT_CORE_FLOOD_99994
UNTERM_NEXT_CORE_FLOOD_99995
UNTERM_NEXT_CORE_FLOOD_99996
UNTERM_NEXT_CORE_FLOOD_99997
UNTERM_NEXT_CORE_FLOOD_99998
UNTERM_NEXT_CORE_FLOOD_99999
UNTERM_NEXT_CORE_FLOOD_100000

C:\Users\lixd2>echo UNTERM_NEXT_CORE_FLOOD_DONE_100000_1
UNTERM_NEXT_CORE_FLOOD_DONE_100000_1

C:\Users\lixd2>exit

```

### paste 10kb

```text
bench_paste bytes=10240 elapsed_ms=21 bytes_per_sec=480776.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
render_frame revision=14 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=16140 foreground_cwd=none root=cmd.exe root_pid=16140 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
QRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGH
IJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01
23456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRST
UVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKL
MNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCD
EFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ012345
6789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWX
YZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOP
QRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGH
IJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01
23456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRST
UVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKL
MNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCD
EFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ012345
6789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWX
YZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOP
QRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGH
IJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01
23456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRST
UVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKL
MNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOP
UNTERM_NEXT_CORE_PASTE_DONE_10240

C:\Users\lixd2>输入行太长。

C:\Users\lixd2>exit

```

### scrollback paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1299 lines_per_sec=7694.9 bytes_per_sec=806870.6
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=21 min_us=56 p50_us=61 p95_us=73 max_us=114
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=25081 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=6636 foreground_cwd=none root=cmd.exe root_pid=6636 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25081 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_9977
UNTERM_NEXT_CORE_FLOOD_9978
UNTERM_NEXT_CORE_FLOOD_9979
UNTERM_NEXT_CORE_FLOOD_9980
UNTERM_NEXT_CORE_FLOOD_9981
UNTERM_NEXT_CORE_FLOOD_9982
UNTERM_NEXT_CORE_FLOOD_9983
UNTERM_NEXT_CORE_FLOOD_9984
UNTERM_NEXT_CORE_FLOOD_9985
UNTERM_NEXT_CORE_FLOOD_9986
UNTERM_NEXT_CORE_FLOOD_9987
UNTERM_NEXT_CORE_FLOOD_9988
UNTERM_NEXT_CORE_FLOOD_9989
UNTERM_NEXT_CORE_FLOOD_9990
UNTERM_NEXT_CORE_FLOOD_9991
UNTERM_NEXT_CORE_FLOOD_9992
UNTERM_NEXT_CORE_FLOOD_9993
UNTERM_NEXT_CORE_FLOOD_9994
UNTERM_NEXT_CORE_FLOOD_9995
UNTERM_NEXT_CORE_FLOOD_9996
UNTERM_NEXT_CORE_FLOOD_9997
UNTERM_NEXT_CORE_FLOOD_9998
UNTERM_NEXT_CORE_FLOOD_9999
UNTERM_NEXT_CORE_FLOOD_10000

C:\Users\lixd2>echo UNTERM_NEXT_CORE_FLOOD_DONE_10000_1
UNTERM_NEXT_CORE_FLOOD_DONE_10000_1

C:\Users\lixd2>exit

```

### viewport scroll paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1252 lines_per_sec=7985.3 bytes_per_sec=837321.4
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=23 min_us=64 p50_us=70 p95_us=75 max_us=101
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25568 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=30936 foreground_cwd=none root=cmd.exe root_pid=30936 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25235 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>for /L %i in (1,1,10000) do @echo UNTERM_NEXT_CORE_FLOOD_%i
UNTERM_NEXT_CORE_FLOOD_1
UNTERM_NEXT_CORE_FLOOD_2
UNTERM_NEXT_CORE_FLOOD_3
UNTERM_NEXT_CORE_FLOOD_4
UNTERM_NEXT_CORE_FLOOD_5
UNTERM_NEXT_CORE_FLOOD_6
UNTERM_NEXT_CORE_FLOOD_7
UNTERM_NEXT_CORE_FLOOD_8
UNTERM_NEXT_CORE_FLOOD_9
UNTERM_NEXT_CORE_FLOOD_10
UNTERM_NEXT_CORE_FLOOD_11
UNTERM_NEXT_CORE_FLOOD_12
UNTERM_NEXT_CORE_FLOOD_13
UNTERM_NEXT_CORE_FLOOD_14
UNTERM_NEXT_CORE_FLOOD_15
UNTERM_NEXT_CORE_FLOOD_16
UNTERM_NEXT_CORE_FLOOD_17
UNTERM_NEXT_CORE_FLOOD_18
UNTERM_NEXT_CORE_FLOOD_19
UNTERM_NEXT_CORE_FLOOD_20
UNTERM_NEXT_CORE_FLOOD_21
UNTERM_NEXT_CORE_FLOOD_22
UNTERM_NEXT_CORE_FLOOD_23
UNTERM_NEXT_CORE_FLOOD_24
UNTERM_NEXT_CORE_FLOOD_25
UNTERM_NEXT_CORE_FLOOD_26
```

### viewport scroll during flood

```text
bench_viewport_scroll_flood lines=5000 scrolls=116 rows_read=3391 total_ms=676 min_us=13 p50_us=233 p95_us=374 max_us=546
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=653354
render_frame revision=14687 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=44916 foreground_cwd=none root=cmd.exe root_pid=44916 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14571 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=235 viewport_scrolls=116
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_1904
UNTERM_NEXT_CORE_FLOOD_1905
UNTERM_NEXT_CORE_FLOOD_1906
UNTERM_NEXT_CORE_FLOOD_1907
UNTERM_NEXT_CORE_FLOOD_1908
UNTERM_NEXT_CORE_FLOOD_1909
UNTERM_NEXT_CORE_FLOOD_1910
UNTERM_NEXT_CORE_FLOOD_1911
UNTERM_NEXT_CORE_FLOOD_1912
UNTERM_NEXT_CORE_FLOOD_1913
UNTERM_NEXT_CORE_FLOOD_1914
UNTERM_NEXT_CORE_FLOOD_1915
UNTERM_NEXT_CORE_FLOOD_1916
UNTERM_NEXT_CORE_FLOOD_1917
UNTERM_NEXT_CORE_FLOOD_1918
UNTERM_NEXT_CORE_FLOOD_1919
UNTERM_NEXT_CORE_FLOOD_1920
UNTERM_NEXT_CORE_FLOOD_1921
UNTERM_NEXT_CORE_FLOOD_1922
UNTERM_NEXT_CORE_FLOOD_1923
UNTERM_NEXT_CORE_FLOOD_1924
UNTERM_NEXT_CORE_FLOOD_1925
UNTERM_NEXT_CORE_FLOOD_1926
UNTERM_NEXT_CORE_FLOOD_1927
UNTERM_NEXT_CORE_FLOOD_1928
UNTERM_NEXT_CORE_FLOOD_1929
UNTERM_NEXT_CORE_FLOOD_1930
UNTERM_NEXT_CORE_FLOOD_1931
UNTERM_NEXT_CORE_FLOOD_1932
UNTERM_NEXT_CORE_FLOOD_1933
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5074 p50_us=5343 p95_us=5632 max_us=5701
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=764 combined_lines_per_sec=13084.2 combined_bytes_per_sec=1709457.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
render_frame revision=152 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=23288 foreground_cwd=none root=cmd.exe root_pid=23288 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=153 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0011
UNTERM_NEXT_CORE_BENCH_0011

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0012
UNTERM_NEXT_CORE_BENCH_0012

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0013
UNTERM_NEXT_CORE_BENCH_0013

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0014
UNTERM_NEXT_CORE_BENCH_0014

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0015
UNTERM_NEXT_CORE_BENCH_0015

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0016
UNTERM_NEXT_CORE_BENCH_0016

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0017
UNTERM_NEXT_CORE_BENCH_0017

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0018
UNTERM_NEXT_CORE_BENCH_0018

C:\Users\lixd2>echo UNTERM_NEXT_CORE_BENCH_0019
UNTERM_NEXT_CORE_BENCH_0019

C:\Users\lixd2>exit

```

### agent startup stall

```text
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=126 screen_reads=126 elapsed_ms=706 input_min_us=5 input_p50_us=8 input_p95_us=20 input_max_us=45 screen_read_min_us=14 screen_read_p50_us=19 screen_read_p95_us=31 screen_read_max_us=48
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=43372 foreground_cwd=none root=cmd.exe root_pid=43372 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=127 input_bytes=383 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=129 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=114 total_ms=644 min_us=11 p50_us=96 p95_us=141 max_us=427 text_bytes=85726
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=653354
render_frame revision=14534 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=2224 foreground_cwd=none root=cmd.exe root_pid=2224 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14534 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=117 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_4977
UNTERM_NEXT_CORE_FLOOD_4978
UNTERM_NEXT_CORE_FLOOD_4979
UNTERM_NEXT_CORE_FLOOD_4980
UNTERM_NEXT_CORE_FLOOD_4981
UNTERM_NEXT_CORE_FLOOD_4982
UNTERM_NEXT_CORE_FLOOD_4983
UNTERM_NEXT_CORE_FLOOD_4984
UNTERM_NEXT_CORE_FLOOD_4985
UNTERM_NEXT_CORE_FLOOD_4986
UNTERM_NEXT_CORE_FLOOD_4987
UNTERM_NEXT_CORE_FLOOD_4988
UNTERM_NEXT_CORE_FLOOD_4989
UNTERM_NEXT_CORE_FLOOD_4990
UNTERM_NEXT_CORE_FLOOD_4991
UNTERM_NEXT_CORE_FLOOD_4992
UNTERM_NEXT_CORE_FLOOD_4993
UNTERM_NEXT_CORE_FLOOD_4994
UNTERM_NEXT_CORE_FLOOD_4995
UNTERM_NEXT_CORE_FLOOD_4996
UNTERM_NEXT_CORE_FLOOD_4997
UNTERM_NEXT_CORE_FLOOD_4998
UNTERM_NEXT_CORE_FLOOD_4999
UNTERM_NEXT_CORE_FLOOD_5000

C:\Users\lixd2>echo UNTERM_NEXT_CORE_FLOOD_DONE_5000_1
UNTERM_NEXT_CORE_FLOOD_DONE_5000_1

C:\Users\lixd2>exit

```

### render frame latency

```text
bench_render_frame rounds=1000 full_us=458 full_lines=30 empty_deltas=1000 min_us=0 p50_us=0 p95_us=0 max_us=2 dirty_rounds=50 dirty_lines=1500 dirty_min_us=278 dirty_p50_us=382 dirty_p95_us=529 dirty_max_us=761
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
render_frame revision=463 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=17888 foreground_cwd=none root=cmd.exe root_pid=17888 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=463 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1204 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0041
RENDER_FRAME_DIRTY_0041

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0042
RENDER_FRAME_DIRTY_0042

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0043
RENDER_FRAME_DIRTY_0043

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0044
RENDER_FRAME_DIRTY_0044

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0045
RENDER_FRAME_DIRTY_0045

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0046
RENDER_FRAME_DIRTY_0046

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0047
RENDER_FRAME_DIRTY_0047

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0048
RENDER_FRAME_DIRTY_0048

C:\Users\lixd2>echo RENDER_FRAME_DIRTY_0049
RENDER_FRAME_DIRTY_0049

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=248 p50_us=286 p95_us=363 max_us=16851
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
render_frame revision=9 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=11508 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=11508 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=9 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session create latency

```text
bench_session_create rounds=20 min_us=6830 p50_us=10028 p95_us=48127 max_us=49594
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=62696 foreground_cwd=none root=cmd.exe root_pid=62696 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=29114 p50_us=37552 p95_us=74435 max_us=75885
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=72228 foreground_cwd=none root=cmd.exe root_pid=72228 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


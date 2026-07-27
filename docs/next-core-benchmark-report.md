# Next-Core Benchmark Report

- Generated: 2026-07-27 11:40:10 +08:00
- Commit: `8038dde`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=3 render_frame_revision=16 render_frame_lines=7 render_delta_lines=0 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| key-to-screen p95 | 5860 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5781 us | 16000 us | ok |
| dual-agent echo p95 | 5467 us | 33000 us | ok |
| agent startup input p95 | 17 us | 33000 us | ok |
| paste 10kb elapsed | 24 ms | 50 ms | ok |
| scrollback page p95 | 70 us | 1000 us | ok |
| viewport scroll p95 | 72 us | 1000 us | ok |
| viewport scroll under flood p95 | 300 us | 50000 us | ok |
| screen read under flood p95 | 115 us | 50000 us | ok |
| render frame p95 | 0 us | 1000 us | ok |
| focus switch p95 | 457 us | 100000 us | ok |
| session create p95 | 11275 us | 100000 us | ok |
| session ready p95 | 40966 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=65 bytes_per_sec=2388535.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=52204 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=52204 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5149 p50_us=5574 p95_us=5860 max_us=16620
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=86988 foreground_cwd=none root=cmd.exe root_pid=86988 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=366 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=105 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2797 min_us=0 p50_us=1 p95_us=1 max_us=206
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=89624 foreground_cwd=none root=cmd.exe root_pid=89624 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5077 p50_us=5349 p95_us=5781 max_us=16020
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=89492 foreground_cwd=none root=cmd.exe root_pid=89492 root_cwd=none child_count=0 detected_agent=none
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=28269 lines_per_sec=3537.4 bytes_per_sec=37092.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=39128 foreground_cwd=none root=cmd.exe root_pid=39128 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=170702 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=24 bytes_per_sec=415000.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=9876 foreground_cwd=none root=cmd.exe root_pid=9876 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=15 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1025 lines_per_sec=9752.3 bytes_per_sec=1022597.6
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=47 p50_us=50 p95_us=70 max_us=134
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=41024 foreground_cwd=none root=cmd.exe root_pid=41024 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24966 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1222 lines_per_sec=8176.7 bytes_per_sec=857392.6
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=20 min_us=50 p50_us=60 p95_us=72 max_us=141
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=81376 foreground_cwd=none root=cmd.exe root_pid=81376 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24986 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=97 rows_read=2822 total_ms=559 min_us=25 p50_us=211 p95_us=300 max_us=327
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=64876 foreground_cwd=none root=cmd.exe root_pid=64876 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14502 output_bytes=653271 paste_count=0 paste_text_bytes=0 screen_reads=197 viewport_scrolls=97
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5071 p50_us=5243 p95_us=5467 max_us=5632
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=802 combined_lines_per_sec=12468.6 combined_bytes_per_sec=1629024.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=68232 foreground_cwd=none root=cmd.exe root_pid=68232 root_cwd=none child_count=0 detected_agent=none
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=100 screen_reads=100 elapsed_ms=560 input_min_us=3 input_p50_us=7 input_p95_us=17 input_max_us=41 screen_read_min_us=11 screen_read_p50_us=16 screen_read_p95_us=28 screen_read_max_us=73
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=27764 foreground_cwd=none root=cmd.exe root_pid=27764 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=101 input_bytes=305 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=103 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=96 total_ms=540 min_us=10 p50_us=82 p95_us=115 max_us=153 text_bytes=72235
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=75032 foreground_cwd=none root=cmd.exe root_pid=75032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14663 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=99 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=168 full_lines=30 empty_deltas=1000 min_us=0 p50_us=0 p95_us=0 max_us=1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=3895
activity_process foreground=cmd.exe foreground_pid=78292 foreground_cwd=none root=cmd.exe root_pid=78292 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=2 input_bytes=58 output_chunks=104 output_bytes=3895 paste_count=0 paste_text_bytes=0 screen_reads=1004 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
RENDER_FRAME_BENCH_4
RENDER_FRAME_BENCH_5
RENDER_FRAME_BENCH_6
RENDER_FRAME_BENCH_7
RENDER_FRAME_BENCH_8
RENDER_FRAME_BENCH_9
RENDER_FRAME_BENCH_10
RENDER_FRAME_BENCH_11
RENDER_FRAME_BENCH_12
RENDER_FRAME_BENCH_13
RENDER_FRAME_BENCH_14
RENDER_FRAME_BENCH_15
RENDER_FRAME_BENCH_16
RENDER_FRAME_BENCH_17
RENDER_FRAME_BENCH_18
RENDER_FRAME_BENCH_19
RENDER_FRAME_BENCH_20
RENDER_FRAME_BENCH_21
RENDER_FRAME_BENCH_22
RENDER_FRAME_BENCH_23
RENDER_FRAME_BENCH_24
RENDER_FRAME_BENCH_25
RENDER_FRAME_BENCH_26
RENDER_FRAME_BENCH_27
RENDER_FRAME_BENCH_28
RENDER_FRAME_BENCH_29
RENDER_FRAME_BENCH_30
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=215 p50_us=295 p95_us=457 max_us=17621
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
activity_process foreground=cmd.exe foreground_pid=37004 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=37004 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=7609 p50_us=9732 p95_us=11275 max_us=45240
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=73472 foreground_cwd=none root=cmd.exe root_pid=73472 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=25921 p50_us=33252 p95_us=40966 max_us=45949
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=88720 foreground_cwd=none root=cmd.exe root_pid=88720 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=65 bytes_per_sec=2388535.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
render_frame revision=2 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=1 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=52204 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=52204 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5149 p50_us=5574 p95_us=5860 max_us=16620
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=7545
render_frame revision=366 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=86988 foreground_cwd=none root=cmd.exe root_pid=86988 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=366 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=105 viewport_scrolls=0
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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2797 min_us=0 p50_us=1 p95_us=1 max_us=206
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=4 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=89624 foreground_cwd=none root=cmd.exe root_pid=89624 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5077 p50_us=5349 p95_us=5781 max_us=16020
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
render_frame revision=360 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=89492 foreground_cwd=none root=cmd.exe root_pid=89492 root_cwd=none child_count=0 detected_agent=none
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=28269 lines_per_sec=3537.4 bytes_per_sec=37092.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=170702 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=39128 foreground_cwd=none root=cmd.exe root_pid=39128 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=170702 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
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
bench_paste bytes=10240 elapsed_ms=24 bytes_per_sec=415000.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
render_frame revision=15 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=9876 foreground_cwd=none root=cmd.exe root_pid=9876 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=15 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=3 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1025 lines_per_sec=9752.3 bytes_per_sec=1022597.6
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=47 p50_us=50 p95_us=70 max_us=134
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=24965 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=41024 foreground_cwd=none root=cmd.exe root_pid=41024 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24966 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1222 lines_per_sec=8176.7 bytes_per_sec=857392.6
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=20 min_us=50 p50_us=60 p95_us=72 max_us=141
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25319 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=81376 foreground_cwd=none root=cmd.exe root_pid=81376 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24986 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=338 viewport_scrolls=334
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
bench_viewport_scroll_flood lines=5000 scrolls=97 rows_read=2822 total_ms=559 min_us=25 p50_us=211 p95_us=300 max_us=327
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14599 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=64876 foreground_cwd=none root=cmd.exe root_pid=64876 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14502 output_bytes=653271 paste_count=0 paste_text_bytes=0 screen_reads=197 viewport_scrolls=97
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_2034
UNTERM_NEXT_CORE_FLOOD_2035
UNTERM_NEXT_CORE_FLOOD_2036
UNTERM_NEXT_CORE_FLOOD_2037
UNTERM_NEXT_CORE_FLOOD_2038
UNTERM_NEXT_CORE_FLOOD_2039
UNTERM_NEXT_CORE_FLOOD_2040
UNTERM_NEXT_CORE_FLOOD_2041
UNTERM_NEXT_CORE_FLOOD_2042
UNTERM_NEXT_CORE_FLOOD_2043
UNTERM_NEXT_CORE_FLOOD_2044
UNTERM_NEXT_CORE_FLOOD_2045
UNTERM_NEXT_CORE_FLOOD_2046
UNTERM_NEXT_CORE_FLOOD_2047
UNTERM_NEXT_CORE_FLOOD_2048
UNTERM_NEXT_CORE_FLOOD_2049
UNTERM_NEXT_CORE_FLOOD_2050
UNTERM_NEXT_CORE_FLOOD_2051
UNTERM_NEXT_CORE_FLOOD_2052
UNTERM_NEXT_CORE_FLOOD_2053
UNTERM_NEXT_CORE_FLOOD_2054
UNTERM_NEXT_CORE_FLOOD_2055
UNTERM_NEXT_CORE_FLOOD_2056
UNTERM_NEXT_CORE_FLOOD_2057
UNTERM_NEXT_CORE_FLOOD_2058
UNTERM_NEXT_CORE_FLOOD_2059
UNTERM_NEXT_CORE_FLOOD_2060
UNTERM_NEXT_CORE_FLOOD_2061
UNTERM_NEXT_CORE_FLOOD_2062
UNTERM_NEXT_CORE_FLOOD_2063
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5071 p50_us=5243 p95_us=5467 max_us=5632
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=802 combined_lines_per_sec=12468.6 combined_bytes_per_sec=1629024.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
render_frame revision=152 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=68232 foreground_cwd=none root=cmd.exe root_pid=68232 root_cwd=none child_count=0 detected_agent=none
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=100 screen_reads=100 elapsed_ms=560 input_min_us=3 input_p50_us=7 input_p95_us=17 input_max_us=41 screen_read_min_us=11 screen_read_p50_us=16 screen_read_p95_us=28 screen_read_max_us=73
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
render_frame revision=13 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=5 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=27764 foreground_cwd=none root=cmd.exe root_pid=27764 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=101 input_bytes=305 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=103 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=96 total_ms=540 min_us=10 p50_us=82 p95_us=115 max_us=153 text_bytes=72235
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14662 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=75032 foreground_cwd=none root=cmd.exe root_pid=75032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14663 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=99 viewport_scrolls=0
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
bench_render_frame rounds=1000 full_us=168 full_lines=30 empty_deltas=1000 min_us=0 p50_us=0 p95_us=0 max_us=1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=3895
render_frame revision=104 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=78292 foreground_cwd=none root=cmd.exe root_pid=78292 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=2 input_bytes=58 output_chunks=104 output_bytes=3895 paste_count=0 paste_text_bytes=0 screen_reads=1004 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
RENDER_FRAME_BENCH_4
RENDER_FRAME_BENCH_5
RENDER_FRAME_BENCH_6
RENDER_FRAME_BENCH_7
RENDER_FRAME_BENCH_8
RENDER_FRAME_BENCH_9
RENDER_FRAME_BENCH_10
RENDER_FRAME_BENCH_11
RENDER_FRAME_BENCH_12
RENDER_FRAME_BENCH_13
RENDER_FRAME_BENCH_14
RENDER_FRAME_BENCH_15
RENDER_FRAME_BENCH_16
RENDER_FRAME_BENCH_17
RENDER_FRAME_BENCH_18
RENDER_FRAME_BENCH_19
RENDER_FRAME_BENCH_20
RENDER_FRAME_BENCH_21
RENDER_FRAME_BENCH_22
RENDER_FRAME_BENCH_23
RENDER_FRAME_BENCH_24
RENDER_FRAME_BENCH_25
RENDER_FRAME_BENCH_26
RENDER_FRAME_BENCH_27
RENDER_FRAME_BENCH_28
RENDER_FRAME_BENCH_29
RENDER_FRAME_BENCH_30

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=215 p50_us=295 p95_us=457 max_us=17621
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=4 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37004 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=37004 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session create latency

```text
bench_session_create rounds=20 min_us=7609 p50_us=9732 p95_us=11275 max_us=45240
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=4 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=73472 foreground_cwd=none root=cmd.exe root_pid=73472 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=25921 p50_us=33252 p95_us=40966 max_us=45949
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=4 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=88720 foreground_cwd=none root=cmd.exe root_pid=88720 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=3 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


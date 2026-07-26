# Next-Core Benchmark Report

- Generated: 2026-07-27 03:15:02 +08:00
- Commit: `56ffdb9`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=65 foreground=cmd.exe screen_reads=1 lifecycle_created=1 dead_reason=process_exited:Success`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5647 us | 16000 us | ok |
| dual-agent echo p95 | 5769 us | 33000 us | ok |
| paste 10kb elapsed | 35 ms | 50 ms | ok |
| scrollback page p95 | 42 us | 1000 us | ok |
| viewport scroll p95 | 61 us | 1000 us | ok |
| viewport scroll under flood p95 | 257 us | 50000 us | ok |
| screen read under flood p95 | 136 us | 50000 us | ok |
| focus switch p95 | 36 us | 100000 us | ok |
| session create p95 | 12151 us | 100000 us | ok |
| session ready p95 | 69128 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=19 bytes_per_sec=3610108.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=43
activity_process foreground=cmd.exe foreground_pid=79344 root=cmd.exe root_pid=79344 child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=43 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2962 min_us=0 p50_us=1 p95_us=1 max_us=22
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=19048 root=cmd.exe root_pid=19048 child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5089 p50_us=5538 p95_us=5647 max_us=21330
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=33900 root=cmd.exe root_pid=33900 child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=366 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=32077 lines_per_sec=3117.4 bytes_per_sec=32688.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=50012 root=cmd.exe root_pid=50012 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=177448 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=35 bytes_per_sec=285914.5
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=21076 root=cmd.exe root_pid=21076 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1241 lines_per_sec=8056.5 bytes_per_sec=844785.3
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=11 min_us=28 p50_us=34 p95_us=42 max_us=87
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=83092 root=cmd.exe root_pid=83092 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24725 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=2 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1301 lines_per_sec=7681.1 bytes_per_sec=805422.9
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=16 min_us=37 p50_us=49 p95_us=61 max_us=124
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=18628 root=cmd.exe root_pid=18628 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24806 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=336 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=115 rows_read=3346 total_ms=661 min_us=11 p50_us=157 p95_us=257 max_us=314
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=77368 root=cmd.exe root_pid=77368 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14677 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=231 viewport_scrolls=115
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5200 p50_us=5513 p95_us=5769 max_us=10976
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=763 combined_lines_per_sec=13091.6 combined_bytes_per_sec=1710423.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=19388 root=cmd.exe root_pid=19388 child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=158 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=122 total_ms=703 min_us=15 p50_us=79 p95_us=136 max_us=220 text_bytes=91988
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=29468 root=cmd.exe root_pid=29468 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14688 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=123 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=10 p50_us=14 p95_us=36 max_us=141
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=29252 root=cmd.exe root_pid=29252 child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=14 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=process_exited:Success
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=7194 p50_us=9619 p95_us=12151 max_us=44129
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=62964 root=cmd.exe root_pid=62964 child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=29690 p50_us=37154 p95_us=69128 max_us=74102
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=50364 root=cmd.exe root_pid=50364 child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=19 bytes_per_sec=3610108.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=43
activity_process foreground=cmd.exe foreground_pid=79344 root=cmd.exe root_pid=79344 child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=43 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2962 min_us=0 p50_us=1 p95_us=1 max_us=22
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=19048 root=cmd.exe root_pid=19048 child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5089 p50_us=5538 p95_us=5647 max_us=21330
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=33900 root=cmd.exe root_pid=33900 child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=366 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=32077 lines_per_sec=3117.4 bytes_per_sec=32688.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=50012 root=cmd.exe root_pid=50012 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=177448 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_paste bytes=10240 elapsed_ms=35 bytes_per_sec=285914.5
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=21076 root=cmd.exe root_pid=21076 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
























UNTERM_NEXT_CORE_PASTE_DONE_10240

C:\Users\lixd2>输入行太长。

C:\Users\lixd2>exit

```

### scrollback paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1241 lines_per_sec=8056.5 bytes_per_sec=844785.3
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=11 min_us=28 p50_us=34 p95_us=42 max_us=87
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=83092 root=cmd.exe root_pid=83092 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24725 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=2 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1301 lines_per_sec=7681.1 bytes_per_sec=805422.9
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=16 min_us=37 p50_us=49 p95_us=61 max_us=124
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=18628 root=cmd.exe root_pid=18628 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24806 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=336 viewport_scrolls=334
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
bench_viewport_scroll_flood lines=5000 scrolls=115 rows_read=3346 total_ms=661 min_us=11 p50_us=157 p95_us=257 max_us=314
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=77368 root=cmd.exe root_pid=77368 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14677 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=231 viewport_scrolls=115
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_133
UNTERM_NEXT_CORE_FLOOD_134
UNTERM_NEXT_CORE_FLOOD_135
UNTERM_NEXT_CORE_FLOOD_136
UNTERM_NEXT_CORE_FLOOD_137
UNTERM_NEXT_CORE_FLOOD_138
UNTERM_NEXT_CORE_FLOOD_139
UNTERM_NEXT_CORE_FLOOD_140
UNTERM_NEXT_CORE_FLOOD_141
UNTERM_NEXT_CORE_FLOOD_142
UNTERM_NEXT_CORE_FLOOD_143
UNTERM_NEXT_CORE_FLOOD_144
UNTERM_NEXT_CORE_FLOOD_145
UNTERM_NEXT_CORE_FLOOD_146
UNTERM_NEXT_CORE_FLOOD_147
UNTERM_NEXT_CORE_FLOOD_148
UNTERM_NEXT_CORE_FLOOD_149
UNTERM_NEXT_CORE_FLOOD_150
UNTERM_NEXT_CORE_FLOOD_151
UNTERM_NEXT_CORE_FLOOD_152
UNTERM_NEXT_CORE_FLOOD_153
UNTERM_NEXT_CORE_FLOOD_154
UNTERM_NEXT_CORE_FLOOD_155
UNTERM_NEXT_CORE_FLOOD_156
UNTERM_NEXT_CORE_FLOOD_157
UNTERM_NEXT_CORE_FLOOD_158
UNTERM_NEXT_CORE_FLOOD_159
UNTERM_NEXT_CORE_FLOOD_160
UNTERM_NEXT_CORE_FLOOD_161
UNTERM_NEXT_CORE_FLOOD_162
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5200 p50_us=5513 p95_us=5769 max_us=10976
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=763 combined_lines_per_sec=13091.6 combined_bytes_per_sec=1710423.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=19388 root=cmd.exe root_pid=19388 child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=158 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=122 total_ms=703 min_us=15 p50_us=79 p95_us=136 max_us=220 text_bytes=91988
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=29468 root=cmd.exe root_pid=29468 child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14688 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=123 viewport_scrolls=0
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

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=10 p50_us=14 p95_us=36 max_us=141
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=29252 root=cmd.exe root_pid=29252 child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=14 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=7194 p50_us=9619 p95_us=12151 max_us=44129
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=62964 root=cmd.exe root_pid=62964 child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=29690 p50_us=37154 p95_us=69128 max_us=74102
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=50364 root=cmd.exe root_pid=50364 child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


# Next-Core Benchmark Report

- Generated: 2026-07-27 03:19:40 +08:00
- Commit: `c0f8cc8`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ screen_reads=1 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5693 us | 16000 us | ok |
| dual-agent echo p95 | 5567 us | 33000 us | ok |
| paste 10kb elapsed | 21 ms | 50 ms | ok |
| scrollback page p95 | 45 us | 1000 us | ok |
| viewport scroll p95 | 50 us | 1000 us | ok |
| viewport scroll under flood p95 | 271 us | 50000 us | ok |
| screen read under flood p95 | 143 us | 50000 us | ok |
| focus switch p95 | 373 us | 100000 us | ok |
| session create p95 | 13729 us | 100000 us | ok |
| session ready p95 | 41232 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=35 bytes_per_sec=2967359.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=61384 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=61384 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3284 min_us=0 p50_us=1 p95_us=1 max_us=246
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=33896 foreground_cwd=none root=cmd.exe root_pid=33896 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5151 p50_us=5520 p95_us=5693 max_us=16446
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=67224 foreground_cwd=none root=cmd.exe root_pid=67224 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=367 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=30327 lines_per_sec=3297.3 bytes_per_sec=34575.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=75660 foreground_cwd=none root=cmd.exe root_pid=75660 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=172611 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=21 bytes_per_sec=474311.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=22120 foreground_cwd=none root=cmd.exe root_pid=22120 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=13 output_bytes=3273 paste_count=1 paste_text_bytes=10241 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1264 lines_per_sec=7905.8 bytes_per_sec=828982.3
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=11 min_us=28 p50_us=33 p95_us=45 max_us=54
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=51632 foreground_cwd=none root=cmd.exe root_pid=51632 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24772 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=2 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1350 lines_per_sec=7404.5 bytes_per_sec=776414.4
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=13 min_us=31 p50_us=39 p95_us=50 max_us=71
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=18176 foreground_cwd=none root=cmd.exe root_pid=18176 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24987 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=336 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=111 rows_read=3242 total_ms=636 min_us=11 p50_us=172 p95_us=271 max_us=503
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=74588 foreground_cwd=none root=cmd.exe root_pid=74588 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14646 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=223 viewport_scrolls=111
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5043 p50_us=5506 p95_us=5567 max_us=5597
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=786 combined_lines_per_sec=12718.5 combined_bytes_per_sec=1661669.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=30412 foreground_cwd=none root=cmd.exe root_pid=30412 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=153 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_screen_read_flood lines=5000 reads=123 total_ms=698 min_us=18 p50_us=71 p95_us=143 max_us=176 text_bytes=92768
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=32240 foreground_cwd=none root=cmd.exe root_pid=32240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14796 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=124 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=202 p50_us=265 p95_us=373 max_us=17715
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
activity_process foreground=cmd.exe foreground_pid=56444 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=56444 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=7911 p50_us=10358 p95_us=13729 max_us=43855
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=73368 foreground_cwd=none root=cmd.exe root_pid=73368 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=33539 p50_us=35658 p95_us=41232 max_us=46772
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=80372 foreground_cwd=none root=cmd.exe root_pid=80372 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=35 bytes_per_sec=2967359.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=61384 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=61384 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3284 min_us=0 p50_us=1 p95_us=1 max_us=246
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=33896 foreground_cwd=none root=cmd.exe root_pid=33896 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5151 p50_us=5520 p95_us=5693 max_us=16446
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=67224 foreground_cwd=none root=cmd.exe root_pid=67224 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=367 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=30327 lines_per_sec=3297.3 bytes_per_sec=34575.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=75660 foreground_cwd=none root=cmd.exe root_pid=75660 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=172611 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_paste bytes=10240 elapsed_ms=21 bytes_per_sec=474311.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=22120 foreground_cwd=none root=cmd.exe root_pid=22120 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=13 output_bytes=3273 paste_count=1 paste_text_bytes=10241 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
























UNTERM_NEXT_CORE_PASTE_DONE_10240

C:\Users\lixd2>输入行太长。

C:\Users\lixd2>exit

```

### scrollback paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1264 lines_per_sec=7905.8 bytes_per_sec=828982.3
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=11 min_us=28 p50_us=33 p95_us=45 max_us=54
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=51632 foreground_cwd=none root=cmd.exe root_pid=51632 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24772 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=2 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1350 lines_per_sec=7404.5 bytes_per_sec=776414.4
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=13 min_us=31 p50_us=39 p95_us=50 max_us=71
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=18176 foreground_cwd=none root=cmd.exe root_pid=18176 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24987 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=336 viewport_scrolls=334
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
bench_viewport_scroll_flood lines=5000 scrolls=111 rows_read=3242 total_ms=636 min_us=11 p50_us=172 p95_us=271 max_us=503
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=74588 foreground_cwd=none root=cmd.exe root_pid=74588 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14646 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=223 viewport_scrolls=111
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_1678
UNTERM_NEXT_CORE_FLOOD_1679
UNTERM_NEXT_CORE_FLOOD_1680
UNTERM_NEXT_CORE_FLOOD_1681
UNTERM_NEXT_CORE_FLOOD_1682
UNTERM_NEXT_CORE_FLOOD_1683
UNTERM_NEXT_CORE_FLOOD_1684
UNTERM_NEXT_CORE_FLOOD_1685
UNTERM_NEXT_CORE_FLOOD_1686
UNTERM_NEXT_CORE_FLOOD_1687
UNTERM_NEXT_CORE_FLOOD_1688
UNTERM_NEXT_CORE_FLOOD_1689
UNTERM_NEXT_CORE_FLOOD_1690
UNTERM_NEXT_CORE_FLOOD_1691
UNTERM_NEXT_CORE_FLOOD_1692
UNTERM_NEXT_CORE_FLOOD_1693
UNTERM_NEXT_CORE_FLOOD_1694
UNTERM_NEXT_CORE_FLOOD_1695
UNTERM_NEXT_CORE_FLOOD_1696
UNTERM_NEXT_CORE_FLOOD_1697
UNTERM_NEXT_CORE_FLOOD_1698
UNTERM_NEXT_CORE_FLOOD_1699
UNTERM_NEXT_CORE_FLOOD_1700
UNTERM_NEXT_CORE_FLOOD_1701
UNTERM_NEXT_CORE_FLOOD_1702
UNTERM_NEXT_CORE_FLOOD_1703
UNTERM_NEXT_CORE_FLOOD_1704
UNTERM_NEXT_CORE_FLOOD_1705
UNTERM_NEXT_CORE_FLOOD_1706
UNTERM_NEXT_CORE_FLOOD_1707
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5043 p50_us=5506 p95_us=5567 max_us=5597
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=786 combined_lines_per_sec=12718.5 combined_bytes_per_sec=1661669.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=30412 foreground_cwd=none root=cmd.exe root_pid=30412 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=153 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
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
bench_screen_read_flood lines=5000 reads=123 total_ms=698 min_us=18 p50_us=71 p95_us=143 max_us=176 text_bytes=92768
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=32240 foreground_cwd=none root=cmd.exe root_pid=32240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14796 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=124 viewport_scrolls=0
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
bench_focus_switch rounds=1000 sessions=4 min_us=202 p50_us=265 p95_us=373 max_us=17715
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
activity_process foreground=cmd.exe foreground_pid=56444 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=56444 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session create latency

```text
bench_session_create rounds=20 min_us=7911 p50_us=10358 p95_us=13729 max_us=43855
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=73368 foreground_cwd=none root=cmd.exe root_pid=73368 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=33539 p50_us=35658 p95_us=41232 max_us=46772
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=80372 foreground_cwd=none root=cmd.exe root_pid=80372 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


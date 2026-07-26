# Next-Core Benchmark Report

- Generated: 2026-07-27 03:04:08 +08:00
- Commit: `7397895`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=65 screen_reads=1 lifecycle_created=1 dead_reason=process_exited:Success`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5672 us | 16000 us | ok |
| dual-agent echo p95 | 5924 us | 33000 us | ok |
| paste 10kb elapsed | 36 ms | 50 ms | ok |
| scrollback page p95 | 45 us | 1000 us | ok |
| viewport scroll p95 | 41 us | 1000 us | ok |
| viewport scroll under flood p95 | 246 us | 50000 us | ok |
| screen read under flood p95 | 111 us | 50000 us | ok |
| focus switch p95 | 35 us | 100000 us | ok |
| session create p95 | 11761 us | 100000 us | ok |
| session ready p95 | 42905 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=0 p95_us=1 max_us=25 bytes_per_sec=7874015.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=0
health_io input_writes=1001 input_bytes=3005 output_chunks=0 output_bytes=0 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3025 min_us=0 p50_us=1 p95_us=1 max_us=240
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1001 input_bytes=3005 output_chunks=11 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=3 total_destroyed=2 total_marked_dead=2 last_dead_reason=destroyed
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5068 p50_us=5505 p95_us=5672 max_us=16439
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=10455
health_io input_writes=51 input_bytes=1655 output_chunks=359 output_bytes=10435 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=31466 lines_per_sec=3177.9 bytes_per_sec=33323.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
health_io input_writes=3 input_bytes=108 output_chunks=176155 output_bytes=13278263 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=36 bytes_per_sec=280624.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=319
health_io input_writes=3 input_bytes=10322 output_chunks=12 output_bytes=319 paste_count=1 paste_text_bytes=10241 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1252 lines_per_sec=7985.1 bytes_per_sec=837297.3
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=12 min_us=29 p50_us=37 p95_us=45 max_us=64
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
health_io input_writes=3 input_bytes=106 output_chunks=24804 output_bytes=1308257 paste_count=0 paste_text_bytes=0 screen_reads=2 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1305 lines_per_sec=7661.9 bytes_per_sec=803411.2
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=11 min_us=30 p50_us=33 p95_us=41 max_us=61
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
health_io input_writes=3 input_bytes=106 output_chunks=24795 output_bytes=1308257 paste_count=0 paste_text_bytes=0 screen_reads=336 viewport_scrolls=334
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=111 rows_read=3214 total_ms=636 min_us=11 p50_us=163 p95_us=246 max_us=356
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653251
health_io input_writes=3 input_bytes=104 output_chunks=14657 output_bytes=653251 paste_count=0 paste_text_bytes=0 screen_reads=223 viewport_scrolls=111
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5064 p50_us=5527 p95_us=5924 max_us=10163
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=770 combined_lines_per_sec=12984.2 combined_bytes_per_sec=1696386.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=4274
health_io input_writes=21 input_bytes=665 output_chunks=150 output_bytes=4254 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=3 total_destroyed=2 total_marked_dead=2 last_dead_reason=destroyed
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
bench_screen_read_flood lines=5000 reads=117 total_ms=660 min_us=11 p50_us=67 p95_us=111 max_us=163 text_bytes=87276
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653251
health_io input_writes=3 input_bytes=104 output_chunks=14707 output_bytes=653251 paste_count=0 paste_text_bytes=0 screen_reads=118 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=10 p50_us=14 p95_us=35 max_us=190
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1 input_bytes=5 output_chunks=10 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=6914 p50_us=10237 p95_us=11761 max_us=25640
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1 input_bytes=5 output_chunks=8 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=21 total_destroyed=20 total_marked_dead=20 last_dead_reason=destroyed
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=31510 p50_us=36723 p95_us=42905 max_us=46773
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1 input_bytes=5 output_chunks=9 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=21 total_destroyed=20 total_marked_dead=20 last_dead_reason=destroyed
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=0 p95_us=1 max_us=25 bytes_per_sec=7874015.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=0
health_io input_writes=1001 input_bytes=3005 output_chunks=0 output_bytes=0 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3025 min_us=0 p50_us=1 p95_us=1 max_us=240
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1001 input_bytes=3005 output_chunks=11 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=3 total_destroyed=2 total_marked_dead=2 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### echo latency

```text
bench_echo rounds=50 min_us=5068 p50_us=5505 p95_us=5672 max_us=16439
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=10455
health_io input_writes=51 input_bytes=1655 output_chunks=359 output_bytes=10435 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

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
bench_flood lines=100000 bytes=1048576 elapsed_ms=31466 lines_per_sec=3177.9 bytes_per_sec=33323.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
health_io input_writes=3 input_bytes=108 output_chunks=176155 output_bytes=13278263 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
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
bench_paste bytes=10240 elapsed_ms=36 bytes_per_sec=280624.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=319
health_io input_writes=3 input_bytes=10322 output_chunks=12 output_bytes=319 paste_count=1 paste_text_bytes=10241 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>set /p UNTERM_NEXT_CORE_PASTE_INPUT=&echo UNTERM_NEXT_CORE_PASTE_DONE_10240

```

### scrollback paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1252 lines_per_sec=7985.1 bytes_per_sec=837297.3
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=12 min_us=29 p50_us=37 p95_us=45 max_us=64
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
health_io input_writes=3 input_bytes=106 output_chunks=24804 output_bytes=1308257 paste_count=0 paste_text_bytes=0 screen_reads=2 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1305 lines_per_sec=7661.9 bytes_per_sec=803411.2
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=11 min_us=30 p50_us=33 p95_us=41 max_us=61
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=1048576
health_io input_writes=3 input_bytes=106 output_chunks=24795 output_bytes=1308257 paste_count=0 paste_text_bytes=0 screen_reads=336 viewport_scrolls=334
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
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
bench_viewport_scroll_flood lines=5000 scrolls=111 rows_read=3214 total_ms=636 min_us=11 p50_us=163 p95_us=246 max_us=356
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653251
health_io input_writes=3 input_bytes=104 output_chunks=14657 output_bytes=653251 paste_count=0 paste_text_bytes=0 screen_reads=223 viewport_scrolls=111
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
UNTERM_NEXT_CORE_FLOOD_632
UNTERM_NEXT_CORE_FLOOD_633
UNTERM_NEXT_CORE_FLOOD_634
UNTERM_NEXT_CORE_FLOOD_635
UNTERM_NEXT_CORE_FLOOD_636
UNTERM_NEXT_CORE_FLOOD_637
UNTERM_NEXT_CORE_FLOOD_638
UNTERM_NEXT_CORE_FLOOD_639
UNTERM_NEXT_CORE_FLOOD_640
UNTERM_NEXT_CORE_FLOOD_641
UNTERM_NEXT_CORE_FLOOD_642
UNTERM_NEXT_CORE_FLOOD_643
UNTERM_NEXT_CORE_FLOOD_644
UNTERM_NEXT_CORE_FLOOD_645
UNTERM_NEXT_CORE_FLOOD_646
UNTERM_NEXT_CORE_FLOOD_647
UNTERM_NEXT_CORE_FLOOD_648
UNTERM_NEXT_CORE_FLOOD_649
UNTERM_NEXT_CORE_FLOOD_650
UNTERM_NEXT_CORE_FLOOD_651
UNTERM_NEXT_CORE_FLOOD_652
UNTERM_NEXT_CORE_FLOOD_653
UNTERM_NEXT_CORE_FLOOD_654
UNTERM_NEXT_CORE_FLOOD_655
UNTERM_NEXT_CORE_FLOOD_656
UNTERM_NEXT_CORE_FLOOD_657
UNTERM_NEXT_CORE_FLOOD_658
UNTERM_NEXT_CORE_FLOOD_659
UNTERM_NEXT_CORE_FLOOD_660
UNTERM_NEXT_CORE_FLOOD_661
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5064 p50_us=5527 p95_us=5924 max_us=10163
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=770 combined_lines_per_sec=12984.2 combined_bytes_per_sec=1696386.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=4274
health_io input_writes=21 input_bytes=665 output_chunks=150 output_bytes=4254 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=3 total_destroyed=2 total_marked_dead=2 last_dead_reason=destroyed

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
bench_screen_read_flood lines=5000 reads=117 total_ms=660 min_us=11 p50_us=67 p95_us=111 max_us=163 text_bytes=87276
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 29) raw_bytes=653251
health_io input_writes=3 input_bytes=104 output_chunks=14707 output_bytes=653251 paste_count=0 paste_text_bytes=0 screen_reads=118 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
UNTERM_NEXT_CORE_FLOOD_4976
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

C:\Users\lixd2>
```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=10 p50_us=14 p95_us=35 max_us=190
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1 input_bytes=5 output_chunks=10 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session create latency

```text
bench_session_create rounds=20 min_us=6914 p50_us=10237 p95_us=11761 max_us=25640
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1 input_bytes=5 output_chunks=8 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=21 total_destroyed=20 total_marked_dead=20 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session ready latency

```text
bench_session_ready rounds=20 min_us=31510 p50_us=36723 p95_us=42905 max_us=46773
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
health_io input_writes=1 input_bytes=5 output_chunks=9 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=1 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=21 total_destroyed=20 total_marked_dead=20 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```


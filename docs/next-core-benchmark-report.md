# Next-Core Benchmark Report

- Generated: 2026-07-27 02:45:48 +08:00
- Commit: `c324320`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=65`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5639 us | 16000 us | ok |
| dual-agent echo p95 | 5541 us | 33000 us | ok |
| paste 10kb elapsed | 19 ms | 50 ms | ok |
| scrollback page p95 | 46 us | 1000 us | ok |
| viewport scroll p95 | 57 us | 1000 us | ok |
| viewport scroll under flood p95 | 357 us | 50000 us | ok |
| screen read under flood p95 | 223 us | 50000 us | ok |
| focus switch p95 | 35 us | 100000 us | ok |
| session create p95 | 49541 us | 100000 us | ok |
| session ready p95 | 78562 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=0 p95_us=1 max_us=28 bytes_per_sec=14354067.0
session id=1 cols=100 rows=30 dead=false cursor=(0, 0) raw_bytes=0
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2985 min_us=0 p50_us=1 p95_us=1 max_us=92
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5198 p50_us=5524 p95_us=5639 max_us=16357
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=10464
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=28711 lines_per_sec=3483.0 bytes_per_sec=36521.6
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=1048576
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=19 bytes_per_sec=519151.3
session id=1 cols=100 rows=30 dead=false cursor=(0, 4) raw_bytes=319
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1221 lines_per_sec=8188.1 bytes_per_sec=858583.4
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=13 min_us=30 p50_us=39 p95_us=46 max_us=166
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=1048576
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1290 lines_per_sec=7747.0 bytes_per_sec=812332.6
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=15 min_us=36 p50_us=45 p95_us=57 max_us=135
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=1048576
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=107 rows_read=3094 total_ms=617 min_us=49 p50_us=244 p95_us=357 max_us=495
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=653251
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5050 p50_us=5500 p95_us=5541 max_us=5566
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=723 combined_lines_per_sec=13830.4 combined_bytes_per_sec=1806938.5
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=4262
UNTERM_NEXT_CORE_BENCH_0010
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
bench_screen_read_flood lines=5000 reads=115 total_ms=653 min_us=35 p50_us=134 p95_us=223 max_us=332 text_bytes=86974
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=653251
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=10 p50_us=15 p95_us=35 max_us=161
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=6673 p50_us=9286 p95_us=49541 max_us=51154
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=34564 p50_us=43990 p95_us=78562 max_us=81423
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=0 p95_us=1 max_us=28 bytes_per_sec=14354067.0
session id=1 cols=100 rows=30 dead=false cursor=(0, 0) raw_bytes=0

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2985 min_us=0 p50_us=1 p95_us=1 max_us=92
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### echo latency

```text
bench_echo rounds=50 min_us=5198 p50_us=5524 p95_us=5639 max_us=16357
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=10464

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
bench_flood lines=100000 bytes=1048576 elapsed_ms=28711 lines_per_sec=3483.0 bytes_per_sec=36521.6
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=1048576
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
bench_paste bytes=10240 elapsed_ms=19 bytes_per_sec=519151.3
session id=1 cols=100 rows=30 dead=false cursor=(0, 4) raw_bytes=319
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>set /p UNTERM_NEXT_CORE_PASTE_INPUT=&echo UNTERM_NEXT_CORE_PASTE_DONE_10240

```

### scrollback paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1221 lines_per_sec=8188.1 bytes_per_sec=858583.4
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=13 min_us=30 p50_us=39 p95_us=46 max_us=166
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=1048576
UNTERM_NEXT_CORE_FLOOD_9976
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

C:\Users\lixd2>
```

### viewport scroll paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1290 lines_per_sec=7747.0 bytes_per_sec=812332.6
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=15 min_us=36 p50_us=45 p95_us=57 max_us=135
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=1048576
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
bench_viewport_scroll_flood lines=5000 scrolls=107 rows_read=3094 total_ms=617 min_us=49 p50_us=244 p95_us=357 max_us=495
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=653251
UNTERM_NEXT_CORE_FLOOD_904
UNTERM_NEXT_CORE_FLOOD_905
UNTERM_NEXT_CORE_FLOOD_906
UNTERM_NEXT_CORE_FLOOD_907
UNTERM_NEXT_CORE_FLOOD_908
UNTERM_NEXT_CORE_FLOOD_909
UNTERM_NEXT_CORE_FLOOD_910
UNTERM_NEXT_CORE_FLOOD_911
UNTERM_NEXT_CORE_FLOOD_912
UNTERM_NEXT_CORE_FLOOD_913
UNTERM_NEXT_CORE_FLOOD_914
UNTERM_NEXT_CORE_FLOOD_915
UNTERM_NEXT_CORE_FLOOD_916
UNTERM_NEXT_CORE_FLOOD_917
UNTERM_NEXT_CORE_FLOOD_918
UNTERM_NEXT_CORE_FLOOD_919
UNTERM_NEXT_CORE_FLOOD_920
UNTERM_NEXT_CORE_FLOOD_921
UNTERM_NEXT_CORE_FLOOD_922
UNTERM_NEXT_CORE_FLOOD_923
UNTERM_NEXT_CORE_FLOOD_924
UNTERM_NEXT_CORE_FLOOD_925
UNTERM_NEXT_CORE_FLOOD_926
UNTERM_NEXT_CORE_FLOOD_927
UNTERM_NEXT_CORE_FLOOD_928
UNTERM_NEXT_CORE_FLOOD_929
UNTERM_NEXT_CORE_FLOOD_930
UNTERM_NEXT_CORE_FLOOD_931
UNTERM_NEXT_CORE_FLOOD_932
UNTERM_NEXT_CORE_FLOOD_933
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5050 p50_us=5500 p95_us=5541 max_us=5566
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=723 combined_lines_per_sec=13830.4 combined_bytes_per_sec=1806938.5
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=4262
UNTERM_NEXT_CORE_BENCH_0010

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

C:\Users\lixd2>
```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=115 total_ms=653 min_us=35 p50_us=134 p95_us=223 max_us=332 text_bytes=86974
session id=1 cols=100 rows=30 dead=false cursor=(15, 29) raw_bytes=653251
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

C:\Users\lixd2>exit
```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=10 p50_us=15 p95_us=35 max_us=161
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session create latency

```text
bench_session_create rounds=20 min_us=6673 p50_us=9286 p95_us=49541 max_us=51154
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session ready latency

```text
bench_session_ready rounds=20 min_us=34564 p50_us=43990 p95_us=78562 max_us=81423
session id=1 cols=100 rows=30 dead=false cursor=(15, 3) raw_bytes=155
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```


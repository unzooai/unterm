# Next-Core Benchmark Report

- Generated: 2026-07-28 00:36:48 +08:00
- Commit: `664e792`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=16 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=16 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=19 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 runtime_pump_dispatches=10 runtime_pump_lanes=lifecycle:1,input:1,render:5,screen:1,background:2 runtime_pump_waited=0 runtime_pump_completed_without_wait=10 runtime_pump_max_dispatch_us=21536 runtime_pump_max_drain_us=21621 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 5 us | 16000 us | ok |
| key-to-screen p95 | 5742 us | 16000 us | ok |
| input burst p95 | 7 us | 33000 us | ok |
| echo p95 | 5705 us | 16000 us | ok |
| dual-agent echo p95 | 5648 us | 33000 us | ok |
| agent startup input p95 | 81 us | 33000 us | ok |
| paste 10kb elapsed | 37 ms | 50 ms | ok |
| scrollback page p95 | 88 us | 1000 us | ok |
| viewport scroll p95 | 102 us | 1000 us | ok |
| viewport scroll under flood p95 | 466 us | 50000 us | ok |
| screen read under flood p95 | 196 us | 50000 us | ok |
| render frame p95 | 2 us | 1000 us | ok |
| render draw plan p95 | 254 us | 1000 us | ok |
| render geometry plan p95 | 11 us | 1000 us | ok |
| render submission plan p95 | 7 us | 1000 us | ok |
| render commit plan p95 | 814 us | 1000 us | ok |
| render dirty frame p95 | 522 us | 1000 us | ok |
| render cursor move p95 | 49 us | 1000 us | ok |
| render cursor move full frames | 0 frames | 0 frames | ok |
| render cursor move missed moves | 0 moves | 0 moves | ok |
| focus switch p95 | 625 us | 100000 us | ok |
| session create p95 | 62021 us | 100000 us | ok |
| session ready p95 | 96910 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=3 p50_us=3 p95_us=5 max_us=45 bytes_per_sec=826674.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=43
activity_process foreground=cmd.exe foreground_pid=30768 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=30768 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=43 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=38599 max_dispatch_us=18427 total_drain_us=40033 max_drain_us=18463
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=103 min_us=5214 p50_us=5545 p95_us=5742 max_us=22093
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=41556 foreground_cwd=none root=cmd.exe root_pid=41556 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=109 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=163 dispatched_commands=163 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=104 dispatched_background=2 waited_for_response=0 completed_without_wait=163 total_dispatch_us=43057 max_dispatch_us=19609 total_drain_us=44292 max_drain_us=19652
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3703 min_us=2 p50_us=4 p95_us=7 max_us=141
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=29260 foreground_cwd=none root=cmd.exe root_pid=29260 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1592 dispatched_commands=1592 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=576 waited_for_response=0 completed_without_wait=1592 total_dispatch_us=238285 max_dispatch_us=26920 total_drain_us=249231 max_drain_us=26947
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5132 p50_us=5490 p95_us=5705 max_us=21871
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=5272 foreground_cwd=none root=cmd.exe root_pid=5272 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=362 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=164 dispatched_commands=164 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=1 dispatched_background=106 waited_for_response=0 completed_without_wait=164 total_dispatch_us=44736 max_dispatch_us=24658 total_drain_us=46537 max_drain_us=24700
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=15280 lines_per_sec=6544.3 bytes_per_sec=68622.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=76544 foreground_cwd=none root=cmd.exe root_pid=76544 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=64640 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2288 dispatched_commands=2288 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2278 waited_for_response=0 completed_without_wait=2288 total_dispatch_us=854416 max_dispatch_us=28219 total_drain_us=888727 max_drain_us=28261
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=37 bytes_per_sec=272420.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=27592 foreground_cwd=none root=cmd.exe root_pid=27592 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=19 total_dispatch_us=106709 max_dispatch_us=75943 total_drain_us=107024 max_drain_us=75979
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1403 lines_per_sec=7123.0 bytes_per_sec=746902.2
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=23 min_us=52 p50_us=67 p95_us=88 max_us=225
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=55836 foreground_cwd=none root=cmd.exe root_pid=55836 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24971 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=578 dispatched_commands=578 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=233 waited_for_response=0 completed_without_wait=578 total_dispatch_us=96630 max_dispatch_us=21177 total_drain_us=100457 max_drain_us=21207
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1609 lines_per_sec=6213.7 bytes_per_sec=651551.7
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=25 min_us=56 p50_us=74 p95_us=102 max_us=177
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=28732 foreground_cwd=none root=cmd.exe root_pid=28732 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25273 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=948 dispatched_commands=948 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=269 waited_for_response=0 completed_without_wait=948 total_dispatch_us=103391 max_dispatch_us=25027 total_drain_us=108408 max_drain_us=25058
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=145 rows_read=4233 total_ms=865 min_us=38 p50_us=320 p95_us=466 max_us=567
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=67084 foreground_cwd=none root=cmd.exe root_pid=67084 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14581 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=296 viewport_scrolls=145
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=593 dispatched_commands=593 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=436 dispatched_background=148 waited_for_response=0 completed_without_wait=593 total_dispatch_us=95116 max_dispatch_us=28810 total_drain_us=99333 max_drain_us=28846
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5126 p50_us=5517 p95_us=5648 max_us=5748
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=858 combined_lines_per_sec=11651.4 combined_bytes_per_sec=1522251.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=42052 foreground_cwd=none root=cmd.exe root_pid=42052 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=155 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=214 dispatched_commands=214 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=178 waited_for_response=0 completed_without_wait=214 total_dispatch_us=73395 max_dispatch_us=21483 total_drain_us=76070 max_drain_us=21532
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=166 screen_reads=166 elapsed_ms=954 input_min_us=19 input_p50_us=38 input_p95_us=81 input_max_us=134 screen_read_min_us=21 screen_read_p50_us=40 screen_read_p95_us=66 screen_read_max_us=97
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=69828 foreground_cwd=none root=cmd.exe root_pid=69828 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=167 input_bytes=503 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=172 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=513 dispatched_commands=513 dispatched_lifecycle=3 dispatched_input=169 dispatched_render=5 dispatched_screen=167 dispatched_background=169 waited_for_response=0 completed_without_wait=513 total_dispatch_us=125206 max_dispatch_us=61908 total_drain_us=129042 max_drain_us=61920
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=139 total_ms=798 min_us=35 p50_us=133 p95_us=196 max_us=275 text_bytes=104570
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=2424 foreground_cwd=none root=cmd.exe root_pid=2424 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14739 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=145 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=291 dispatched_commands=291 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=140 dispatched_background=142 waited_for_response=0 completed_without_wait=291 total_dispatch_us=61146 max_dispatch_us=20871 total_drain_us=63452 max_drain_us=20905
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=383 full_lines=30 empty_deltas=1000 min_us=1 p50_us=1 p95_us=2 max_us=13 dirty_rounds=50 dirty_lines=1500 dirty_min_us=296 dirty_p50_us=412 dirty_p95_us=522 dirty_max_us=545
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=13784
activity_process foreground=cmd.exe foreground_pid=32012 foreground_cwd=none root=cmd.exe root_pid=32012 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=463 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1270 dispatched_commands=1270 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=9 waited_for_response=0 completed_without_wait=1270 total_dispatch_us=85405 max_dispatch_us=25523 total_drain_us=88185 max_drain_us=25558
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=140 p50_us=180 p95_us=254 max_us=429
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=5667
activity_process foreground=cmd.exe foreground_pid=13968 foreground_cwd=none root=cmd.exe root_pid=13968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=110 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=21 dispatched_commands=21 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=21 total_dispatch_us=41344 max_dispatch_us=20528 total_drain_us=41573 max_drain_us=20573
RENDER_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz
C:\Users\lixd2>echo RENDER_PLAN_BENCH_READY
RENDER_PLAN_BENCH_READY
```

### render geometry plan latency

- Status: ok
- Args: `--bench-render-geometry-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=6 p50_us=8 p95_us=11 max_us=54
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
activity_process foreground=cmd.exe foreground_pid=65052 foreground_cwd=none root=cmd.exe root_pid=65052 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=98 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=42533 max_dispatch_us=24567 total_drain_us=42758 max_drain_us=24613
RENDER_GEOMETRY_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz
C:\Users\lixd2>echo RENDER_GEOMETRY_PLAN_BENCH_READY
RENDER_GEOMETRY_PLAN_BENCH_READY
```

### render submission plan latency

- Status: ok
- Args: `--bench-render-submission-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=5 p95_us=7 max_us=28
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
activity_process foreground=cmd.exe foreground_pid=61524 foreground_cwd=none root=cmd.exe root_pid=61524 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=94 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=39186 max_dispatch_us=20781 total_drain_us=39390 max_drain_us=20818
RENDER_SUBMISSION_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz
C:\Users\lixd2>echo RENDER_SUBMISSION_PLAN_BENCH_READY
RENDER_SUBMISSION_PLAN_BENCH_READY
```

### render commit plan latency

- Status: ok
- Args: `--bench-render-commit-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=363 full_p50_us=537 full_p95_us=814 full_max_us=2619 skip_min_us=2 skip_p50_us=5 skip_p95_us=24 skip_max_us=107
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6116
activity_process foreground=cmd.exe foreground_pid=86760 foreground_cwd=none root=cmd.exe root_pid=86760 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=107 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2006 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2019 dispatched_commands=2019 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=2019 total_dispatch_us=388790 max_dispatch_us=72872 total_drain_us=396500 max_drain_us=72910
RENDER_COMMIT_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz
C:\Users\lixd2>echo RENDER_COMMIT_PLAN_BENCH_READY
RENDER_COMMIT_PLAN_BENCH_READY
```

### render cursor move latency

- Status: ok
- Args: `--bench-render-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=14 p50_us=29 p95_us=49 max_us=194
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 5) raw_bytes=1709
activity_process foreground=cmd.exe foreground_pid=88488 foreground_cwd=none root=cmd.exe root_pid=88488 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=215 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=612 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=818 dispatched_commands=818 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=406 dispatched_background=2 waited_for_response=0 completed_without_wait=818 total_dispatch_us=63618 max_dispatch_us=24860 total_drain_us=70150 max_drain_us=24895
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=236 p50_us=360 p95_us=625 max_us=20953
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=28864 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=28864 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
health_runtime_pump drain_calls=2016 dispatched_commands=2016 dispatched_lifecycle=1007 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=1002 waited_for_response=0 completed_without_wait=2016 total_dispatch_us=461638 max_dispatch_us=21831 total_drain_us=469594 max_drain_us=21879
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=8287 p50_us=10853 p95_us=62021 max_us=69152
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=77692 foreground_cwd=none root=cmd.exe root_pid=77692 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=357125 max_dispatch_us=68473 total_drain_us=357631 max_drain_us=68485
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=31928 p50_us=42473 p95_us=96910 max_us=100241
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=87616 foreground_cwd=none root=cmd.exe root_pid=87616 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=136 dispatched_commands=136 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=88 waited_for_response=0 completed_without_wait=136 total_dispatch_us=626457 max_dispatch_us=80286 total_drain_us=628143 max_drain_us=80294
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=3 p50_us=3 p95_us=5 max_us=45 bytes_per_sec=826674.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=43
render_frame revision=2 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=1
activity_process foreground=cmd.exe foreground_pid=30768 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=30768 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=43 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=38599 max_dispatch_us=18427 total_drain_us=40033 max_drain_us=18463

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=103 min_us=5214 p50_us=5545 p95_us=5742 max_us=22093
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=7545
render_frame revision=368 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=41556 foreground_cwd=none root=cmd.exe root_pid=41556 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=109 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=163 dispatched_commands=163 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=104 dispatched_background=2 waited_for_response=0 completed_without_wait=163 total_dispatch_us=43057 max_dispatch_us=19609 total_drain_us=44292 max_drain_us=19652

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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3703 min_us=2 p50_us=4 p95_us=7 max_us=141
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=29260 foreground_cwd=none root=cmd.exe root_pid=29260 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1592 dispatched_commands=1592 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=576 waited_for_response=0 completed_without_wait=1592 total_dispatch_us=238285 max_dispatch_us=26920 total_drain_us=249231 max_drain_us=26947
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5132 p50_us=5490 p95_us=5705 max_us=21871
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=10545
render_frame revision=362 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=5272 foreground_cwd=none root=cmd.exe root_pid=5272 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=362 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=164 dispatched_commands=164 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=1 dispatched_background=106 waited_for_response=0 completed_without_wait=164 total_dispatch_us=44736 max_dispatch_us=24658 total_drain_us=46537 max_drain_us=24700

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
bench_flood lines=100000 bytes=1048576 elapsed_ms=15280 lines_per_sec=6544.3 bytes_per_sec=68622.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=64640 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=76544 foreground_cwd=none root=cmd.exe root_pid=76544 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=64640 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2288 dispatched_commands=2288 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2278 waited_for_response=0 completed_without_wait=2288 total_dispatch_us=854416 max_dispatch_us=28219 total_drain_us=888727 max_drain_us=28261
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
bench_paste bytes=10240 elapsed_ms=37 bytes_per_sec=272420.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
render_frame revision=14 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=27592 foreground_cwd=none root=cmd.exe root_pid=27592 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=19 total_dispatch_us=106709 max_dispatch_us=75943 total_drain_us=107024 max_drain_us=75979
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1403 lines_per_sec=7123.0 bytes_per_sec=746902.2
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=23 min_us=52 p50_us=67 p95_us=88 max_us=225
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=24971 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=55836 foreground_cwd=none root=cmd.exe root_pid=55836 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24971 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=578 dispatched_commands=578 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=233 waited_for_response=0 completed_without_wait=578 total_dispatch_us=96630 max_dispatch_us=21177 total_drain_us=100457 max_drain_us=21207
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1609 lines_per_sec=6213.7 bytes_per_sec=651551.7
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=25 min_us=56 p50_us=74 p95_us=102 max_us=177
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25607 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=28732 foreground_cwd=none root=cmd.exe root_pid=28732 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25273 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=948 dispatched_commands=948 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=269 waited_for_response=0 completed_without_wait=948 total_dispatch_us=103391 max_dispatch_us=25027 total_drain_us=108408 max_drain_us=25058
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
bench_viewport_scroll_flood lines=5000 scrolls=145 rows_read=4233 total_ms=865 min_us=38 p50_us=320 p95_us=466 max_us=567
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14726 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=67084 foreground_cwd=none root=cmd.exe root_pid=67084 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14581 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=296 viewport_scrolls=145
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=593 dispatched_commands=593 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=436 dispatched_background=148 waited_for_response=0 completed_without_wait=593 total_dispatch_us=95116 max_dispatch_us=28810 total_drain_us=99333 max_drain_us=28846
UNTERM_NEXT_CORE_FLOOD_488
UNTERM_NEXT_CORE_FLOOD_489
UNTERM_NEXT_CORE_FLOOD_490
UNTERM_NEXT_CORE_FLOOD_491
UNTERM_NEXT_CORE_FLOOD_492
UNTERM_NEXT_CORE_FLOOD_493
UNTERM_NEXT_CORE_FLOOD_494
UNTERM_NEXT_CORE_FLOOD_495
UNTERM_NEXT_CORE_FLOOD_496
UNTERM_NEXT_CORE_FLOOD_497
UNTERM_NEXT_CORE_FLOOD_498
UNTERM_NEXT_CORE_FLOOD_499
UNTERM_NEXT_CORE_FLOOD_500
UNTERM_NEXT_CORE_FLOOD_501
UNTERM_NEXT_CORE_FLOOD_502
UNTERM_NEXT_CORE_FLOOD_503
UNTERM_NEXT_CORE_FLOOD_504
UNTERM_NEXT_CORE_FLOOD_505
UNTERM_NEXT_CORE_FLOOD_506
UNTERM_NEXT_CORE_FLOOD_507
UNTERM_NEXT_CORE_FLOOD_508
UNTERM_NEXT_CORE_FLOOD_509
UNTERM_NEXT_CORE_FLOOD_510
UNTERM_NEXT_CORE_FLOOD_511
UNTERM_NEXT_CORE_FLOOD_512
UNTERM_NEXT_CORE_FLOOD_513
UNTERM_NEXT_CORE_FLOOD_514
UNTERM_NEXT_CORE_FLOOD_515
UNTERM_NEXT_CORE_FLOOD_516
UNTERM_NEXT_CORE_FLOOD_517
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5126 p50_us=5517 p95_us=5648 max_us=5748
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=858 combined_lines_per_sec=11651.4 combined_bytes_per_sec=1522251.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
render_frame revision=155 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=42052 foreground_cwd=none root=cmd.exe root_pid=42052 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=155 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=214 dispatched_commands=214 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=178 waited_for_response=0 completed_without_wait=214 total_dispatch_us=73395 max_dispatch_us=21483 total_drain_us=76070 max_drain_us=21532

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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=166 screen_reads=166 elapsed_ms=954 input_min_us=19 input_p50_us=38 input_p95_us=81 input_max_us=134 screen_read_min_us=21 screen_read_p50_us=40 screen_read_p95_us=66 screen_read_max_us=97
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=69828 foreground_cwd=none root=cmd.exe root_pid=69828 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=167 input_bytes=503 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=172 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=513 dispatched_commands=513 dispatched_lifecycle=3 dispatched_input=169 dispatched_render=5 dispatched_screen=167 dispatched_background=169 waited_for_response=0 completed_without_wait=513 total_dispatch_us=125206 max_dispatch_us=61908 total_drain_us=129042 max_drain_us=61920
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=139 total_ms=798 min_us=35 p50_us=133 p95_us=196 max_us=275 text_bytes=104570
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14739 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=2424 foreground_cwd=none root=cmd.exe root_pid=2424 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14739 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=145 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=291 dispatched_commands=291 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=140 dispatched_background=142 waited_for_response=0 completed_without_wait=291 total_dispatch_us=61146 max_dispatch_us=20871 total_drain_us=63452 max_drain_us=20905
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
bench_render_frame rounds=1000 full_us=383 full_lines=30 empty_deltas=1000 min_us=1 p50_us=1 p95_us=2 max_us=13 dirty_rounds=50 dirty_lines=1500 dirty_min_us=296 dirty_p50_us=412 dirty_p95_us=522 dirty_max_us=545
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=13784
render_frame revision=463 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=32012 foreground_cwd=none root=cmd.exe root_pid=32012 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=463 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1270 dispatched_commands=1270 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=9 waited_for_response=0 completed_without_wait=1270 total_dispatch_us=85405 max_dispatch_us=25523 total_drain_us=88185 max_drain_us=25558

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

### render draw plan latency

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=140 p50_us=180 p95_us=254 max_us=429
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=5667
render_frame revision=110 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=13968 foreground_cwd=none root=cmd.exe root_pid=13968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=110 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=21 dispatched_commands=21 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=21 total_dispatch_us=41344 max_dispatch_us=20528 total_drain_us=41573 max_drain_us=20573
RENDER_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz

C:\Users\lixd2>echo RENDER_PLAN_BENCH_READY
RENDER_PLAN_BENCH_READY

C:\Users\lixd2>exit

```

### render geometry plan latency

```text
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=6 p50_us=8 p95_us=11 max_us=54
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
render_frame revision=98 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=65052 foreground_cwd=none root=cmd.exe root_pid=65052 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=98 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=42533 max_dispatch_us=24567 total_drain_us=42758 max_drain_us=24613
RENDER_GEOMETRY_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_GEOMETRY_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz

C:\Users\lixd2>echo RENDER_GEOMETRY_PLAN_BENCH_READY
RENDER_GEOMETRY_PLAN_BENCH_READY

C:\Users\lixd2>exit

```

### render submission plan latency

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=5 p95_us=7 max_us=28
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
render_frame revision=94 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=61524 foreground_cwd=none root=cmd.exe root_pid=61524 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=94 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=39186 max_dispatch_us=20781 total_drain_us=39390 max_drain_us=20818
RENDER_SUBMISSION_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_SUBMISSION_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz

C:\Users\lixd2>echo RENDER_SUBMISSION_PLAN_BENCH_READY
RENDER_SUBMISSION_PLAN_BENCH_READY

C:\Users\lixd2>exit

```

### render commit plan latency

```text
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=363 full_p50_us=537 full_p95_us=814 full_max_us=2619 skip_min_us=2 skip_p50_us=5 skip_p95_us=24 skip_max_us=107
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6116
render_frame revision=107 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=86760 foreground_cwd=none root=cmd.exe root_pid=86760 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=107 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2006 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2019 dispatched_commands=2019 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=2019 total_dispatch_us=388790 max_dispatch_us=72872 total_drain_us=396500 max_drain_us=72910
RENDER_COMMIT_PLAN_BENCH_7 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_8 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_9 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_10 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_11 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_12 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_13 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_14 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_15 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_16 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_17 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_18 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_19 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_20 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_21 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_22 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_23 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_24 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_25 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_26 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_27 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_28 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_29 abcdefghijklmnopqrstuvwxyz
RENDER_COMMIT_PLAN_BENCH_30 abcdefghijklmnopqrstuvwxyz

C:\Users\lixd2>echo RENDER_COMMIT_PLAN_BENCH_READY
RENDER_COMMIT_PLAN_BENCH_READY

C:\Users\lixd2>exit

```

### render cursor move latency

```text
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=14 p50_us=29 p95_us=49 max_us=194
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 5) raw_bytes=1709
render_frame revision=214 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=cmd.exe foreground_pid=88488 foreground_cwd=none root=cmd.exe root_pid=88488 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=215 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=612 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=818 dispatched_commands=818 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=406 dispatched_background=2 waited_for_response=0 completed_without_wait=818 total_dispatch_us=63618 max_dispatch_us=24860 total_drain_us=70150 max_drain_us=24895
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=236 p50_us=360 p95_us=625 max_us=20953
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
render_frame revision=9 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=28864 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=28864 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
health_runtime_pump drain_calls=2016 dispatched_commands=2016 dispatched_lifecycle=1007 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=1002 waited_for_response=0 completed_without_wait=2016 total_dispatch_us=461638 max_dispatch_us=21831 total_drain_us=469594 max_drain_us=21879
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=8287 p50_us=10853 p95_us=62021 max_us=69152
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=cmd.exe foreground_pid=77692 foreground_cwd=none root=cmd.exe root_pid=77692 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=357125 max_dispatch_us=68473 total_drain_us=357631 max_drain_us=68485
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=31928 p50_us=42473 p95_us=96910 max_us=100241
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=87616 foreground_cwd=none root=cmd.exe root_pid=87616 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=136 dispatched_commands=136 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=88 waited_for_response=0 completed_without_wait=136 total_dispatch_us=626457 max_dispatch_us=80286 total_drain_us=628143 max_drain_us=80294
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


# Next-Core Benchmark Report

- Generated: 2026-07-27 12:42:36 +08:00
- Commit: `149ecbb`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=5 render_frame_revision=18 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=18 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| key-to-screen p95 | 5977 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5869 us | 16000 us | ok |
| dual-agent echo p95 | 5699 us | 33000 us | ok |
| agent startup input p95 | 25 us | 33000 us | ok |
| paste 10kb elapsed | 22 ms | 50 ms | ok |
| scrollback page p95 | 69 us | 1000 us | ok |
| viewport scroll p95 | 86 us | 1000 us | ok |
| viewport scroll under flood p95 | 386 us | 50000 us | ok |
| screen read under flood p95 | 153 us | 50000 us | ok |
| render frame p95 | 0 us | 1000 us | ok |
| render draw plan p95 | 179 us | 1000 us | ok |
| render geometry plan p95 | 7 us | 1000 us | ok |
| render dirty frame p95 | 635 us | 1000 us | ok |
| render cursor move p95 | 37 us | 1000 us | ok |
| focus switch p95 | 393 us | 100000 us | ok |
| session create p95 | 47210 us | 100000 us | ok |
| session ready p95 | 79587 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=1 p50_us=1 p95_us=1 max_us=27 bytes_per_sec=2608695.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=45284 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=45284 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5225 p50_us=5637 p95_us=5977 max_us=15712
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=42684 foreground_cwd=none root=cmd.exe root_pid=42684 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=107 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3160 min_us=1 p50_us=1 p95_us=1 max_us=88
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=37268 foreground_cwd=none root=cmd.exe root_pid=37268 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5078 p50_us=5497 p95_us=5869 max_us=16064
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=76780 foreground_cwd=none root=cmd.exe root_pid=76780 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=357 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=15592 lines_per_sec=6413.3 bytes_per_sec=67248.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=1912 foreground_cwd=none root=cmd.exe root_pid=1912 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=71664 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=22 bytes_per_sec=461927.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=46272 foreground_cwd=none root=cmd.exe root_pid=46272 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=13 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1509 lines_per_sec=6624.4 bytes_per_sec=694615.6
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=45 p50_us=53 p95_us=69 max_us=176
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=24260 foreground_cwd=none root=cmd.exe root_pid=24260 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24892 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=340 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1259 lines_per_sec=7938.0 bytes_per_sec=832354.5
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=21 min_us=47 p50_us=61 p95_us=86 max_us=168
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=61752 foreground_cwd=none root=cmd.exe root_pid=61752 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24731 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=340 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=113 rows_read=3282 total_ms=671 min_us=17 p50_us=254 p95_us=386 max_us=453
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=70456 foreground_cwd=none root=cmd.exe root_pid=70456 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14425 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=231 viewport_scrolls=113
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5069 p50_us=5482 p95_us=5699 max_us=5824
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=726 combined_lines_per_sec=13768.1 combined_bytes_per_sec=1798809.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4365
activity_process foreground=cmd.exe foreground_pid=51368 foreground_cwd=none root=cmd.exe root_pid=51368 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=152 output_bytes=4365 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=117 screen_reads=117 elapsed_ms=663 input_min_us=5 input_p50_us=10 input_p95_us=25 input_max_us=68 screen_read_min_us=13 screen_read_p50_us=23 screen_read_p95_us=36 screen_read_max_us=64
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=6556 foreground_cwd=none root=cmd.exe root_pid=6556 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=118 input_bytes=356 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=122 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=113 total_ms=649 min_us=11 p50_us=100 p95_us=153 max_us=230 text_bytes=85417
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=47048 foreground_cwd=none root=cmd.exe root_pid=47048 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14615 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=118 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=468 full_lines=30 empty_deltas=1000 min_us=0 p50_us=0 p95_us=0 max_us=2 dirty_rounds=50 dirty_lines=1500 dirty_min_us=286 dirty_p50_us=341 dirty_p95_us=635 dirty_max_us=720
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
activity_process foreground=cmd.exe foreground_pid=65956 foreground_cwd=none root=cmd.exe root_pid=65956 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=460 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1206 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=133 p50_us=138 p95_us=179 max_us=499
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=5667
activity_process foreground=cmd.exe foreground_pid=84748 foreground_cwd=none root=cmd.exe root_pid=84748 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=95 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=7 max_us=11
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6242
activity_process foreground=cmd.exe foreground_pid=8376 foreground_cwd=none root=cmd.exe root_pid=8376 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=82 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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

### render cursor move latency

- Status: ok
- Args: `--bench-render-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 min_us=11 p50_us=18 p95_us=37 max_us=64
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
activity_process foreground=cmd.exe foreground_pid=11868 foreground_cwd=none root=cmd.exe root_pid=11868 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=211 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=610 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=214 p50_us=258 p95_us=393 max_us=21931
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
activity_process foreground=cmd.exe foreground_pid=86780 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=86780 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=10 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=6580 p50_us=8277 p95_us=47210 max_us=47752
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=27420 foreground_cwd=none root=cmd.exe root_pid=27420 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=29425 p50_us=36749 p95_us=79587 max_us=83982
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=45480 foreground_cwd=none root=cmd.exe root_pid=45480 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=10 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=1 p50_us=1 p95_us=1 max_us=27 bytes_per_sec=2608695.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
render_frame revision=1 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=45284 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=45284 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5225 p50_us=5637 p95_us=5977 max_us=15712
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
render_frame revision=368 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=42684 foreground_cwd=none root=cmd.exe root_pid=42684 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=107 viewport_scrolls=0
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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3160 min_us=1 p50_us=1 p95_us=1 max_us=88
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=cmd.exe foreground_pid=37268 foreground_cwd=none root=cmd.exe root_pid=37268 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5078 p50_us=5497 p95_us=5869 max_us=16064
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
render_frame revision=357 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=76780 foreground_cwd=none root=cmd.exe root_pid=76780 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=357 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=15592 lines_per_sec=6413.3 bytes_per_sec=67248.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=71663 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=1912 foreground_cwd=none root=cmd.exe root_pid=1912 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=71664 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
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
bench_paste bytes=10240 elapsed_ms=22 bytes_per_sec=461927.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=46272 foreground_cwd=none root=cmd.exe root_pid=46272 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=13 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=5 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1509 lines_per_sec=6624.4 bytes_per_sec=694615.6
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=45 p50_us=53 p95_us=69 max_us=176
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=24891 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=24260 foreground_cwd=none root=cmd.exe root_pid=24260 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24892 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=340 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1259 lines_per_sec=7938.0 bytes_per_sec=832354.5
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=21 min_us=47 p50_us=61 p95_us=86 max_us=168
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25064 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=61752 foreground_cwd=none root=cmd.exe root_pid=61752 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24731 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=340 viewport_scrolls=334
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
bench_viewport_scroll_flood lines=5000 scrolls=113 rows_read=3282 total_ms=671 min_us=17 p50_us=254 p95_us=386 max_us=453
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14537 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=70456 foreground_cwd=none root=cmd.exe root_pid=70456 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14425 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=231 viewport_scrolls=113
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_897
UNTERM_NEXT_CORE_FLOOD_898
UNTERM_NEXT_CORE_FLOOD_899
UNTERM_NEXT_CORE_FLOOD_900
UNTERM_NEXT_CORE_FLOOD_901
UNTERM_NEXT_CORE_FLOOD_902
UNTERM_NEXT_CORE_FLOOD_903
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
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5069 p50_us=5482 p95_us=5699 max_us=5824
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=726 combined_lines_per_sec=13768.1 combined_bytes_per_sec=1798809.1
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4365
render_frame revision=152 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=51368 foreground_cwd=none root=cmd.exe root_pid=51368 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=152 output_bytes=4365 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=117 screen_reads=117 elapsed_ms=663 input_min_us=5 input_p50_us=10 input_p95_us=25 input_max_us=68 screen_read_min_us=13 screen_read_p50_us=23 screen_read_p95_us=36 screen_read_max_us=64
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=6556 foreground_cwd=none root=cmd.exe root_pid=6556 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=118 input_bytes=356 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=122 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=113 total_ms=649 min_us=11 p50_us=100 p95_us=153 max_us=230 text_bytes=85417
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14614 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=47048 foreground_cwd=none root=cmd.exe root_pid=47048 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14615 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=118 viewport_scrolls=0
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
bench_render_frame rounds=1000 full_us=468 full_lines=30 empty_deltas=1000 min_us=0 p50_us=0 p95_us=0 max_us=2 dirty_rounds=50 dirty_lines=1500 dirty_min_us=286 dirty_p50_us=341 dirty_p95_us=635 dirty_max_us=720
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
render_frame revision=460 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=65956 foreground_cwd=none root=cmd.exe root_pid=65956 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=460 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1206 viewport_scrolls=0
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

### render draw plan latency

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=133 p50_us=138 p95_us=179 max_us=499
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=5667
render_frame revision=95 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=84748 foreground_cwd=none root=cmd.exe root_pid=84748 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=95 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=7 max_us=11
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6242
render_frame revision=82 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=8376 foreground_cwd=none root=cmd.exe root_pid=8376 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=82 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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

### render cursor move latency

```text
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 min_us=11 p50_us=18 p95_us=37 max_us=64
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
render_frame revision=211 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=11868 foreground_cwd=none root=cmd.exe root_pid=11868 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=211 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=610 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=214 p50_us=258 p95_us=393 max_us=21931
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=155
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=86780 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=86780 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=10 output_bytes=155 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>
```

### session create latency

```text
bench_session_create rounds=20 min_us=6580 p50_us=8277 p95_us=47210 max_us=47752
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=cmd.exe foreground_pid=27420 foreground_cwd=none root=cmd.exe root_pid=27420 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=29425 p50_us=36749 p95_us=79587 max_us=83982
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=9 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=45480 foreground_cwd=none root=cmd.exe root_pid=45480 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=10 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=5 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


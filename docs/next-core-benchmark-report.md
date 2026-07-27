# Next-Core Benchmark Report

- Generated: 2026-07-27 13:02:37 +08:00
- Commit: `0aae4db`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=17 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=17 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=19 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 1 us | 16000 us | ok |
| key-to-screen p95 | 5816 us | 16000 us | ok |
| input burst p95 | 1 us | 33000 us | ok |
| echo p95 | 5723 us | 16000 us | ok |
| dual-agent echo p95 | 5532 us | 33000 us | ok |
| agent startup input p95 | 21 us | 33000 us | ok |
| paste 10kb elapsed | 20 ms | 50 ms | ok |
| scrollback page p95 | 88 us | 1000 us | ok |
| viewport scroll p95 | 65 us | 1000 us | ok |
| viewport scroll under flood p95 | 253 us | 50000 us | ok |
| screen read under flood p95 | 117 us | 50000 us | ok |
| render frame p95 | 347 us | 1000 us | ok |
| render draw plan p95 | 184 us | 1000 us | ok |
| render geometry plan p95 | 7 us | 1000 us | ok |
| render submission plan p95 | 6 us | 1000 us | ok |
| render commit plan p95 | 500 us | 1000 us | ok |
| render dirty frame p95 | 759 us | 1000 us | ok |
| render cursor move p95 | 39 us | 1000 us | ok |
| focus switch p95 | 358 us | 100000 us | ok |
| session create p95 | 12202 us | 100000 us | ok |
| session ready p95 | 42657 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=28 bytes_per_sec=2752293.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=2564 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=2564 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5154 p50_us=5536 p95_us=5816 max_us=16015
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=30148 foreground_cwd=none root=cmd.exe root_pid=30148 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=108 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2758 min_us=1 p50_us=1 p95_us=1 max_us=69
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=53980 foreground_cwd=none root=cmd.exe root_pid=53980 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5051 p50_us=5401 p95_us=5723 max_us=15862
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
activity_process foreground=cmd.exe foreground_pid=36264 foreground_cwd=none root=cmd.exe root_pid=36264 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=360 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=26452 lines_per_sec=3780.3 bytes_per_sec=39639.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=46872 foreground_cwd=none root=cmd.exe root_pid=46872 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=163307 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=20 bytes_per_sec=495497.5
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=66144 foreground_cwd=none root=cmd.exe root_pid=66144 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=16 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1076 lines_per_sec=9286.5 bytes_per_sec=973762.1
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=23 min_us=48 p50_us=71 p95_us=88 max_us=109
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=86592 foreground_cwd=none root=cmd.exe root_pid=86592 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24950 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1109 lines_per_sec=9012.1 bytes_per_sec=944983.6
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=47 p50_us=51 p95_us=65 max_us=150
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=12880 foreground_cwd=none root=cmd.exe root_pid=12880 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24928 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=95 rows_read=2763 total_ms=542 min_us=12 p50_us=199 p95_us=253 max_us=304
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=86768 foreground_cwd=none root=cmd.exe root_pid=86768 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14524 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=196 viewport_scrolls=95
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5032 p50_us=5290 p95_us=5532 max_us=11011
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=681 combined_lines_per_sec=14663.1 combined_bytes_per_sec=1915733.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4365
activity_process foreground=cmd.exe foreground_pid=26564 foreground_cwd=none root=cmd.exe root_pid=26564 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=152 output_bytes=4365 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=108 screen_reads=108 elapsed_ms=600 input_min_us=4 input_p50_us=7 input_p95_us=21 input_max_us=61 screen_read_min_us=11 screen_read_p50_us=18 screen_read_p95_us=29 screen_read_max_us=54
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=46732 foreground_cwd=none root=cmd.exe root_pid=46732 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=109 input_bytes=329 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=114 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=100 total_ms=556 min_us=9 p50_us=76 p95_us=117 max_us=205 text_bytes=75353
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=45700 foreground_cwd=none root=cmd.exe root_pid=45700 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14665 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=106 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=468 full_lines=30 empty_deltas=0 min_us=237 p50_us=246 p95_us=347 max_us=741 dirty_rounds=50 dirty_lines=1500 dirty_min_us=331 dirty_p50_us=432 dirty_p95_us=759 dirty_max_us=859
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
activity_process foreground=cmd.exe foreground_pid=18176 foreground_cwd=none root=cmd.exe root_pid=18176 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=459 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=133 p50_us=140 p95_us=184 max_us=298
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=5667
activity_process foreground=cmd.exe foreground_pid=39392 foreground_cwd=none root=cmd.exe root_pid=39392 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=99 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=7 max_us=16
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
activity_process foreground=cmd.exe foreground_pid=18044 foreground_cwd=none root=cmd.exe root_pid=18044 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=85 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
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

### render submission plan latency

- Status: ok
- Args: `--bench-render-submission-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=4 p95_us=6 max_us=67
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
activity_process foreground=cmd.exe foreground_pid=5416 foreground_cwd=none root=cmd.exe root_pid=5416 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=88 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=382 full_p50_us=406 full_p95_us=500 full_max_us=853 skip_min_us=0 skip_p50_us=1 skip_p95_us=2 skip_max_us=23
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6116
activity_process foreground=cmd.exe foreground_pid=20032 foreground_cwd=none root=cmd.exe root_pid=20032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=85 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2006 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 min_us=11 p50_us=22 p95_us=39 max_us=58
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
activity_process foreground=cmd.exe foreground_pid=24944 foreground_cwd=none root=cmd.exe root_pid=24944 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=212 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=611 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=214 p50_us=252 p95_us=358 max_us=16745
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=34064 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=34064 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=5025 p50_us=8189 p95_us=12202 max_us=16845
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=73008 foreground_cwd=none root=cmd.exe root_pid=73008 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=27935 p50_us=37902 p95_us=42657 max_us=83008
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=29344 foreground_cwd=none root=cmd.exe root_pid=29344 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=0 p50_us=1 p95_us=1 max_us=28 bytes_per_sec=2752293.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
render_frame revision=3 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=2564 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=2564 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5154 p50_us=5536 p95_us=5816 max_us=16015
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
render_frame revision=368 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=30148 foreground_cwd=none root=cmd.exe root_pid=30148 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=108 viewport_scrolls=0
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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=2758 min_us=1 p50_us=1 p95_us=1 max_us=69
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
render_frame revision=13 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=53980 foreground_cwd=none root=cmd.exe root_pid=53980 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5051 p50_us=5401 p95_us=5723 max_us=15862
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10545
render_frame revision=360 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=36264 foreground_cwd=none root=cmd.exe root_pid=36264 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=360 output_bytes=10545 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=26452 lines_per_sec=3780.3 bytes_per_sec=39639.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=163307 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=46872 foreground_cwd=none root=cmd.exe root_pid=46872 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=163307 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
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
bench_paste bytes=10240 elapsed_ms=20 bytes_per_sec=495497.5
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
render_frame revision=16 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=66144 foreground_cwd=none root=cmd.exe root_pid=66144 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=16 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1076 lines_per_sec=9286.5 bytes_per_sec=973762.1
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=23 min_us=48 p50_us=71 p95_us=88 max_us=109
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=24949 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=86592 foreground_cwd=none root=cmd.exe root_pid=86592 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24950 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1109 lines_per_sec=9012.1 bytes_per_sec=944983.6
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=47 p50_us=51 p95_us=65 max_us=150
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25261 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=12880 foreground_cwd=none root=cmd.exe root_pid=12880 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24928 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
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
bench_viewport_scroll_flood lines=5000 scrolls=95 rows_read=2763 total_ms=542 min_us=12 p50_us=199 p95_us=253 max_us=304
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14618 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=86768 foreground_cwd=none root=cmd.exe root_pid=86768 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14524 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=196 viewport_scrolls=95
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
UNTERM_NEXT_CORE_FLOOD_585
UNTERM_NEXT_CORE_FLOOD_586
UNTERM_NEXT_CORE_FLOOD_587
UNTERM_NEXT_CORE_FLOOD_588
UNTERM_NEXT_CORE_FLOOD_589
UNTERM_NEXT_CORE_FLOOD_590
UNTERM_NEXT_CORE_FLOOD_591
UNTERM_NEXT_CORE_FLOOD_592
UNTERM_NEXT_CORE_FLOOD_593
UNTERM_NEXT_CORE_FLOOD_594
UNTERM_NEXT_CORE_FLOOD_595
UNTERM_NEXT_CORE_FLOOD_596
UNTERM_NEXT_CORE_FLOOD_597
UNTERM_NEXT_CORE_FLOOD_598
UNTERM_NEXT_CORE_FLOOD_599
UNTERM_NEXT_CORE_FLOOD_600
UNTERM_NEXT_CORE_FLOOD_601
UNTERM_NEXT_CORE_FLOOD_602
UNTERM_NEXT_CORE_FLOOD_603
UNTERM_NEXT_CORE_FLOOD_604
UNTERM_NEXT_CORE_FLOOD_605
UNTERM_NEXT_CORE_FLOOD_606
UNTERM_NEXT_CORE_FLOOD_607
UNTERM_NEXT_CORE_FLOOD_608
UNTERM_NEXT_CORE_FLOOD_609
UNTERM_NEXT_CORE_FLOOD_610
UNTERM_NEXT_CORE_FLOOD_611
UNTERM_NEXT_CORE_FLOOD_612
UNTERM_NEXT_CORE_FLOOD_613
UNTERM_NEXT_CORE_FLOOD_614
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5032 p50_us=5290 p95_us=5532 max_us=11011
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=681 combined_lines_per_sec=14663.1 combined_bytes_per_sec=1915733.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4365
render_frame revision=151 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=26564 foreground_cwd=none root=cmd.exe root_pid=26564 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=152 output_bytes=4365 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=108 screen_reads=108 elapsed_ms=600 input_min_us=4 input_p50_us=7 input_p95_us=21 input_max_us=61 screen_read_min_us=11 screen_read_p50_us=18 screen_read_p95_us=29 screen_read_max_us=54
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=46732 foreground_cwd=none root=cmd.exe root_pid=46732 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=109 input_bytes=329 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=114 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=100 total_ms=556 min_us=9 p50_us=76 p95_us=117 max_us=205 text_bytes=75353
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14664 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=45700 foreground_cwd=none root=cmd.exe root_pid=45700 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14665 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=106 viewport_scrolls=0
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
bench_render_frame rounds=1000 full_us=468 full_lines=30 empty_deltas=0 min_us=237 p50_us=246 p95_us=347 max_us=741 dirty_rounds=50 dirty_lines=1500 dirty_min_us=331 dirty_p50_us=432 dirty_p95_us=759 dirty_max_us=859
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
render_frame revision=459 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=18176 foreground_cwd=none root=cmd.exe root_pid=18176 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=459 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
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
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=133 p50_us=140 p95_us=184 max_us=298
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=5667
render_frame revision=98 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=39392 foreground_cwd=none root=cmd.exe root_pid=39392 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=99 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=7 max_us=16
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
render_frame revision=84 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=18044 foreground_cwd=none root=cmd.exe root_pid=18044 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=85 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
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

### render submission plan latency

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=4 p95_us=6 max_us=67
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
render_frame revision=87 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=5416 foreground_cwd=none root=cmd.exe root_pid=5416 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=88 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=382 full_p50_us=406 full_p95_us=500 full_max_us=853 skip_min_us=0 skip_p50_us=1 skip_p95_us=2 skip_max_us=23
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6116
render_frame revision=85 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=20032 foreground_cwd=none root=cmd.exe root_pid=20032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=85 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2006 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
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
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 min_us=11 p50_us=22 p95_us=39 max_us=58
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
render_frame revision=212 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=24944 foreground_cwd=none root=cmd.exe root_pid=24944 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=212 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=611 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=214 p50_us=252 p95_us=358 max_us=16745
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
render_frame revision=9 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=34064 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=34064 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=5025 p50_us=8189 p95_us=12202 max_us=16845
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=255
render_frame revision=13 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=73008 foreground_cwd=none root=cmd.exe root_pid=73008 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=27935 p50_us=37902 p95_us=42657 max_us=83008
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=29344 foreground_cwd=none root=cmd.exe root_pid=29344 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


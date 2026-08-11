# Next-Core Benchmark Report

- Generated: 2026-08-12 03:37:21 +08:00
- Commit: `44757f9d`
- Machine: `ALEX-PC01`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=253 foreground=cmd.exe cwd=C:\Users\Alex\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=4 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=4 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=19 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 runtime_pump_dispatches=10 runtime_pump_lanes=lifecycle:1,input:1,render:5,screen:1,background:2 runtime_pump_waited=0 runtime_pump_completed_without_wait=10 runtime_pump_max_dispatch_us=14706 runtime_pump_max_drain_us=14724 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 2 us | 16000 us | ok |
| key-to-screen p95 | 6078 us | 16000 us | ok |
| input burst p95 | 2 us | 33000 us | ok |
| echo p95 | 5787 us | 16000 us | ok |
| dual-agent echo p95 | 15961 us | 33000 us | ok |
| agent startup input p95 | 32 us | 33000 us | ok |
| paste 10kb elapsed | 19 ms | 50 ms | ok |
| paste under flood elapsed | 15 ms | 50 ms | ok |
| paste under flood marker misses | 0 misses | 0 misses | ok |
| scrollback page p95 | 58 us | 1000 us | ok |
| viewport scroll p95 | 55 us | 1000 us | ok |
| viewport page cycle p95 | 55 us | 1000 us | ok |
| viewport page cycle boundary misses | 0 misses | 0 misses | ok |
| viewport page cycle missed pages | 0 pages | 0 pages | ok |
| viewport scroll under flood p95 | 251 us | 50000 us | ok |
| screen read under flood p95 | 111 us | 50000 us | ok |
| render frame p95 | 2 us | 1000 us | ok |
| render draw plan p95 | 146 us | 1000 us | ok |
| render geometry plan p95 | 5 us | 1000 us | ok |
| render submission plan p95 | 4 us | 1000 us | ok |
| render commit plan p95 | 433 us | 1000 us | ok |
| render dirty frame p95 | 668 us | 1000 us | ok |
| render cursor move p95 | 155 us | 1000 us | ok |
| render cursor move full frames | 0 frames | 0 frames | ok |
| render cursor move missed moves | 0 moves | 0 moves | ok |
| render application cursor move p95 | 158 us | 1000 us | ok |
| render application cursor move full frames | 0 frames | 0 frames | ok |
| render application cursor move missed moves | 0 moves | 0 moves | ok |
| focus switch p95 | 787 us | 100000 us | ok |
| focus switch active misses | 0 misses | 0 misses | ok |
| focus switch missing sessions | 0 misses | 0 misses | ok |
| focus switch duplicate sessions | 0 misses | 0 misses | ok |
| session create p95 | 6355 us | 100000 us | ok |
| session ready p95 | 35115 us | 100000 us | ok |
| first session ready elapsed | 36 ms | 1000 ms | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=2 p50_us=2 p95_us=2 max_us=34 bytes_per_sec=1459854.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=61
activity_process foreground=cmd.exe foreground_pid=17316 foreground_cwd=C:\Users\Alex\ root=cmd.exe root_pid=17316 root_cwd=C:\Users\Alex\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=61 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=18576 max_dispatch_us=9752 total_drain_us=19043 max_drain_us=9780
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=200 snapshots=415 min_us=5102 p50_us=5602 p95_us=6078 max_us=22068
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=23187
activity_process foreground=cmd.exe foreground_pid=37952 foreground_cwd=none root=cmd.exe root_pid=37952 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=2605 output_chunks=655 output_bytes=23187 paste_count=0 paste_text_bytes=0 screen_reads=421 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=625 dispatched_commands=625 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=416 dispatched_background=2 waited_for_response=0 completed_without_wait=625 total_dispatch_us=47108 max_dispatch_us=9130 total_drain_us=51539 max_drain_us=9160
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=1815 min_us=2 p50_us=2 p95_us=2 max_us=41
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=7892 foreground_cwd=none root=cmd.exe root_pid=7892 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1329 dispatched_commands=1329 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=313 waited_for_response=0 completed_without_wait=1329 total_dispatch_us=73922 max_dispatch_us=10912 total_drain_us=76653 max_drain_us=10942
```

### echo latency

- Status: ok
- Args: `--bench-echo 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=200 min_us=5047 p50_us=5551 p95_us=5787 max_us=32152
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=35006
activity_process foreground=cmd.exe foreground_pid=38480 foreground_cwd=none root=cmd.exe root_pid=38480 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=6605 output_chunks=649 output_bytes=35006 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=630 dispatched_commands=630 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=1 dispatched_background=422 waited_for_response=0 completed_without_wait=630 total_dispatch_us=22698 max_dispatch_us=9414 total_drain_us=28440 max_drain_us=9444
UNTERM_NEXT_CORE_BENCH_0190
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0191
UNTERM_NEXT_CORE_BENCH_0191
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0192
UNTERM_NEXT_CORE_BENCH_0192
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0193
UNTERM_NEXT_CORE_BENCH_0193
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0194
UNTERM_NEXT_CORE_BENCH_0194
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0195
UNTERM_NEXT_CORE_BENCH_0195
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0196
UNTERM_NEXT_CORE_BENCH_0196
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0197
UNTERM_NEXT_CORE_BENCH_0197
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0198
UNTERM_NEXT_CORE_BENCH_0198
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0199
UNTERM_NEXT_CORE_BENCH_0199
```

### output flood

- Status: ok
- Args: `--bench-flood-lines 100000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=100000 bytes=1048576 elapsed_ms=7217 lines_per_sec=13855.4 bytes_per_sec=145284.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=35020 foreground_cwd=none root=cmd.exe root_pid=35020 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=82219 output_bytes=9883983 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1201 dispatched_commands=1201 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=1191 waited_for_response=0 completed_without_wait=1201 total_dispatch_us=226343 max_dispatch_us=10161 total_drain_us=234225 max_drain_us=10187
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=19 bytes_per_sec=522134.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(100, 29) raw_bytes=11698
activity_process foreground=cmd.exe foreground_pid=37472 foreground_cwd=none root=cmd.exe root_pid=37472 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=7 output_bytes=11698 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=17 total_dispatch_us=44135 max_dispatch_us=14513 total_drain_us=44275 max_drain_us=14529
```

### paste under output flood

- Status: ok
- Args: `--bench-paste-under-flood-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste_under_flood bytes=10240 flood_lines=5000 flood_bytes=482102 elapsed_ms=15 write_ms=4 marker_misses=0 background_elapsed_ms=407
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=11740
activity_process foreground=cmd.exe foreground_pid=11564 foreground_cwd=none root=cmd.exe root_pid=11564 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10334 output_chunks=11 output_bytes=11740 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=92 dispatched_commands=92 dispatched_lifecycle=3 dispatched_input=5 dispatched_render=5 dispatched_screen=1 dispatched_background=78 waited_for_response=0 completed_without_wait=92 total_dispatch_us=29109 max_dispatch_us=9096 total_drain_us=29674 max_drain_us=9121
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=967443 elapsed_ms=801 lines_per_sec=12483.1 bytes_per_sec=1207664.9
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=17 min_us=46 p50_us=49 p95_us=58 max_us=185
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967449
activity_process foreground=cmd.exe foreground_pid=7832 foreground_cwd=none root=cmd.exe root_pid=7832 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9994 output_bytes=967449 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=490 dispatched_commands=490 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=145 waited_for_response=0 completed_without_wait=490 total_dispatch_us=38985 max_dispatch_us=9186 total_drain_us=40354 max_drain_us=9214
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=967332 elapsed_ms=806 lines_per_sec=12394.5 bytes_per_sec=1198962.1
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=51 p50_us=53 p95_us=55 max_us=233
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 10008) raw_bytes=967353
activity_process foreground=cmd.exe foreground_pid=37228 foreground_cwd=none root=cmd.exe root_pid=37228 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9985 output_bytes=967353 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=825 dispatched_commands=825 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=146 waited_for_response=0 completed_without_wait=825 total_dispatch_us=41888 max_dispatch_us=9950 total_drain_us=43522 max_drain_us=9977
```

### viewport page cycle

- Status: ok
- Args: `--bench-viewport-page-cycle-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=967364 elapsed_ms=799 lines_per_sec=12500.6 bytes_per_sec=1209258.7
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=73 min_us=46 p50_us=50 p95_us=55 max_us=209
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967385
activity_process foreground=cmd.exe foreground_pid=36544 foreground_cwd=none root=cmd.exe root_pid=36544 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9980 output_bytes=967385 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2269 dispatched_commands=2269 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=145 waited_for_response=0 completed_without_wait=2269 total_dispatch_us=90787 max_dispatch_us=8922 total_drain_us=94045 max_drain_us=8948
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=77 rows_read=2223 total_ms=427 min_us=31 p50_us=205 p95_us=251 max_us=424
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1822) raw_bytes=481995
activity_process foreground=cmd.exe foreground_pid=32612 foreground_cwd=none root=cmd.exe root_pid=32612 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4981 output_bytes=481995 paste_count=0 paste_text_bytes=0 screen_reads=160 viewport_scrolls=77
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=321 dispatched_commands=321 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=232 dispatched_background=80 waited_for_response=0 completed_without_wait=321 total_dispatch_us=32173 max_dispatch_us=9129 total_drain_us=33235 max_drain_us=9155
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5105 p50_us=5540 p95_us=15961 max_us=16191
bench_dual_agents lines_per_agent=5000 total_bytes=964236 elapsed_ms=452 combined_lines_per_sec=22078.2 combined_bytes_per_sec=2128858.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2967
activity_process foreground=cmd.exe foreground_pid=23640 foreground_cwd=none root=cmd.exe root_pid=23640 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=52 output_bytes=2967 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=146 dispatched_commands=146 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=110 waited_for_response=0 completed_without_wait=146 total_dispatch_us=31998 max_dispatch_us=9281 total_drain_us=32745 max_drain_us=9309
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0011
UNTERM_NEXT_CORE_BENCH_0011
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0012
UNTERM_NEXT_CORE_BENCH_0012
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0013
UNTERM_NEXT_CORE_BENCH_0013
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0014
UNTERM_NEXT_CORE_BENCH_0014
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0015
UNTERM_NEXT_CORE_BENCH_0015
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0016
UNTERM_NEXT_CORE_BENCH_0016
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0017
UNTERM_NEXT_CORE_BENCH_0017
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0018
UNTERM_NEXT_CORE_BENCH_0018
C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0019
UNTERM_NEXT_CORE_BENCH_0019
```

### agent startup stall

- Status: ok
- Args: `--bench-agent-startup-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_agent_startup_stall lines=5000 bytes=481910 input_writes=77 screen_reads=77 elapsed_ms=418 input_min_us=4 input_p50_us=14 input_p95_us=32 input_max_us=65 screen_read_min_us=8 screen_read_p50_us=16 screen_read_p95_us=25 screen_read_max_us=72
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=38240 foreground_cwd=none root=cmd.exe root_pid=38240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=78 input_bytes=236 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=83 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=246 dispatched_commands=246 dispatched_lifecycle=3 dispatched_input=80 dispatched_render=5 dispatched_screen=78 dispatched_background=80 waited_for_response=0 completed_without_wait=246 total_dispatch_us=26371 max_dispatch_us=9886 total_drain_us=27247 max_drain_us=9914
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=76 total_ms=415 min_us=24 p50_us=72 p95_us=111 max_us=242 text_bytes=58615
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=481802
activity_process foreground=cmd.exe foreground_pid=1832 foreground_cwd=none root=cmd.exe root_pid=1832 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4981 output_bytes=481802 paste_count=0 paste_text_bytes=0 screen_reads=82 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=165 dispatched_commands=165 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=77 dispatched_background=79 waited_for_response=0 completed_without_wait=165 total_dispatch_us=23217 max_dispatch_us=9153 total_drain_us=23903 max_drain_us=9181
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=343 full_lines=30 empty_deltas=1000 min_us=1 p50_us=1 p95_us=2 max_us=11 dirty_rounds=50 dirty_lines=1500 dirty_min_us=266 dirty_p50_us=427 dirty_p95_us=668 dirty_max_us=1006
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=9753
activity_process foreground=cmd.exe foreground_pid=31032 foreground_cwd=none root=cmd.exe root_pid=31032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=176 output_bytes=9753 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1268 dispatched_commands=1268 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=7 waited_for_response=0 completed_without_wait=1268 total_dispatch_us=64532 max_dispatch_us=9579 total_drain_us=67027 max_drain_us=9607
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=55 cell_runs=30 min_us=125 p50_us=131 p95_us=146 max_us=329
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2386
activity_process foreground=cmd.exe foreground_pid=37724 foreground_cwd=none root=cmd.exe root_pid=37724 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=8 output_bytes=2386 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=19 total_dispatch_us=16779 max_dispatch_us=8954 total_drain_us=16902 max_drain_us=8981
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
C:\Users\Alex>echo RENDER_PLAN_BENCH_READY
RENDER_PLAN_BENCH_READY
```

### render geometry plan latency

- Status: ok
- Args: `--bench-render-geometry-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_geometry_plan rounds=1000 glyph_runs=55 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=5 max_us=73
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2834
activity_process foreground=cmd.exe foreground_pid=37968 foreground_cwd=none root=cmd.exe root_pid=37968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=7 output_bytes=2834 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=18 dispatched_commands=18 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=18 total_dispatch_us=17401 max_dispatch_us=9197 total_drain_us=17514 max_drain_us=9223
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
C:\Users\Alex>echo RENDER_GEOMETRY_PLAN_BENCH_READY
RENDER_GEOMETRY_PLAN_BENCH_READY
```

### render submission plan latency

- Status: ok
- Args: `--bench-render-submission-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=4 p95_us=4 max_us=14
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2914
activity_process foreground=cmd.exe foreground_pid=24916 foreground_cwd=none root=cmd.exe root_pid=24916 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=8 output_bytes=2914 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=16492 max_dispatch_us=8840 total_drain_us=16617 max_drain_us=8866
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
C:\Users\Alex>echo RENDER_SUBMISSION_PLAN_BENCH_READY
RENDER_SUBMISSION_PLAN_BENCH_READY
```

### render commit plan latency

- Status: ok
- Args: `--bench-render-commit-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=324 full_p50_us=345 full_p95_us=433 full_max_us=737 skip_min_us=2 skip_p50_us=3 skip_p95_us=10 skip_max_us=59
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2659
activity_process foreground=cmd.exe foreground_pid=32968 foreground_cwd=none root=cmd.exe root_pid=32968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=8 output_bytes=2659 paste_count=0 paste_text_bytes=0 screen_reads=2011 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2022 dispatched_commands=2022 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=6 dispatched_background=7 waited_for_response=0 completed_without_wait=2022 total_dispatch_us=215041 max_dispatch_us=8998 total_drain_us=218803 max_drain_us=9025
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
C:\Users\Alex>echo RENDER_COMMIT_PLAN_BENCH_READY
RENDER_COMMIT_PLAN_BENCH_READY
```

### render cursor move latency

- Status: ok
- Args: `--bench-render-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_cursor_move rounds=200 snapshots=768 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=14 p50_us=52 p95_us=155 max_us=581
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(42, 3) raw_bytes=714
activity_process foreground=cmd.exe foreground_pid=32172 foreground_cwd=none root=cmd.exe root_pid=32172 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=204 output_bytes=714 paste_count=0 paste_text_bytes=0 screen_reads=980 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1186 dispatched_commands=1186 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=774 dispatched_background=2 waited_for_response=0 completed_without_wait=1186 total_dispatch_us=80064 max_dispatch_us=9095 total_drain_us=94171 max_drain_us=9124
```

### render application cursor move latency

- Status: ok
- Args: `--bench-render-application-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_application_cursor_move rounds=200 snapshots=769 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=13 p50_us=67 p95_us=158 max_us=550
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=752
activity_process foreground=cmd.exe foreground_pid=35332 foreground_cwd=none root=cmd.exe root_pid=35332 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=205 output_bytes=752 paste_count=0 paste_text_bytes=0 screen_reads=982 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1188 dispatched_commands=1188 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=776 dispatched_background=2 waited_for_response=0 completed_without_wait=1188 total_dispatch_us=89456 max_dispatch_us=9681 total_drain_us=107560 max_drain_us=9709
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 background_bytes=1048515 background_elapsed_ms=1576 active_misses=0 missing_sessions=0 duplicate_sessions=0 min_us=358 p50_us=447 p95_us=787 max_us=9725
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=28468 foreground_cwd=none root=cmd.exe root_pid=28468 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=3200 dispatched_commands=3200 dispatched_lifecycle=1007 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2184 waited_for_response=0 completed_without_wait=3200 total_dispatch_us=534703 max_dispatch_us=9782 total_drain_us=544259 max_drain_us=9809
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=4760 p50_us=5072 p95_us=6355 max_us=6523
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=17752 foreground_cwd=none root=cmd.exe root_pid=17752 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=118128 max_dispatch_us=9392 total_drain_us=118452 max_drain_us=9438
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=25967 p50_us=30279 p95_us=35115 max_us=35307
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=36148 foreground_cwd=none root=cmd.exe root_pid=36148 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=154 dispatched_commands=154 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=106 waited_for_response=0 completed_without_wait=154 total_dispatch_us=196569 max_dispatch_us=12254 total_drain_us=198254 max_drain_us=12263
```

### first session ready

- Status: ok
- Args: `--bench-first-session-ready --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_first_session_ready elapsed_ms=36 create_us=9427 visible_bytes=108
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(14, 3) raw_bytes=186
activity_process foreground=cmd.exe foreground_pid=36188 foreground_cwd=none root=cmd.exe root_pid=36188 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=4 output_bytes=186 paste_count=0 paste_text_bytes=0 screen_reads=12 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=16 dispatched_commands=16 dispatched_lifecycle=1 dispatched_input=1 dispatched_render=5 dispatched_screen=7 dispatched_background=2 waited_for_response=0 completed_without_wait=16 total_dispatch_us=16805 max_dispatch_us=9078 total_drain_us=16941 max_drain_us=9106
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=2 p50_us=2 p95_us=2 max_us=34 bytes_per_sec=1459854.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=61
render_frame revision=1 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=17316 foreground_cwd=C:\Users\Alex\ root=cmd.exe root_pid=17316 root_cwd=C:\Users\Alex\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=61 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=18576 max_dispatch_us=9752 total_drain_us=19043 max_drain_us=9780

```

### key-to-screen latency

```text
bench_key_to_screen rounds=200 snapshots=415 min_us=5102 p50_us=5602 p95_us=6078 max_us=22068
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=23187
render_frame revision=655 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37952 foreground_cwd=none root=cmd.exe root_pid=37952 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=2605 output_chunks=655 output_bytes=23187 paste_count=0 paste_text_bytes=0 screen_reads=421 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=625 dispatched_commands=625 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=416 dispatched_background=2 waited_for_response=0 completed_without_wait=625 total_dispatch_us=47108 max_dispatch_us=9130 total_drain_us=51539 max_drain_us=9160

C:\Users\Alex>echo KTS0191
KTS0191

C:\Users\Alex>echo KTS0192
KTS0192

C:\Users\Alex>echo KTS0193
KTS0193

C:\Users\Alex>echo KTS0194
KTS0194

C:\Users\Alex>echo KTS0195
KTS0195

C:\Users\Alex>echo KTS0196
KTS0196

C:\Users\Alex>echo KTS0197
KTS0197

C:\Users\Alex>echo KTS0198
KTS0198

C:\Users\Alex>echo KTS0199
KTS0199

C:\Users\Alex>exit

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=1815 min_us=2 p50_us=2 p95_us=2 max_us=41
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=7892 foreground_cwd=none root=cmd.exe root_pid=7892 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1329 dispatched_commands=1329 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=313 waited_for_response=0 completed_without_wait=1329 total_dispatch_us=73922 max_dispatch_us=10912 total_drain_us=76653 max_drain_us=10942
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### echo latency

```text
bench_echo rounds=200 min_us=5047 p50_us=5551 p95_us=5787 max_us=32152
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=35006
render_frame revision=649 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=38480 foreground_cwd=none root=cmd.exe root_pid=38480 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=6605 output_chunks=649 output_bytes=35006 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=630 dispatched_commands=630 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=1 dispatched_background=422 waited_for_response=0 completed_without_wait=630 total_dispatch_us=22698 max_dispatch_us=9414 total_drain_us=28440 max_drain_us=9444
UNTERM_NEXT_CORE_BENCH_0190

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0191
UNTERM_NEXT_CORE_BENCH_0191

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0192
UNTERM_NEXT_CORE_BENCH_0192

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0193
UNTERM_NEXT_CORE_BENCH_0193

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0194
UNTERM_NEXT_CORE_BENCH_0194

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0195
UNTERM_NEXT_CORE_BENCH_0195

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0196
UNTERM_NEXT_CORE_BENCH_0196

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0197
UNTERM_NEXT_CORE_BENCH_0197

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0198
UNTERM_NEXT_CORE_BENCH_0198

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0199
UNTERM_NEXT_CORE_BENCH_0199

C:\Users\Alex>exit
```

### output flood

```text
bench_flood lines=100000 bytes=1048576 elapsed_ms=7217 lines_per_sec=13855.4 bytes_per_sec=145284.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=82219 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=35020 foreground_cwd=none root=cmd.exe root_pid=35020 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=82219 output_bytes=9883983 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1201 dispatched_commands=1201 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=1191 waited_for_response=0 completed_without_wait=1201 total_dispatch_us=226343 max_dispatch_us=10161 total_drain_us=234225 max_drain_us=10187
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

C:\Users\Alex>echo UNTERM_NEXT_CORE_FLOOD_DONE_100000_1
UNTERM_NEXT_CORE_FLOOD_DONE_100000_1

C:\Users\Alex>exit

```

### paste 10kb

```text
bench_paste bytes=10240 elapsed_ms=19 bytes_per_sec=522134.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(100, 29) raw_bytes=11698
render_frame revision=7 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37472 foreground_cwd=none root=cmd.exe root_pid=37472 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=7 output_bytes=11698 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=17 total_dispatch_us=44135 max_dispatch_us=14513 total_drain_us=44275 max_drain_us=14529
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

C:\Users\Alex>输入行太长。

C:\Users\Alex>exit

```

### paste under output flood

```text
bench_paste_under_flood bytes=10240 flood_lines=5000 flood_bytes=482102 elapsed_ms=15 write_ms=4 marker_misses=0 background_elapsed_ms=407
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=11740
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=11564 foreground_cwd=none root=cmd.exe root_pid=11564 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10334 output_chunks=11 output_bytes=11740 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=92 dispatched_commands=92 dispatched_lifecycle=3 dispatched_input=5 dispatched_render=5 dispatched_screen=1 dispatched_background=78 waited_for_response=0 completed_without_wait=92 total_dispatch_us=29109 max_dispatch_us=9096 total_drain_us=29674 max_drain_us=9121
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
UNTERM_NEXT_CORE_PASTE_FLOOD_DONE_10240

C:\Users\Alex>输入行太长。

C:\Users\Alex>exit

```

### scrollback paging

```text
bench_flood lines=10000 bytes=967443 elapsed_ms=801 lines_per_sec=12483.1 bytes_per_sec=1207664.9
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=17 min_us=46 p50_us=49 p95_us=58 max_us=185
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967449
render_frame revision=9994 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=7832 foreground_cwd=none root=cmd.exe root_pid=7832 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9994 output_bytes=967449 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=490 dispatched_commands=490 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=145 waited_for_response=0 completed_without_wait=490 total_dispatch_us=38985 max_dispatch_us=9186 total_drain_us=40354 max_drain_us=9214
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

C:\Users\Alex>echo UNTERM_NEXT_CORE_FLOOD_DONE_10000_1
UNTERM_NEXT_CORE_FLOOD_DONE_10000_1

C:\Users\Alex>exit

```

### viewport scroll paging

```text
bench_flood lines=10000 bytes=967332 elapsed_ms=806 lines_per_sec=12394.5 bytes_per_sec=1198962.1
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=51 p50_us=53 p95_us=55 max_us=233
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 10008) raw_bytes=967353
render_frame revision=10319 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37228 foreground_cwd=none root=cmd.exe root_pid=37228 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9985 output_bytes=967353 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=825 dispatched_commands=825 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=146 waited_for_response=0 completed_without_wait=825 total_dispatch_us=41888 max_dispatch_us=9950 total_drain_us=43522 max_drain_us=9977
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>for /L %i in (1,1,10000) do @echo UNTERM_NEXT_CORE_FLOOD_%i
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

### viewport page cycle

```text
bench_flood lines=10000 bytes=967364 elapsed_ms=799 lines_per_sec=12500.6 bytes_per_sec=1209258.7
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=73 min_us=46 p50_us=50 p95_us=55 max_us=209
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967385
render_frame revision=10684 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=36544 foreground_cwd=none root=cmd.exe root_pid=36544 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9980 output_bytes=967385 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2269 dispatched_commands=2269 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=145 waited_for_response=0 completed_without_wait=2269 total_dispatch_us=90787 max_dispatch_us=8922 total_drain_us=94045 max_drain_us=8948
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

C:\Users\Alex>echo UNTERM_NEXT_CORE_FLOOD_DONE_10000_1
UNTERM_NEXT_CORE_FLOOD_DONE_10000_1

C:\Users\Alex>exit

```

### viewport scroll during flood

```text
bench_viewport_scroll_flood lines=5000 scrolls=77 rows_read=2223 total_ms=427 min_us=31 p50_us=205 p95_us=251 max_us=424
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1822) raw_bytes=481995
render_frame revision=5058 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=32612 foreground_cwd=none root=cmd.exe root_pid=32612 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4981 output_bytes=481995 paste_count=0 paste_text_bytes=0 screen_reads=160 viewport_scrolls=77
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=321 dispatched_commands=321 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=232 dispatched_background=80 waited_for_response=0 completed_without_wait=321 total_dispatch_us=32173 max_dispatch_us=9129 total_drain_us=33235 max_drain_us=9155
UNTERM_NEXT_CORE_FLOOD_3183
UNTERM_NEXT_CORE_FLOOD_3184
UNTERM_NEXT_CORE_FLOOD_3185
UNTERM_NEXT_CORE_FLOOD_3186
UNTERM_NEXT_CORE_FLOOD_3187
UNTERM_NEXT_CORE_FLOOD_3188
UNTERM_NEXT_CORE_FLOOD_3189
UNTERM_NEXT_CORE_FLOOD_3190
UNTERM_NEXT_CORE_FLOOD_3191
UNTERM_NEXT_CORE_FLOOD_3192
UNTERM_NEXT_CORE_FLOOD_3193
UNTERM_NEXT_CORE_FLOOD_3194
UNTERM_NEXT_CORE_FLOOD_3195
UNTERM_NEXT_CORE_FLOOD_3196
UNTERM_NEXT_CORE_FLOOD_3197
UNTERM_NEXT_CORE_FLOOD_3198
UNTERM_NEXT_CORE_FLOOD_3199
UNTERM_NEXT_CORE_FLOOD_3200
UNTERM_NEXT_CORE_FLOOD_3201
UNTERM_NEXT_CORE_FLOOD_3202
UNTERM_NEXT_CORE_FLOOD_3203
UNTERM_NEXT_CORE_FLOOD_3204
UNTERM_NEXT_CORE_FLOOD_3205
UNTERM_NEXT_CORE_FLOOD_3206
UNTERM_NEXT_CORE_FLOOD_3207
UNTERM_NEXT_CORE_FLOOD_3208
UNTERM_NEXT_CORE_FLOOD_3209
UNTERM_NEXT_CORE_FLOOD_3210
UNTERM_NEXT_CORE_FLOOD_3211
UNTERM_NEXT_CORE_FLOOD_3212
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5105 p50_us=5540 p95_us=15961 max_us=16191
bench_dual_agents lines_per_agent=5000 total_bytes=964236 elapsed_ms=452 combined_lines_per_sec=22078.2 combined_bytes_per_sec=2128858.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2967
render_frame revision=52 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=23640 foreground_cwd=none root=cmd.exe root_pid=23640 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=52 output_bytes=2967 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=146 dispatched_commands=146 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=110 waited_for_response=0 completed_without_wait=146 total_dispatch_us=31998 max_dispatch_us=9281 total_drain_us=32745 max_drain_us=9309

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0011
UNTERM_NEXT_CORE_BENCH_0011

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0012
UNTERM_NEXT_CORE_BENCH_0012

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0013
UNTERM_NEXT_CORE_BENCH_0013

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0014
UNTERM_NEXT_CORE_BENCH_0014

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0015
UNTERM_NEXT_CORE_BENCH_0015

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0016
UNTERM_NEXT_CORE_BENCH_0016

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0017
UNTERM_NEXT_CORE_BENCH_0017

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0018
UNTERM_NEXT_CORE_BENCH_0018

C:\Users\Alex>echo UNTERM_NEXT_CORE_BENCH_0019
UNTERM_NEXT_CORE_BENCH_0019

C:\Users\Alex>exit

```

### agent startup stall

```text
bench_agent_startup_stall lines=5000 bytes=481910 input_writes=77 screen_reads=77 elapsed_ms=418 input_min_us=4 input_p50_us=14 input_p95_us=32 input_max_us=65 screen_read_min_us=8 screen_read_p50_us=16 screen_read_p95_us=25 screen_read_max_us=72
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=38240 foreground_cwd=none root=cmd.exe root_pid=38240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=78 input_bytes=236 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=83 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=246 dispatched_commands=246 dispatched_lifecycle=3 dispatched_input=80 dispatched_render=5 dispatched_screen=78 dispatched_background=80 waited_for_response=0 completed_without_wait=246 total_dispatch_us=26371 max_dispatch_us=9886 total_drain_us=27247 max_drain_us=9914
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=76 total_ms=415 min_us=24 p50_us=72 p95_us=111 max_us=242 text_bytes=58615
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=481802
render_frame revision=4981 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=1832 foreground_cwd=none root=cmd.exe root_pid=1832 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4981 output_bytes=481802 paste_count=0 paste_text_bytes=0 screen_reads=82 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=165 dispatched_commands=165 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=77 dispatched_background=79 waited_for_response=0 completed_without_wait=165 total_dispatch_us=23217 max_dispatch_us=9153 total_drain_us=23903 max_drain_us=9181
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

C:\Users\Alex>echo UNTERM_NEXT_CORE_FLOOD_DONE_5000_1
UNTERM_NEXT_CORE_FLOOD_DONE_5000_1

C:\Users\Alex>exit
```

### render frame latency

```text
bench_render_frame rounds=1000 full_us=343 full_lines=30 empty_deltas=1000 min_us=1 p50_us=1 p95_us=2 max_us=11 dirty_rounds=50 dirty_lines=1500 dirty_min_us=266 dirty_p50_us=427 dirty_p95_us=668 dirty_max_us=1006
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=9753
render_frame revision=176 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=31032 foreground_cwd=none root=cmd.exe root_pid=31032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=176 output_bytes=9753 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1268 dispatched_commands=1268 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=7 waited_for_response=0 completed_without_wait=1268 total_dispatch_us=64532 max_dispatch_us=9579 total_drain_us=67027 max_drain_us=9607

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0041
RENDER_FRAME_DIRTY_0041

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0042
RENDER_FRAME_DIRTY_0042

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0043
RENDER_FRAME_DIRTY_0043

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0044
RENDER_FRAME_DIRTY_0044

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0045
RENDER_FRAME_DIRTY_0045

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0046
RENDER_FRAME_DIRTY_0046

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0047
RENDER_FRAME_DIRTY_0047

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0048
RENDER_FRAME_DIRTY_0048

C:\Users\Alex>echo RENDER_FRAME_DIRTY_0049
RENDER_FRAME_DIRTY_0049

C:\Users\Alex>exit

```

### render draw plan latency

```text
bench_render_plan rounds=1000 glyph_runs=55 cell_runs=30 min_us=125 p50_us=131 p95_us=146 max_us=329
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2386
render_frame revision=8 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37724 foreground_cwd=none root=cmd.exe root_pid=37724 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=8 output_bytes=2386 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=19 total_dispatch_us=16779 max_dispatch_us=8954 total_drain_us=16902 max_drain_us=8981
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

C:\Users\Alex>echo RENDER_PLAN_BENCH_READY
RENDER_PLAN_BENCH_READY

C:\Users\Alex>exit

```

### render geometry plan latency

```text
bench_render_geometry_plan rounds=1000 glyph_runs=55 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=5 max_us=73
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2834
render_frame revision=7 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37968 foreground_cwd=none root=cmd.exe root_pid=37968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=7 output_bytes=2834 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=18 dispatched_commands=18 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=18 total_dispatch_us=17401 max_dispatch_us=9197 total_drain_us=17514 max_drain_us=9223
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

C:\Users\Alex>echo RENDER_GEOMETRY_PLAN_BENCH_READY
RENDER_GEOMETRY_PLAN_BENCH_READY

C:\Users\Alex>exit

```

### render submission plan latency

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=4 p95_us=4 max_us=14
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2914
render_frame revision=8 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=24916 foreground_cwd=none root=cmd.exe root_pid=24916 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=8 output_bytes=2914 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=16492 max_dispatch_us=8840 total_drain_us=16617 max_drain_us=8866
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

C:\Users\Alex>echo RENDER_SUBMISSION_PLAN_BENCH_READY
RENDER_SUBMISSION_PLAN_BENCH_READY

C:\Users\Alex>exit

```

### render commit plan latency

```text
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=324 full_p50_us=345 full_p95_us=433 full_max_us=737 skip_min_us=2 skip_p50_us=3 skip_p95_us=10 skip_max_us=59
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2659
render_frame revision=8 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=32968 foreground_cwd=none root=cmd.exe root_pid=32968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=8 output_bytes=2659 paste_count=0 paste_text_bytes=0 screen_reads=2011 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2022 dispatched_commands=2022 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=6 dispatched_background=7 waited_for_response=0 completed_without_wait=2022 total_dispatch_us=215041 max_dispatch_us=8998 total_drain_us=218803 max_drain_us=9025
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

C:\Users\Alex>echo RENDER_COMMIT_PLAN_BENCH_READY
RENDER_COMMIT_PLAN_BENCH_READY

C:\Users\Alex>exit

```

### render cursor move latency

```text
bench_render_cursor_move rounds=200 snapshots=768 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=14 p50_us=52 p95_us=155 max_us=581
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(42, 3) raw_bytes=714
render_frame revision=204 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=32172 foreground_cwd=none root=cmd.exe root_pid=32172 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=204 output_bytes=714 paste_count=0 paste_text_bytes=0 screen_reads=980 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1186 dispatched_commands=1186 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=774 dispatched_background=2 waited_for_response=0 completed_without_wait=1186 total_dispatch_us=80064 max_dispatch_us=9095 total_drain_us=94171 max_drain_us=9124
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>UNTERM_CURSOR_MOVE_BENCHMARK
```

### render application cursor move latency

```text
bench_render_application_cursor_move rounds=200 snapshots=769 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=13 p50_us=67 p95_us=158 max_us=550
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=752
render_frame revision=205 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=35332 foreground_cwd=none root=cmd.exe root_pid=35332 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=205 output_bytes=752 paste_count=0 paste_text_bytes=0 screen_reads=982 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1188 dispatched_commands=1188 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=776 dispatched_background=2 waited_for_response=0 completed_without_wait=1188 total_dispatch_us=89456 max_dispatch_us=9681 total_drain_us=107560 max_drain_us=9709
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\Alex>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 background_bytes=1048515 background_elapsed_ms=1576 active_misses=0 missing_sessions=0 duplicate_sessions=0 min_us=358 p50_us=447 p95_us=787 max_us=9725
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=28468 foreground_cwd=none root=cmd.exe root_pid=28468 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=3200 dispatched_commands=3200 dispatched_lifecycle=1007 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2184 waited_for_response=0 completed_without_wait=3200 total_dispatch_us=534703 max_dispatch_us=9782 total_drain_us=544259 max_drain_us=9809
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=4760 p50_us=5072 p95_us=6355 max_us=6523
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=17752 foreground_cwd=none root=cmd.exe root_pid=17752 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=118128 max_dispatch_us=9392 total_drain_us=118452 max_drain_us=9438
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=25967 p50_us=30279 p95_us=35115 max_us=35307
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=36148 foreground_cwd=none root=cmd.exe root_pid=36148 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=154 dispatched_commands=154 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=106 waited_for_response=0 completed_without_wait=154 total_dispatch_us=196569 max_dispatch_us=12254 total_drain_us=198254 max_drain_us=12263
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### first session ready

```text
bench_first_session_ready elapsed_ms=36 create_us=9427 visible_bytes=108
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(14, 3) raw_bytes=186
render_frame revision=4 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=36188 foreground_cwd=none root=cmd.exe root_pid=36188 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=4 output_bytes=186 paste_count=0 paste_text_bytes=0 screen_reads=12 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=16 dispatched_commands=16 dispatched_lifecycle=1 dispatched_input=1 dispatched_render=5 dispatched_screen=7 dispatched_background=2 waited_for_response=0 completed_without_wait=16 total_dispatch_us=16805 max_dispatch_us=9078 total_drain_us=16941 max_drain_us=9106
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>
```


# Next-Core Benchmark Report

- Generated: 2026-07-31 08:55:54 +08:00
- Commit: `82d15857`
- Machine: `ALEX-PC01`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=253 foreground=cmd.exe cwd=C:\Users\Alex\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=4 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=4 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=19 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 runtime_pump_dispatches=10 runtime_pump_lanes=lifecycle:1,input:1,render:5,screen:1,background:2 runtime_pump_waited=0 runtime_pump_completed_without_wait=10 runtime_pump_max_dispatch_us=370520 runtime_pump_max_drain_us=370550 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 2 us | 16000 us | ok |
| key-to-screen p95 | 6225 us | 16000 us | ok |
| input burst p95 | 5 us | 33000 us | ok |
| echo p95 | 5839 us | 16000 us | ok |
| dual-agent echo p95 | 16109 us | 33000 us | ok |
| agent startup input p95 | 25 us | 33000 us | ok |
| paste 10kb elapsed | 20 ms | 50 ms | ok |
| paste under flood elapsed | 32 ms | 50 ms | ok |
| paste under flood marker misses | 0 misses | 0 misses | ok |
| scrollback page p95 | 53 us | 1000 us | ok |
| viewport scroll p95 | 58 us | 1000 us | ok |
| viewport page cycle p95 | 57 us | 1000 us | ok |
| viewport page cycle boundary misses | 0 misses | 0 misses | ok |
| viewport page cycle missed pages | 0 pages | 0 pages | ok |
| viewport scroll under flood p95 | 274 us | 50000 us | ok |
| screen read under flood p95 | 129 us | 50000 us | ok |
| render frame p95 | 1 us | 1000 us | ok |
| render draw plan p95 | 141 us | 1000 us | ok |
| render geometry plan p95 | 5 us | 1000 us | ok |
| render submission plan p95 | 4 us | 1000 us | ok |
| render commit plan p95 | 444 us | 1000 us | ok |
| render dirty frame p95 | 496 us | 1000 us | ok |
| render cursor move p95 | 184 us | 1000 us | ok |
| render cursor move full frames | 0 frames | 0 frames | ok |
| render cursor move missed moves | 0 moves | 0 moves | ok |
| render application cursor move p95 | 146 us | 1000 us | ok |
| render application cursor move full frames | 0 frames | 0 frames | ok |
| render application cursor move missed moves | 0 moves | 0 moves | ok |
| focus switch p95 | 816 us | 100000 us | ok |
| focus switch active misses | 0 misses | 0 misses | ok |
| focus switch missing sessions | 0 misses | 0 misses | ok |
| focus switch duplicate sessions | 0 misses | 0 misses | ok |
| session create p95 | 12337 us | 100000 us | ok |
| session ready p95 | 49083 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=2 p50_us=2 p95_us=2 max_us=31 bytes_per_sec=1400560.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=61
activity_process foreground=cmd.exe foreground_pid=26108 foreground_cwd=C:\Users\Alex\ root=cmd.exe root_pid=26108 root_cwd=C:\Users\Alex\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=61 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=25044 max_dispatch_us=16934 total_drain_us=25536 max_drain_us=16963
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=200 snapshots=414 min_us=147 p50_us=5616 p95_us=6225 max_us=26708
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=23187
activity_process foreground=cmd.exe foreground_pid=35392 foreground_cwd=none root=cmd.exe root_pid=35392 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=2605 output_chunks=653 output_bytes=23187 paste_count=0 paste_text_bytes=0 screen_reads=420 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=624 dispatched_commands=624 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=415 dispatched_background=2 waited_for_response=0 completed_without_wait=624 total_dispatch_us=54330 max_dispatch_us=15617 total_drain_us=58431 max_drain_us=15646
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097091 background_elapsed_ms=1833 min_us=2 p50_us=2 p95_us=5 max_us=300
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=25240 foreground_cwd=none root=cmd.exe root_pid=25240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1332 dispatched_commands=1332 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=316 waited_for_response=0 completed_without_wait=1332 total_dispatch_us=444412 max_dispatch_us=368186 total_drain_us=447548 max_drain_us=368214
```

### echo latency

- Status: ok
- Args: `--bench-echo 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=200 min_us=80 p50_us=5504 p95_us=5839 max_us=21575
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=35006
activity_process foreground=cmd.exe foreground_pid=10532 foreground_cwd=none root=cmd.exe root_pid=10532 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=6605 output_chunks=648 output_bytes=35006 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=626 dispatched_commands=626 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=1 dispatched_background=418 waited_for_response=0 completed_without_wait=626 total_dispatch_us=27345 max_dispatch_us=15362 total_drain_us=32164 max_drain_us=15390
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=8352 lines_per_sec=11972.8 bytes_per_sec=125544.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=7096 foreground_cwd=none root=cmd.exe root_pid=7096 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=92058 output_bytes=9884815 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1386 dispatched_commands=1386 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=1376 waited_for_response=0 completed_without_wait=1386 total_dispatch_us=648446 max_dispatch_us=381931 total_drain_us=658235 max_drain_us=381961
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=20 bytes_per_sec=501258.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(100, 29) raw_bytes=11698
activity_process foreground=cmd.exe foreground_pid=21768 foreground_cwd=none root=cmd.exe root_pid=21768 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=8 output_bytes=11698 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=17 total_dispatch_us=49716 max_dispatch_us=16337 total_drain_us=49869 max_drain_us=16368
```

### paste under output flood

- Status: ok
- Args: `--bench-paste-under-flood-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste_under_flood bytes=10240 flood_lines=5000 flood_bytes=481910 elapsed_ms=32 write_ms=6 marker_misses=0 background_elapsed_ms=430
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=11740
activity_process foreground=cmd.exe foreground_pid=11212 foreground_cwd=none root=cmd.exe root_pid=11212 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10334 output_chunks=11 output_bytes=11740 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=96 dispatched_commands=96 dispatched_lifecycle=3 dispatched_input=5 dispatched_render=5 dispatched_screen=1 dispatched_background=82 waited_for_response=0 completed_without_wait=96 total_dispatch_us=43114 max_dispatch_us=15796 total_drain_us=43735 max_drain_us=15824
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=967332 elapsed_ms=823 lines_per_sec=12139.1 bytes_per_sec=1174252.4
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=17 min_us=46 p50_us=50 p95_us=53 max_us=110
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967353
activity_process foreground=cmd.exe foreground_pid=32832 foreground_cwd=none root=cmd.exe root_pid=32832 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=10006 output_bytes=967353 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=494 dispatched_commands=494 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=149 waited_for_response=0 completed_without_wait=494 total_dispatch_us=45877 max_dispatch_us=15466 total_drain_us=47316 max_drain_us=15502
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=967300 elapsed_ms=811 lines_per_sec=12322.3 bytes_per_sec=1191933.9
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=49 p50_us=53 p95_us=58 max_us=155
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 10008) raw_bytes=967321
activity_process foreground=cmd.exe foreground_pid=7084 foreground_cwd=none root=cmd.exe root_pid=7084 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=10002 output_bytes=967321 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=826 dispatched_commands=826 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=147 waited_for_response=0 completed_without_wait=826 total_dispatch_us=46029 max_dispatch_us=16437 total_drain_us=47834 max_drain_us=16469
```

### viewport page cycle

- Status: ok
- Args: `--bench-viewport-page-cycle-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=967044 elapsed_ms=819 lines_per_sec=12199.5 bytes_per_sec=1179740.6
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=73 min_us=46 p50_us=50 p95_us=57 max_us=193
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967065
activity_process foreground=cmd.exe foreground_pid=31980 foreground_cwd=none root=cmd.exe root_pid=31980 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9991 output_bytes=967065 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2272 dispatched_commands=2272 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=148 waited_for_response=0 completed_without_wait=2272 total_dispatch_us=97127 max_dispatch_us=16102 total_drain_us=100550 max_drain_us=16132
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=74 rows_read=2133 total_ms=409 min_us=25 p50_us=205 p95_us=274 max_us=382
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1989) raw_bytes=482027
activity_process foreground=cmd.exe foreground_pid=32256 foreground_cwd=none root=cmd.exe root_pid=32256 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4980 output_bytes=482027 paste_count=0 paste_text_bytes=0 screen_reads=154 viewport_scrolls=74
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=309 dispatched_commands=309 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=223 dispatched_background=77 waited_for_response=0 completed_without_wait=309 total_dispatch_us=37043 max_dispatch_us=15458 total_drain_us=38171 max_drain_us=15486
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5104 p50_us=5567 p95_us=16109 max_us=16692
bench_dual_agents lines_per_agent=5000 total_bytes=964079 elapsed_ms=455 combined_lines_per_sec=21945.6 combined_bytes_per_sec=2115729.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2967
activity_process foreground=cmd.exe foreground_pid=24468 foreground_cwd=none root=cmd.exe root_pid=24468 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=52 output_bytes=2967 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=146 dispatched_commands=146 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=110 waited_for_response=0 completed_without_wait=146 total_dispatch_us=51449 max_dispatch_us=16130 total_drain_us=52325 max_drain_us=16158
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
bench_agent_startup_stall lines=5000 bytes=481910 input_writes=76 screen_reads=76 elapsed_ms=414 input_min_us=6 input_p50_us=14 input_p95_us=25 input_max_us=40 screen_read_min_us=10 screen_read_p50_us=16 screen_read_p95_us=22 screen_read_max_us=27
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=19528 foreground_cwd=none root=cmd.exe root_pid=19528 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=77 input_bytes=233 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=82 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=243 dispatched_commands=243 dispatched_lifecycle=3 dispatched_input=79 dispatched_render=5 dispatched_screen=77 dispatched_background=79 waited_for_response=0 completed_without_wait=243 total_dispatch_us=39215 max_dispatch_us=16001 total_drain_us=39982 max_drain_us=16030
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=76 total_ms=417 min_us=20 p50_us=78 p95_us=129 max_us=145 text_bytes=58597
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=481899
activity_process foreground=cmd.exe foreground_pid=35104 foreground_cwd=none root=cmd.exe root_pid=35104 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4986 output_bytes=481899 paste_count=0 paste_text_bytes=0 screen_reads=82 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=165 dispatched_commands=165 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=77 dispatched_background=79 waited_for_response=0 completed_without_wait=165 total_dispatch_us=29881 max_dispatch_us=15231 total_drain_us=30711 max_drain_us=15264
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=342 full_lines=30 empty_deltas=1000 min_us=1 p50_us=1 p95_us=1 max_us=7 dirty_rounds=50 dirty_lines=1500 dirty_min_us=215 dirty_p50_us=341 dirty_p95_us=496 dirty_max_us=831
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=9752
activity_process foreground=cmd.exe foreground_pid=27032 foreground_cwd=none root=cmd.exe root_pid=27032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=176 output_bytes=9752 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1268 dispatched_commands=1268 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=7 waited_for_response=0 completed_without_wait=1268 total_dispatch_us=60795 max_dispatch_us=15864 total_drain_us=62969 max_drain_us=15893
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=120 p50_us=127 p95_us=141 max_us=397
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2386
activity_process foreground=cmd.exe foreground_pid=5848 foreground_cwd=none root=cmd.exe root_pid=5848 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=8 output_bytes=2386 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=20 dispatched_commands=20 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=20 total_dispatch_us=23709 max_dispatch_us=15253 total_drain_us=23868 max_drain_us=15282
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
bench_render_geometry_plan rounds=1000 glyph_runs=55 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=5 max_us=113
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2833
activity_process foreground=cmd.exe foreground_pid=20240 foreground_cwd=none root=cmd.exe root_pid=20240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=7 output_bytes=2833 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=22844 max_dispatch_us=15969 total_drain_us=22980 max_drain_us=15998
RENDER_GEOMETRY_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=55 cursor=true min_us=4 p50_us=4 p95_us=4 max_us=16
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2913
activity_process foreground=cmd.exe foreground_pid=8500 foreground_cwd=none root=cmd.exe root_pid=8500 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=7 output_bytes=2913 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=22742 max_dispatch_us=15784 total_drain_us=22877 max_drain_us=15813
RENDER_SUBMISSION_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=323 full_p50_us=350 full_p95_us=444 full_max_us=714 skip_min_us=2 skip_p50_us=3 skip_p95_us=12 skip_max_us=40
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2659
activity_process foreground=cmd.exe foreground_pid=24896 foreground_cwd=none root=cmd.exe root_pid=24896 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=8 output_bytes=2659 paste_count=0 paste_text_bytes=0 screen_reads=2012 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2024 dispatched_commands=2024 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=7 dispatched_background=8 waited_for_response=0 completed_without_wait=2024 total_dispatch_us=220964 max_dispatch_us=19233 total_drain_us=225944 max_drain_us=19263
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
bench_render_cursor_move rounds=200 snapshots=764 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=12 p50_us=57 p95_us=184 max_us=921
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(42, 3) raw_bytes=714
activity_process foreground=cmd.exe foreground_pid=22544 foreground_cwd=none root=cmd.exe root_pid=22544 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=204 output_bytes=714 paste_count=0 paste_text_bytes=0 screen_reads=978 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1184 dispatched_commands=1184 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=772 dispatched_background=2 waited_for_response=0 completed_without_wait=1184 total_dispatch_us=440444 max_dispatch_us=367933 total_drain_us=455766 max_drain_us=367963
```

### render application cursor move latency

- Status: ok
- Args: `--bench-render-application-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_application_cursor_move rounds=200 snapshots=772 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=11 p50_us=53 p95_us=146 max_us=539
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(42, 3) raw_bytes=714
activity_process foreground=cmd.exe foreground_pid=31872 foreground_cwd=none root=cmd.exe root_pid=31872 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=204 output_bytes=714 paste_count=0 paste_text_bytes=0 screen_reads=984 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1190 dispatched_commands=1190 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=778 dispatched_background=2 waited_for_response=0 completed_without_wait=1190 total_dispatch_us=82778 max_dispatch_us=17589 total_drain_us=97764 max_drain_us=17620
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 background_bytes=1048390 background_elapsed_ms=1629 active_misses=0 missing_sessions=0 duplicate_sessions=0 min_us=348 p50_us=391 p95_us=816 max_us=7485
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=34720 foreground_cwd=none root=cmd.exe root_pid=34720 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=3216 dispatched_commands=3216 dispatched_lifecycle=1007 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2200 waited_for_response=0 completed_without_wait=3216 total_dispatch_us=531901 max_dispatch_us=17297 total_drain_us=540792 max_drain_us=17327
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=11174 p50_us=11611 p95_us=12337 max_us=365842
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=1792 foreground_cwd=none root=cmd.exe root_pid=1792 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=609679 max_dispatch_us=365626 total_drain_us=610027 max_drain_us=365635
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=30852 p50_us=47015 p95_us=49083 max_us=398091
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
activity_process foreground=cmd.exe foreground_pid=9400 foreground_cwd=none root=cmd.exe root_pid=9400 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=174 dispatched_commands=174 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=126 waited_for_response=0 completed_without_wait=174 total_dispatch_us=689687 max_dispatch_us=367079 total_drain_us=691406 max_drain_us=367088
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=2 p50_us=2 p95_us=2 max_us=31 bytes_per_sec=1400560.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=61
render_frame revision=2 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=1
activity_process foreground=cmd.exe foreground_pid=26108 foreground_cwd=C:\Users\Alex\ root=cmd.exe root_pid=26108 root_cwd=C:\Users\Alex\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=61 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=25044 max_dispatch_us=16934 total_drain_us=25536 max_drain_us=16963

```

### key-to-screen latency

```text
bench_key_to_screen rounds=200 snapshots=414 min_us=147 p50_us=5616 p95_us=6225 max_us=26708
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=23187
render_frame revision=653 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=35392 foreground_cwd=none root=cmd.exe root_pid=35392 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=2605 output_chunks=653 output_bytes=23187 paste_count=0 paste_text_bytes=0 screen_reads=420 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=624 dispatched_commands=624 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=415 dispatched_background=2 waited_for_response=0 completed_without_wait=624 total_dispatch_us=54330 max_dispatch_us=15617 total_drain_us=58431 max_drain_us=15646

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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097091 background_elapsed_ms=1833 min_us=2 p50_us=2 p95_us=5 max_us=300
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=25240 foreground_cwd=none root=cmd.exe root_pid=25240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1332 dispatched_commands=1332 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=316 waited_for_response=0 completed_without_wait=1332 total_dispatch_us=444412 max_dispatch_us=368186 total_drain_us=447548 max_drain_us=368214
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### echo latency

```text
bench_echo rounds=200 min_us=80 p50_us=5504 p95_us=5839 max_us=21575
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=35006
render_frame revision=648 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=10532 foreground_cwd=none root=cmd.exe root_pid=10532 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=6605 output_chunks=648 output_bytes=35006 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=626 dispatched_commands=626 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=1 dispatched_background=418 waited_for_response=0 completed_without_wait=626 total_dispatch_us=27345 max_dispatch_us=15362 total_drain_us=32164 max_drain_us=15390
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=8352 lines_per_sec=11972.8 bytes_per_sec=125544.3
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=92058 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=7096 foreground_cwd=none root=cmd.exe root_pid=7096 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=92058 output_bytes=9884815 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1386 dispatched_commands=1386 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=1376 waited_for_response=0 completed_without_wait=1386 total_dispatch_us=648446 max_dispatch_us=381931 total_drain_us=658235 max_drain_us=381961
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
bench_paste bytes=10240 elapsed_ms=20 bytes_per_sec=501258.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(100, 29) raw_bytes=11698
render_frame revision=7 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=21768 foreground_cwd=none root=cmd.exe root_pid=21768 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=8 output_bytes=11698 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=7 waited_for_response=0 completed_without_wait=17 total_dispatch_us=49716 max_dispatch_us=16337 total_drain_us=49869 max_drain_us=16368
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
bench_paste_under_flood bytes=10240 flood_lines=5000 flood_bytes=481910 elapsed_ms=32 write_ms=6 marker_misses=0 background_elapsed_ms=430
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=11740
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=11212 foreground_cwd=none root=cmd.exe root_pid=11212 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10334 output_chunks=11 output_bytes=11740 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=96 dispatched_commands=96 dispatched_lifecycle=3 dispatched_input=5 dispatched_render=5 dispatched_screen=1 dispatched_background=82 waited_for_response=0 completed_without_wait=96 total_dispatch_us=43114 max_dispatch_us=15796 total_drain_us=43735 max_drain_us=15824
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
bench_flood lines=10000 bytes=967332 elapsed_ms=823 lines_per_sec=12139.1 bytes_per_sec=1174252.4
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=17 min_us=46 p50_us=50 p95_us=53 max_us=110
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967353
render_frame revision=10006 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=32832 foreground_cwd=none root=cmd.exe root_pid=32832 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=10006 output_bytes=967353 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=494 dispatched_commands=494 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=149 waited_for_response=0 completed_without_wait=494 total_dispatch_us=45877 max_dispatch_us=15466 total_drain_us=47316 max_drain_us=15502
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
bench_flood lines=10000 bytes=967300 elapsed_ms=811 lines_per_sec=12322.3 bytes_per_sec=1191933.9
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=18 min_us=49 p50_us=53 p95_us=58 max_us=155
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 10008) raw_bytes=967321
render_frame revision=10336 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=7084 foreground_cwd=none root=cmd.exe root_pid=7084 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=10002 output_bytes=967321 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=826 dispatched_commands=826 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=147 waited_for_response=0 completed_without_wait=826 total_dispatch_us=46029 max_dispatch_us=16437 total_drain_us=47834 max_drain_us=16469
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
bench_flood lines=10000 bytes=967044 elapsed_ms=819 lines_per_sec=12199.5 bytes_per_sec=1179740.6
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=73 min_us=46 p50_us=50 p95_us=57 max_us=193
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=967065
render_frame revision=10695 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=31980 foreground_cwd=none root=cmd.exe root_pid=31980 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=9991 output_bytes=967065 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2272 dispatched_commands=2272 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=148 waited_for_response=0 completed_without_wait=2272 total_dispatch_us=97127 max_dispatch_us=16102 total_drain_us=100550 max_drain_us=16132
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
bench_viewport_scroll_flood lines=5000 scrolls=74 rows_read=2133 total_ms=409 min_us=25 p50_us=205 p95_us=274 max_us=382
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1989) raw_bytes=482027
render_frame revision=5054 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=32256 foreground_cwd=none root=cmd.exe root_pid=32256 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4980 output_bytes=482027 paste_count=0 paste_text_bytes=0 screen_reads=154 viewport_scrolls=74
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=309 dispatched_commands=309 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=223 dispatched_background=77 waited_for_response=0 completed_without_wait=309 total_dispatch_us=37043 max_dispatch_us=15458 total_drain_us=38171 max_drain_us=15486
UNTERM_NEXT_CORE_FLOOD_3016
UNTERM_NEXT_CORE_FLOOD_3017
UNTERM_NEXT_CORE_FLOOD_3018
UNTERM_NEXT_CORE_FLOOD_3019
UNTERM_NEXT_CORE_FLOOD_3020
UNTERM_NEXT_CORE_FLOOD_3021
UNTERM_NEXT_CORE_FLOOD_3022
UNTERM_NEXT_CORE_FLOOD_3023
UNTERM_NEXT_CORE_FLOOD_3024
UNTERM_NEXT_CORE_FLOOD_3025
UNTERM_NEXT_CORE_FLOOD_3026
UNTERM_NEXT_CORE_FLOOD_3027
UNTERM_NEXT_CORE_FLOOD_3028
UNTERM_NEXT_CORE_FLOOD_3029
UNTERM_NEXT_CORE_FLOOD_3030
UNTERM_NEXT_CORE_FLOOD_3031
UNTERM_NEXT_CORE_FLOOD_3032
UNTERM_NEXT_CORE_FLOOD_3033
UNTERM_NEXT_CORE_FLOOD_3034
UNTERM_NEXT_CORE_FLOOD_3035
UNTERM_NEXT_CORE_FLOOD_3036
UNTERM_NEXT_CORE_FLOOD_3037
UNTERM_NEXT_CORE_FLOOD_3038
UNTERM_NEXT_CORE_FLOOD_3039
UNTERM_NEXT_CORE_FLOOD_3040
UNTERM_NEXT_CORE_FLOOD_3041
UNTERM_NEXT_CORE_FLOOD_3042
UNTERM_NEXT_CORE_FLOOD_3043
UNTERM_NEXT_CORE_FLOOD_3044
UNTERM_NEXT_CORE_FLOOD_3045
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5104 p50_us=5567 p95_us=16109 max_us=16692
bench_dual_agents lines_per_agent=5000 total_bytes=964079 elapsed_ms=455 combined_lines_per_sec=21945.6 combined_bytes_per_sec=2115729.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2967
render_frame revision=52 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=24468 foreground_cwd=none root=cmd.exe root_pid=24468 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=52 output_bytes=2967 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=146 dispatched_commands=146 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=110 waited_for_response=0 completed_without_wait=146 total_dispatch_us=51449 max_dispatch_us=16130 total_drain_us=52325 max_drain_us=16158

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
bench_agent_startup_stall lines=5000 bytes=481910 input_writes=76 screen_reads=76 elapsed_ms=414 input_min_us=6 input_p50_us=14 input_p95_us=25 input_max_us=40 screen_read_min_us=10 screen_read_p50_us=16 screen_read_p95_us=22 screen_read_max_us=27
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=19528 foreground_cwd=none root=cmd.exe root_pid=19528 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=77 input_bytes=233 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=82 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=243 dispatched_commands=243 dispatched_lifecycle=3 dispatched_input=79 dispatched_render=5 dispatched_screen=77 dispatched_background=79 waited_for_response=0 completed_without_wait=243 total_dispatch_us=39215 max_dispatch_us=16001 total_drain_us=39982 max_drain_us=16030
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=76 total_ms=417 min_us=20 p50_us=78 p95_us=129 max_us=145 text_bytes=58597
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=481899
render_frame revision=4986 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=35104 foreground_cwd=none root=cmd.exe root_pid=35104 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=4986 output_bytes=481899 paste_count=0 paste_text_bytes=0 screen_reads=82 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=165 dispatched_commands=165 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=77 dispatched_background=79 waited_for_response=0 completed_without_wait=165 total_dispatch_us=29881 max_dispatch_us=15231 total_drain_us=30711 max_drain_us=15264
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
bench_render_frame rounds=1000 full_us=342 full_lines=30 empty_deltas=1000 min_us=1 p50_us=1 p95_us=1 max_us=7 dirty_rounds=50 dirty_lines=1500 dirty_min_us=215 dirty_p50_us=341 dirty_p95_us=496 dirty_max_us=831
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=9752
render_frame revision=176 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=27032 foreground_cwd=none root=cmd.exe root_pid=27032 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=176 output_bytes=9752 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1268 dispatched_commands=1268 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=7 waited_for_response=0 completed_without_wait=1268 total_dispatch_us=60795 max_dispatch_us=15864 total_drain_us=62969 max_drain_us=15893
RENDER_FRAME_DIRTY_0040

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
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=120 p50_us=127 p95_us=141 max_us=397
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2386
render_frame revision=7 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=5848 foreground_cwd=none root=cmd.exe root_pid=5848 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=8 output_bytes=2386 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=20 dispatched_commands=20 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=20 total_dispatch_us=23709 max_dispatch_us=15253 total_drain_us=23868 max_drain_us=15282
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
bench_render_geometry_plan rounds=1000 glyph_runs=55 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=5 max_us=113
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2833
render_frame revision=7 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=20240 foreground_cwd=none root=cmd.exe root_pid=20240 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=7 output_bytes=2833 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=22844 max_dispatch_us=15969 total_drain_us=22980 max_drain_us=15998
RENDER_GEOMETRY_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=55 cursor=true min_us=4 p50_us=4 p95_us=4 max_us=16
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2913
render_frame revision=7 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=8500 foreground_cwd=none root=cmd.exe root_pid=8500 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=7 output_bytes=2913 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=22742 max_dispatch_us=15784 total_drain_us=22877 max_drain_us=15813
RENDER_SUBMISSION_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=323 full_p50_us=350 full_p95_us=444 full_max_us=714 skip_min_us=2 skip_p50_us=3 skip_p95_us=12 skip_max_us=40
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2659
render_frame revision=8 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=24896 foreground_cwd=none root=cmd.exe root_pid=24896 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=8 output_bytes=2659 paste_count=0 paste_text_bytes=0 screen_reads=2012 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2024 dispatched_commands=2024 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=7 dispatched_background=8 waited_for_response=0 completed_without_wait=2024 total_dispatch_us=220964 max_dispatch_us=19233 total_drain_us=225944 max_drain_us=19263
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
bench_render_cursor_move rounds=200 snapshots=764 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=12 p50_us=57 p95_us=184 max_us=921
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(42, 3) raw_bytes=714
render_frame revision=204 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=22544 foreground_cwd=none root=cmd.exe root_pid=22544 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=204 output_bytes=714 paste_count=0 paste_text_bytes=0 screen_reads=978 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1184 dispatched_commands=1184 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=772 dispatched_background=2 waited_for_response=0 completed_without_wait=1184 total_dispatch_us=440444 max_dispatch_us=367933 total_drain_us=455766 max_drain_us=367963
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>UNTERM_CURSOR_MOVE_BENCHMARK
```

### render application cursor move latency

```text
bench_render_application_cursor_move rounds=200 snapshots=772 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=11 p50_us=53 p95_us=146 max_us=539
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(42, 3) raw_bytes=714
render_frame revision=204 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=31872 foreground_cwd=none root=cmd.exe root_pid=31872 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=204 output_bytes=714 paste_count=0 paste_text_bytes=0 screen_reads=984 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1190 dispatched_commands=1190 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=778 dispatched_background=2 waited_for_response=0 completed_without_wait=1190 total_dispatch_us=82778 max_dispatch_us=17589 total_drain_us=97764 max_drain_us=17620
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>UNTERM_CURSOR_MOVE_BENCHMARK
```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 background_bytes=1048390 background_elapsed_ms=1629 active_misses=0 missing_sessions=0 duplicate_sessions=0 min_us=348 p50_us=391 p95_us=816 max_us=7485
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=34720 foreground_cwd=none root=cmd.exe root_pid=34720 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=3216 dispatched_commands=3216 dispatched_lifecycle=1007 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2200 waited_for_response=0 completed_without_wait=3216 total_dispatch_us=531901 max_dispatch_us=17297 total_drain_us=540792 max_drain_us=17327
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=11174 p50_us=11611 p95_us=12337 max_us=365842
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=1792 foreground_cwd=none root=cmd.exe root_pid=1792 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=609679 max_dispatch_us=365626 total_drain_us=610027 max_drain_us=365635
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=30852 p50_us=47015 p95_us=49083 max_us=398091
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=192
render_frame revision=5 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=9400 foreground_cwd=none root=cmd.exe root_pid=9400 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=192 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=174 dispatched_commands=174 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=126 waited_for_response=0 completed_without_wait=174 total_dispatch_us=689687 max_dispatch_us=367079 total_drain_us=691406 max_drain_us=367088
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\Alex>exit

```


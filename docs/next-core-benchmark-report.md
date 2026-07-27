# Next-Core Benchmark Report

- Generated: 2026-07-28 00:48:32 +08:00
- Commit: `f982235`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=18 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=18 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=19 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 runtime_pump_dispatches=10 runtime_pump_lanes=lifecycle:1,input:1,render:5,screen:1,background:2 runtime_pump_waited=0 runtime_pump_completed_without_wait=10 runtime_pump_max_dispatch_us=25073 runtime_pump_max_drain_us=25123 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 6 us | 16000 us | ok |
| key-to-screen p95 | 5957 us | 16000 us | ok |
| input burst p95 | 8 us | 33000 us | ok |
| echo p95 | 6185 us | 16000 us | ok |
| dual-agent echo p95 | 5672 us | 33000 us | ok |
| agent startup input p95 | 78 us | 33000 us | ok |
| paste 10kb elapsed | 41 ms | 50 ms | ok |
| scrollback page p95 | 103 us | 1000 us | ok |
| viewport scroll p95 | 101 us | 1000 us | ok |
| viewport page cycle p95 | 107 us | 1000 us | ok |
| viewport page cycle boundary misses | 0 misses | 0 misses | ok |
| viewport page cycle missed pages | 0 pages | 0 pages | ok |
| viewport scroll under flood p95 | 473 us | 50000 us | ok |
| screen read under flood p95 | 214 us | 50000 us | ok |
| render frame p95 | 3 us | 1000 us | ok |
| render draw plan p95 | 257 us | 1000 us | ok |
| render geometry plan p95 | 9 us | 1000 us | ok |
| render submission plan p95 | 8 us | 1000 us | ok |
| render commit plan p95 | 765 us | 1000 us | ok |
| render dirty frame p95 | 512 us | 1000 us | ok |
| render cursor move p95 | 48 us | 1000 us | ok |
| render cursor move full frames | 0 frames | 0 frames | ok |
| render cursor move missed moves | 0 moves | 0 moves | ok |
| render application cursor move p95 | 42 us | 1000 us | ok |
| render application cursor move full frames | 0 frames | 0 frames | ok |
| render application cursor move missed moves | 0 moves | 0 moves | ok |
| focus switch p95 | 687 us | 100000 us | ok |
| session create p95 | 13547 us | 100000 us | ok |
| session ready p95 | 50298 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=3 p50_us=3 p95_us=6 max_us=63 bytes_per_sec=772399.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=21800 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=21800 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=44614 max_dispatch_us=24986 total_drain_us=46058 max_drain_us=25024
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5221 p50_us=5613 p95_us=5957 max_us=15745
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=35064 foreground_cwd=none root=cmd.exe root_pid=35064 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=366 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=108 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=162 dispatched_commands=162 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=103 dispatched_background=2 waited_for_response=0 completed_without_wait=162 total_dispatch_us=44891 max_dispatch_us=20293 total_drain_us=46097 max_drain_us=20330
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3921 min_us=3 p50_us=4 p95_us=8 max_us=100
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=63696 foreground_cwd=none root=cmd.exe root_pid=63696 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1627 dispatched_commands=1627 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=611 waited_for_response=0 completed_without_wait=1627 total_dispatch_us=251687 max_dispatch_us=25246 total_drain_us=263204 max_drain_us=25283
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5172 p50_us=5519 p95_us=6185 max_us=21937
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=10537
activity_process foreground=cmd.exe foreground_pid=59388 foreground_cwd=none root=cmd.exe root_pid=59388 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=357 output_bytes=10537 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=165 dispatched_commands=165 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=1 dispatched_background=107 waited_for_response=0 completed_without_wait=165 total_dispatch_us=49124 max_dispatch_us=28401 total_drain_us=50718 max_drain_us=28438
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=15414 lines_per_sec=6487.5 bytes_per_sec=68026.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=25508 foreground_cwd=none root=cmd.exe root_pid=25508 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=64794 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2311 dispatched_commands=2311 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2301 waited_for_response=0 completed_without_wait=2311 total_dispatch_us=873644 max_dispatch_us=18621 total_drain_us=910584 max_drain_us=18662
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=41 bytes_per_sec=243819.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
activity_process foreground=cmd.exe foreground_pid=80188 foreground_cwd=none root=cmd.exe root_pid=80188 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=15 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=19 total_dispatch_us=56854 max_dispatch_us=20724 total_drain_us=57198 max_drain_us=20763
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1560 lines_per_sec=6406.5 bytes_per_sec=671765.5
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=26 min_us=57 p50_us=76 p95_us=103 max_us=184
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=72384 foreground_cwd=none root=cmd.exe root_pid=72384 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25171 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=597 dispatched_commands=597 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=252 waited_for_response=0 completed_without_wait=597 total_dispatch_us=110743 max_dispatch_us=20290 total_drain_us=115298 max_drain_us=20326
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1421 lines_per_sec=7032.7 bytes_per_sec=737427.7
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=24 min_us=54 p50_us=71 p95_us=101 max_us=155
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=88980 foreground_cwd=none root=cmd.exe root_pid=88980 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24902 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=915 dispatched_commands=915 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=236 waited_for_response=0 completed_without_wait=915 total_dispatch_us=102594 max_dispatch_us=23666 total_drain_us=106978 max_drain_us=23697
```

### viewport page cycle

- Status: ok
- Args: `--bench-viewport-page-cycle-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1602 lines_per_sec=6239.9 bytes_per_sec=654304.2
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=109 min_us=53 p50_us=77 p95_us=107 max_us=202
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=38176 foreground_cwd=none root=cmd.exe root_pid=38176 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25141 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2390 dispatched_commands=2390 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=266 waited_for_response=0 completed_without_wait=2390 total_dispatch_us=182499 max_dispatch_us=21068 total_drain_us=190078 max_drain_us=21105
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=148 rows_read=4334 total_ms=887 min_us=54 p50_us=346 p95_us=473 max_us=622
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=40532 foreground_cwd=none root=cmd.exe root_pid=40532 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14527 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=302 viewport_scrolls=148
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=605 dispatched_commands=605 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=445 dispatched_background=151 waited_for_response=0 completed_without_wait=605 total_dispatch_us=94262 max_dispatch_us=24025 total_drain_us=98677 max_drain_us=24069
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5089 p50_us=5411 p95_us=5672 max_us=5722
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=887 combined_lines_per_sec=11265.0 combined_bytes_per_sec=1471776.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=80064 foreground_cwd=none root=cmd.exe root_pid=80064 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=154 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=219 dispatched_commands=219 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=183 waited_for_response=0 completed_without_wait=219 total_dispatch_us=76930 max_dispatch_us=23518 total_drain_us=79656 max_drain_us=23556
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=161 screen_reads=161 elapsed_ms=920 input_min_us=13 input_p50_us=38 input_p95_us=78 input_max_us=111 screen_read_min_us=17 screen_read_p50_us=38 screen_read_p95_us=63 screen_read_max_us=139
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=14260 foreground_cwd=none root=cmd.exe root_pid=14260 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=162 input_bytes=488 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=167 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=498 dispatched_commands=498 dispatched_lifecycle=3 dispatched_input=164 dispatched_render=5 dispatched_screen=162 dispatched_background=164 waited_for_response=0 completed_without_wait=498 total_dispatch_us=76905 max_dispatch_us=25074 total_drain_us=80432 max_drain_us=25124
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=128 total_ms=736 min_us=36 p50_us=140 p95_us=214 max_us=281 text_bytes=96129
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=33064 foreground_cwd=none root=cmd.exe root_pid=33064 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14625 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=134 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=269 dispatched_commands=269 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=129 dispatched_background=131 waited_for_response=0 completed_without_wait=269 total_dispatch_us=63558 max_dispatch_us=22795 total_drain_us=65868 max_drain_us=22838
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=747 full_lines=30 empty_deltas=1000 min_us=2 p50_us=2 p95_us=3 max_us=40 dirty_rounds=50 dirty_lines=1500 dirty_min_us=306 dirty_p50_us=415 dirty_p95_us=512 dirty_max_us=530
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
activity_process foreground=cmd.exe foreground_pid=18248 foreground_cwd=none root=cmd.exe root_pid=18248 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=462 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1270 dispatched_commands=1270 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=9 waited_for_response=0 completed_without_wait=1270 total_dispatch_us=139632 max_dispatch_us=77096 total_drain_us=142881 max_drain_us=77132
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=141 p50_us=190 p95_us=257 max_us=379
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=5667
activity_process foreground=cmd.exe foreground_pid=28924 foreground_cwd=none root=cmd.exe root_pid=28924 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=112 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=22 dispatched_commands=22 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=22 total_dispatch_us=43740 max_dispatch_us=22956 total_drain_us=44024 max_drain_us=23008
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=6 p50_us=7 p95_us=9 max_us=32
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
activity_process foreground=cmd.exe foreground_pid=33696 foreground_cwd=none root=cmd.exe root_pid=33696 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=111 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=44957 max_dispatch_us=25609 total_drain_us=45227 max_drain_us=25666
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
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=6 p95_us=8 max_us=46
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
activity_process foreground=cmd.exe foreground_pid=34880 foreground_cwd=none root=cmd.exe root_pid=34880 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=93 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=21 dispatched_commands=21 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=21 total_dispatch_us=41394 max_dispatch_us=22090 total_drain_us=41686 max_drain_us=22136
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=345 full_p50_us=553 full_p95_us=765 full_max_us=1096 skip_min_us=2 skip_p50_us=6 skip_p95_us=23 skip_max_us=69
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6116
activity_process foreground=cmd.exe foreground_pid=77132 foreground_cwd=none root=cmd.exe root_pid=77132 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=112 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2008 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2020 dispatched_commands=2020 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=3 dispatched_background=8 waited_for_response=0 completed_without_wait=2020 total_dispatch_us=327652 max_dispatch_us=22114 total_drain_us=335445 max_drain_us=22151
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
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=16 p50_us=26 p95_us=48 max_us=92
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
activity_process foreground=cmd.exe foreground_pid=52968 foreground_cwd=none root=cmd.exe root_pid=52968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=213 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=612 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=818 dispatched_commands=818 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=406 dispatched_background=2 waited_for_response=0 completed_without_wait=818 total_dispatch_us=60245 max_dispatch_us=21438 total_drain_us=64920 max_drain_us=21478
```

### render application cursor move latency

- Status: ok
- Args: `--bench-render-application-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_application_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=13 p50_us=25 p95_us=42 max_us=185
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
activity_process foreground=cmd.exe foreground_pid=59488 foreground_cwd=none root=cmd.exe root_pid=59488 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=212 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=611 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=817 dispatched_commands=817 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=405 dispatched_background=2 waited_for_response=0 completed_without_wait=817 total_dispatch_us=60616 max_dispatch_us=21904 total_drain_us=65459 max_drain_us=21941
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=222 p50_us=441 p95_us=687 max_us=22313
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=173
activity_process foreground=cmd.exe foreground_pid=80136 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=80136 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=173 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
health_runtime_pump drain_calls=2016 dispatched_commands=2016 dispatched_lifecycle=1007 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=1002 waited_for_response=0 completed_without_wait=2016 total_dispatch_us=524049 max_dispatch_us=26754 total_drain_us=537345 max_drain_us=26802
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=7240 p50_us=10447 p95_us=13547 max_us=63268
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=13608 foreground_cwd=none root=cmd.exe root_pid=13608 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=297735 max_dispatch_us=62788 total_drain_us=298284 max_drain_us=62797
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=32670 p50_us=43201 p95_us=50298 max_us=113742
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=70056 foreground_cwd=none root=cmd.exe root_pid=70056 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=146 dispatched_commands=146 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=98 waited_for_response=0 completed_without_wait=146 total_dispatch_us=540731 max_dispatch_us=85158 total_drain_us=543035 max_drain_us=85175
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=3 p50_us=3 p95_us=6 max_us=63 bytes_per_sec=772399.6
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
render_frame revision=3 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=21800 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=21800 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=44614 max_dispatch_us=24986 total_drain_us=46058 max_drain_us=25024

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5221 p50_us=5613 p95_us=5957 max_us=15745
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=7545
render_frame revision=366 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=35064 foreground_cwd=none root=cmd.exe root_pid=35064 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=366 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=108 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=162 dispatched_commands=162 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=103 dispatched_background=2 waited_for_response=0 completed_without_wait=162 total_dispatch_us=44891 max_dispatch_us=20293 total_drain_us=46097 max_drain_us=20330

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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3921 min_us=3 p50_us=4 p95_us=8 max_us=100
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=13 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=63696 foreground_cwd=none root=cmd.exe root_pid=63696 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=13 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1627 dispatched_commands=1627 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=611 waited_for_response=0 completed_without_wait=1627 total_dispatch_us=251687 max_dispatch_us=25246 total_drain_us=263204 max_drain_us=25283
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5172 p50_us=5519 p95_us=6185 max_us=21937
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=10537
render_frame revision=357 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=59388 foreground_cwd=none root=cmd.exe root_pid=59388 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=357 output_bytes=10537 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=165 dispatched_commands=165 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=1 dispatched_background=107 waited_for_response=0 completed_without_wait=165 total_dispatch_us=49124 max_dispatch_us=28401 total_drain_us=50718 max_drain_us=28438

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
bench_flood lines=100000 bytes=1048576 elapsed_ms=15414 lines_per_sec=6487.5 bytes_per_sec=68026.7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=64794 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=25508 foreground_cwd=none root=cmd.exe root_pid=25508 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=64794 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2311 dispatched_commands=2311 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2301 waited_for_response=0 completed_without_wait=2311 total_dispatch_us=873644 max_dispatch_us=18621 total_drain_us=910584 max_drain_us=18662
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
bench_paste bytes=10240 elapsed_ms=41 bytes_per_sec=243819.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(40, 29) raw_bytes=3600
render_frame revision=14 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=80188 foreground_cwd=none root=cmd.exe root_pid=80188 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=15 output_bytes=3600 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=9 waited_for_response=0 completed_without_wait=19 total_dispatch_us=56854 max_dispatch_us=20724 total_drain_us=57198 max_drain_us=20763
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1560 lines_per_sec=6406.5 bytes_per_sec=671765.5
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=26 min_us=57 p50_us=76 p95_us=103 max_us=184
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25171 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=72384 foreground_cwd=none root=cmd.exe root_pid=72384 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25171 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=597 dispatched_commands=597 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=252 waited_for_response=0 completed_without_wait=597 total_dispatch_us=110743 max_dispatch_us=20290 total_drain_us=115298 max_drain_us=20326
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1421 lines_per_sec=7032.7 bytes_per_sec=737427.7
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=24 min_us=54 p50_us=71 p95_us=101 max_us=155
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25236 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=88980 foreground_cwd=none root=cmd.exe root_pid=88980 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=24902 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=915 dispatched_commands=915 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=236 waited_for_response=0 completed_without_wait=915 total_dispatch_us=102594 max_dispatch_us=23666 total_drain_us=106978 max_drain_us=23697
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

### viewport page cycle

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1602 lines_per_sec=6239.9 bytes_per_sec=654304.2
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=109 min_us=53 p50_us=77 p95_us=107 max_us=202
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1048576
render_frame revision=25845 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=38176 foreground_cwd=none root=cmd.exe root_pid=38176 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25141 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2390 dispatched_commands=2390 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=266 waited_for_response=0 completed_without_wait=2390 total_dispatch_us=182499 max_dispatch_us=21068 total_drain_us=190078 max_drain_us=21105
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

### viewport scroll during flood

```text
bench_viewport_scroll_flood lines=5000 scrolls=148 rows_read=4334 total_ms=887 min_us=54 p50_us=346 p95_us=473 max_us=622
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14674 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=40532 foreground_cwd=none root=cmd.exe root_pid=40532 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14527 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=302 viewport_scrolls=148
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=605 dispatched_commands=605 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=445 dispatched_background=151 waited_for_response=0 completed_without_wait=605 total_dispatch_us=94262 max_dispatch_us=24025 total_drain_us=98677 max_drain_us=24069
UNTERM_NEXT_CORE_FLOOD_763
UNTERM_NEXT_CORE_FLOOD_764
UNTERM_NEXT_CORE_FLOOD_765
UNTERM_NEXT_CORE_FLOOD_766
UNTERM_NEXT_CORE_FLOOD_767
UNTERM_NEXT_CORE_FLOOD_768
UNTERM_NEXT_CORE_FLOOD_769
UNTERM_NEXT_CORE_FLOOD_770
UNTERM_NEXT_CORE_FLOOD_771
UNTERM_NEXT_CORE_FLOOD_772
UNTERM_NEXT_CORE_FLOOD_773
UNTERM_NEXT_CORE_FLOOD_774
UNTERM_NEXT_CORE_FLOOD_775
UNTERM_NEXT_CORE_FLOOD_776
UNTERM_NEXT_CORE_FLOOD_777
UNTERM_NEXT_CORE_FLOOD_778
UNTERM_NEXT_CORE_FLOOD_779
UNTERM_NEXT_CORE_FLOOD_780
UNTERM_NEXT_CORE_FLOOD_781
UNTERM_NEXT_CORE_FLOOD_782
UNTERM_NEXT_CORE_FLOOD_783
UNTERM_NEXT_CORE_FLOOD_784
UNTERM_NEXT_CORE_FLOOD_785
UNTERM_NEXT_CORE_FLOOD_786
UNTERM_NEXT_CORE_FLOOD_787
UNTERM_NEXT_CORE_FLOOD_788
UNTERM_NEXT_CORE_FLOOD_789
UNTERM_NEXT_CORE_FLOOD_790
UNTERM_NEXT_CORE_FLOOD_791
UNTERM_NEXT_CORE_FLOOD_792
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5089 p50_us=5411 p95_us=5672 max_us=5722
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=887 combined_lines_per_sec=11265.0 combined_bytes_per_sec=1471776.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
render_frame revision=154 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=80064 foreground_cwd=none root=cmd.exe root_pid=80064 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=154 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=219 dispatched_commands=219 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=183 waited_for_response=0 completed_without_wait=219 total_dispatch_us=76930 max_dispatch_us=23518 total_drain_us=79656 max_drain_us=23556

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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=161 screen_reads=161 elapsed_ms=920 input_min_us=13 input_p50_us=38 input_p95_us=78 input_max_us=111 screen_read_min_us=17 screen_read_p50_us=38 screen_read_p95_us=63 screen_read_max_us=139
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=14260 foreground_cwd=none root=cmd.exe root_pid=14260 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=162 input_bytes=488 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=167 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=498 dispatched_commands=498 dispatched_lifecycle=3 dispatched_input=164 dispatched_render=5 dispatched_screen=162 dispatched_background=164 waited_for_response=0 completed_without_wait=498 total_dispatch_us=76905 max_dispatch_us=25074 total_drain_us=80432 max_drain_us=25124
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=128 total_ms=736 min_us=36 p50_us=140 p95_us=214 max_us=281 text_bytes=96129
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14625 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=33064 foreground_cwd=none root=cmd.exe root_pid=33064 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14625 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=134 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=269 dispatched_commands=269 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=129 dispatched_background=131 waited_for_response=0 completed_without_wait=269 total_dispatch_us=63558 max_dispatch_us=22795 total_drain_us=65868 max_drain_us=22838
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
bench_render_frame rounds=1000 full_us=747 full_lines=30 empty_deltas=1000 min_us=2 p50_us=2 p95_us=3 max_us=40 dirty_rounds=50 dirty_lines=1500 dirty_min_us=306 dirty_p50_us=415 dirty_p95_us=512 dirty_max_us=530
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=13784
render_frame revision=462 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=18248 foreground_cwd=none root=cmd.exe root_pid=18248 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=462 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1270 dispatched_commands=1270 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=9 waited_for_response=0 completed_without_wait=1270 total_dispatch_us=139632 max_dispatch_us=77096 total_drain_us=142881 max_drain_us=77132

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
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=141 p50_us=190 p95_us=257 max_us=379
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=5667
render_frame revision=112 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=28924 foreground_cwd=none root=cmd.exe root_pid=28924 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=112 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=22 dispatched_commands=22 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=22 total_dispatch_us=43740 max_dispatch_us=22956 total_drain_us=44024 max_drain_us=23008
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=6 p50_us=7 p95_us=9 max_us=32
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
render_frame revision=111 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=33696 foreground_cwd=none root=cmd.exe root_pid=33696 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=111 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=44957 max_dispatch_us=25609 total_drain_us=45227 max_drain_us=25666
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
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=4 p50_us=6 p95_us=8 max_us=46
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
render_frame revision=92 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=34880 foreground_cwd=none root=cmd.exe root_pid=34880 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=93 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=21 dispatched_commands=21 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=21 total_dispatch_us=41394 max_dispatch_us=22090 total_drain_us=41686 max_drain_us=22136
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=345 full_p50_us=553 full_p95_us=765 full_max_us=1096 skip_min_us=2 skip_p50_us=6 skip_p95_us=23 skip_max_us=69
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6116
render_frame revision=112 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=77132 foreground_cwd=none root=cmd.exe root_pid=77132 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=112 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2008 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2020 dispatched_commands=2020 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=3 dispatched_background=8 waited_for_response=0 completed_without_wait=2020 total_dispatch_us=327652 max_dispatch_us=22114 total_drain_us=335445 max_drain_us=22151
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
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=16 p50_us=26 p95_us=48 max_us=92
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
render_frame revision=213 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=52968 foreground_cwd=none root=cmd.exe root_pid=52968 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=213 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=612 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=818 dispatched_commands=818 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=406 dispatched_background=2 waited_for_response=0 completed_without_wait=818 total_dispatch_us=60245 max_dispatch_us=21438 total_drain_us=64920 max_drain_us=21478
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\lixd2>exit

```

### render application cursor move latency

```text
bench_render_application_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=13 p50_us=25 p95_us=42 max_us=185
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 6) raw_bytes=1709
render_frame revision=212 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=59488 foreground_cwd=none root=cmd.exe root_pid=59488 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=212 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=611 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=817 dispatched_commands=817 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=405 dispatched_background=2 waited_for_response=0 completed_without_wait=817 total_dispatch_us=60616 max_dispatch_us=21904 total_drain_us=65459 max_drain_us=21941
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=222 p50_us=441 p95_us=687 max_us=22313
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=173
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=80136 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=80136 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=173 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
health_runtime_pump drain_calls=2016 dispatched_commands=2016 dispatched_lifecycle=1007 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=1002 waited_for_response=0 completed_without_wait=2016 total_dispatch_us=524049 max_dispatch_us=26754 total_drain_us=537345 max_drain_us=26802
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit
```

### session create latency

```text
bench_session_create rounds=20 min_us=7240 p50_us=10447 p95_us=13547 max_us=63268
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=13608 foreground_cwd=none root=cmd.exe root_pid=13608 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=297735 max_dispatch_us=62788 total_drain_us=298284 max_drain_us=62797
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=32670 p50_us=43201 p95_us=50298 max_us=113742
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=12 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=70056 foreground_cwd=none root=cmd.exe root_pid=70056 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=146 dispatched_commands=146 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=98 waited_for_response=0 completed_without_wait=146 total_dispatch_us=540731 max_dispatch_us=85158 total_drain_us=543035 max_drain_us=85175
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


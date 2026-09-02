# Next-Core Benchmark Report

- Generated: 2026-09-02 13:23:43 +08:00
- Commit: `eac69271`
- Machine: `zhitongdeMacBook-Air`
- OS: `Unix 26.6.2`
- Binary: `target/debug/unterm-next-core`
- Shell: `/bin/sh`
- JSON smoke: `next-core 100x30 raw_bytes=630 foreground=zsh cwd=/Users/alexlee profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=14 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=14 render_draw_plan_glyph_runs=23 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=23 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=23 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 runtime_pump_dispatches=10 runtime_pump_lanes=lifecycle:1,input:1,render:5,screen:1,background:2 runtime_pump_waited=0 runtime_pump_completed_without_wait=10 runtime_pump_max_dispatch_us=2555 runtime_pump_max_drain_us=2638 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 29 us | 16000 us | ok |
| key-to-screen p95 | 8128 us | 16000 us | ok |
| input burst p95 | 17 us | 33000 us | ok |
| echo p95 | 7661 us | 16000 us | ok |
| dual-agent echo p95 | 7610 us | 33000 us | ok |
| agent startup input p95 | 51 us | 33000 us | ok |
| paste 10kb elapsed | 8 ms | 50 ms | ok |
| paste under flood elapsed | 9 ms | 50 ms | ok |
| paste under flood marker misses | 0 misses | 0 misses | ok |
| scrollback page p95 | 91 us | 1000 us | ok |
| viewport scroll p95 | 87 us | 1000 us | ok |
| viewport page cycle p95 | 80 us | 1000 us | ok |
| viewport page cycle boundary misses | 0 misses | 0 misses | ok |
| viewport page cycle missed pages | 0 pages | 0 pages | ok |
| viewport scroll under flood p95 | 4750 us | 50000 us | ok |
| screen read under flood p95 | 650 us | 50000 us | ok |
| render frame p95 | 2 us | 1000 us | ok |
| render draw plan p95 | 212 us | 1000 us | ok |
| render geometry plan p95 | 6 us | 1000 us | ok |
| render submission plan p95 | 4 us | 1000 us | ok |
| render commit plan p95 | 439 us | 1000 us | ok |
| render dirty frame p95 | 372 us | 1000 us | ok |
| render cursor move p95 | 85 us | 1000 us | ok |
| render cursor move full frames | 0 frames | 0 frames | ok |
| render cursor move missed moves | 0 moves | 0 moves | ok |
| render application cursor move p95 | 86 us | 1000 us | ok |
| render application cursor move full frames | 0 frames | 0 frames | ok |
| render application cursor move missed moves | 0 moves | 0 moves | ok |
| focus switch p95 | 1316 us | 100000 us | ok |
| focus switch active misses | 0 misses | 0 misses | ok |
| focus switch missing sessions | 0 misses | 0 misses | ok |
| focus switch duplicate sessions | 0 misses | 0 misses | ok |
| session create p95 | 8230 us | 100000 us | ok |
| session ready p95 | 10313 us | 100000 us | ok |
| first session ready elapsed | 9 ms | 1000 ms | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_input_write rounds=1000 bytes=3000 min_us=3 p50_us=4 p95_us=29 max_us=36533 bytes_per_sec=65708.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(76, 13) raw_bytes=2620
activity_process foreground=bash foreground_pid=32771 foreground_cwd=/Users/alexlee root=bash root_pid=32771 root_cwd=/Users/alexlee child_count=1 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=867 output_bytes=2620 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=47509 max_dispatch_us=36521 total_drain_us=49405 max_drain_us=36529
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_key_to_screen rounds=200 snapshots=400 min_us=5177 p50_us=7641 p95_us=8128 max_us=8529
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6242
activity_process foreground=/bin/sh foreground_pid=32776 foreground_cwd=none root=/bin/sh root_pid=32776 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=2605 output_chunks=760 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=406 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=610 dispatched_commands=610 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=401 dispatched_background=2 waited_for_response=0 completed_without_wait=610 total_dispatch_us=60743 max_dispatch_us=5840 total_drain_us=66016 max_drain_us=5867
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=1178483 background_elapsed_ms=469 min_us=3 p50_us=4 p95_us=17 max_us=523
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(88, 3) raw_bytes=1408
activity_process foreground=/bin/sh foreground_pid=32835 foreground_cwd=none root=/bin/sh root_pid=32835 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=524 output_bytes=1408 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=1075 dispatched_commands=1075 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=59 waited_for_response=0 completed_without_wait=1075 total_dispatch_us=132624 max_dispatch_us=59373 total_drain_us=135667 max_drain_us=59388
```

### echo latency

- Status: ok
- Args: `--bench-echo 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_echo rounds=200 min_us=5110 p50_us=7238 p95_us=7661 max_us=16282
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=14262
activity_process foreground=/bin/sh foreground_pid=32874 foreground_cwd=none root=/bin/sh root_pid=32874 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=6605 output_chunks=3401 output_bytes=14262 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=612 dispatched_commands=612 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=1 dispatched_background=404 waited_for_response=0 completed_without_wait=612 total_dispatch_us=15877 max_dispatch_us=2913 total_drain_us=23208 max_drain_us=2935
UNTERM_NEXT_CORE_BENCH_0186
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0187
UNTERM_NEXT_CORE_BENCH_0187
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0188
UNTERM_NEXT_CORE_BENCH_0188
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0189
UNTERM_NEXT_CORE_BENCH_0189
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0190
UNTERM_NEXT_CORE_BENCH_0190
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0191
UNTERM_NEXT_CORE_BENCH_0191
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0192
UNTERM_NEXT_CORE_BENCH_0192
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0193
UNTERM_NEXT_CORE_BENCH_0193
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0194
UNTERM_NEXT_CORE_BENCH_0194
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0195
UNTERM_NEXT_CORE_BENCH_0195
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0196
UNTERM_NEXT_CORE_BENCH_0196
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0197
UNTERM_NEXT_CORE_BENCH_0197
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0198
UNTERM_NEXT_CORE_BENCH_0198
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0199
UNTERM_NEXT_CORE_BENCH_0199
```

### output flood

- Status: ok
- Args: `--bench-flood-lines 100000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_flood lines=100000 bytes=1147840 elapsed_ms=2031 lines_per_sec=49216.8 bytes_per_sec=564930.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 29) raw_bytes=1147852
activity_process foreground=/bin/sh foreground_pid=32928 foreground_cwd=none root=/bin/sh root_pid=32928 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=2976 output_bytes=2989259 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=226 dispatched_commands=226 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=216 waited_for_response=0 completed_without_wait=226 total_dispatch_us=11937 max_dispatch_us=2295 total_drain_us=14691 max_drain_us=2322
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_paste bytes=10240 elapsed_ms=8 bytes_per_sec=1170714.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 25) raw_bytes=2243
activity_process foreground=/bin/sh foreground_pid=33000 foreground_cwd=none root=/bin/sh root_pid=33000 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10367 output_chunks=105 output_bytes=2243 paste_count=1 paste_text_bytes=10241 screen_reads=11 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=20 dispatched_commands=20 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=6 dispatched_background=5 waited_for_response=0 completed_without_wait=20 total_dispatch_us=5442 max_dispatch_us=2407 total_drain_us=5577 max_drain_us=2438
```

### paste under output flood

- Status: ok
- Args: `--bench-paste-under-flood-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_paste_under_flood bytes=10240 flood_lines=5000 flood_bytes=144225 elapsed_ms=9 write_ms=1 marker_misses=0 background_elapsed_ms=105
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 23) raw_bytes=2255
activity_process foreground=/bin/sh foreground_pid=33005 foreground_cwd=none root=/bin/sh root_pid=33005 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10373 output_chunks=85 output_bytes=2255 paste_count=1 paste_text_bytes=10241 screen_reads=11 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=34 dispatched_commands=34 dispatched_lifecycle=3 dispatched_input=5 dispatched_render=5 dispatched_screen=6 dispatched_background=15 waited_for_response=0 completed_without_wait=34 total_dispatch_us=67804 max_dispatch_us=60235 total_drain_us=68118 max_drain_us=60244
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_flood lines=10000 bytes=289241 elapsed_ms=196 lines_per_sec=50887.1 bytes_per_sec=1471864.5
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=24 min_us=64 p50_us=68 p95_us=91 max_us=254
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=289253
activity_process foreground=/bin/sh foreground_pid=33017 foreground_cwd=none root=/bin/sh root_pid=33017 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=133 output_chunks=334 output_bytes=289253 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=374 dispatched_commands=374 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=29 waited_for_response=0 completed_without_wait=374 total_dispatch_us=27205 max_dispatch_us=2423 total_drain_us=28233 max_drain_us=2453
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_flood lines=10000 bytes=289241 elapsed_ms=211 lines_per_sec=47224.8 bytes_per_sec=1365935.5
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=25 min_us=70 p50_us=74 p95_us=87 max_us=144
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 10005) raw_bytes=289253
activity_process foreground=/bin/sh foreground_pid=33042 foreground_cwd=none root=/bin/sh root_pid=33042 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=133 output_chunks=343 output_bytes=289253 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=710 dispatched_commands=710 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=31 waited_for_response=0 completed_without_wait=710 total_dispatch_us=28358 max_dispatch_us=2329 total_drain_us=30141 max_drain_us=2358
```

### viewport page cycle

- Status: ok
- Args: `--bench-viewport-page-cycle-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_flood lines=10000 bytes=289243 elapsed_ms=201 lines_per_sec=49674.4 bytes_per_sec=1436796.2
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=101 min_us=65 p50_us=71 p95_us=80 max_us=288
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(10, 29) raw_bytes=289255
activity_process foreground=/bin/sh foreground_pid=33052 foreground_cwd=none root=/bin/sh root_pid=33052 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=133 output_chunks=334 output_bytes=289255 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=2153 dispatched_commands=2153 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=29 waited_for_response=0 completed_without_wait=2153 total_dispatch_us=97825 max_dispatch_us=2420 total_drain_us=102390 max_drain_us=2450
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_viewport_scroll_flood lines=5000 scrolls=14 rows_read=393 total_ms=128 min_us=211 p50_us=2101 p95_us=4750 max_us=4832
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4534) raw_bytes=144238
activity_process foreground=/bin/sh foreground_pid=33055 foreground_cwd=none root=/bin/sh root_pid=33055 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=198 output_bytes=144238 paste_count=0 paste_text_bytes=0 screen_reads=34 viewport_scrolls=14
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=69 dispatched_commands=69 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=43 dispatched_background=17 waited_for_response=0 completed_without_wait=69 total_dispatch_us=31234 max_dispatch_us=2782 total_drain_us=31718 max_drain_us=2800
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_dual_agents_echo rounds=20 min_us=5544 p50_us=7554 p95_us=7610 max_us=7611
bench_dual_agents lines_per_agent=5000 total_bytes=288450 elapsed_ms=143 combined_lines_per_sec=69922.6 combined_bytes_per_sec=2016916.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1482
activity_process foreground=/bin/sh foreground_pid=33077 foreground_cwd=none root=/bin/sh root_pid=33077 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=329 output_bytes=1482 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=83 dispatched_commands=83 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=47 waited_for_response=0 completed_without_wait=83 total_dispatch_us=120163 max_dispatch_us=58436 total_drain_us=120818 max_drain_us=58445
UNTERM_NEXT_CORE_BENCH_0006
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0007
UNTERM_NEXT_CORE_BENCH_0007
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0008
UNTERM_NEXT_CORE_BENCH_0008
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0009
UNTERM_NEXT_CORE_BENCH_0009
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0010
UNTERM_NEXT_CORE_BENCH_0010
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0011
UNTERM_NEXT_CORE_BENCH_0011
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0012
UNTERM_NEXT_CORE_BENCH_0012
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0013
UNTERM_NEXT_CORE_BENCH_0013
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0014
UNTERM_NEXT_CORE_BENCH_0014
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0015
UNTERM_NEXT_CORE_BENCH_0015
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0016
UNTERM_NEXT_CORE_BENCH_0016
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0017
UNTERM_NEXT_CORE_BENCH_0017
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0018
UNTERM_NEXT_CORE_BENCH_0018
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0019
UNTERM_NEXT_CORE_BENCH_0019
```

### agent startup stall

- Status: ok
- Args: `--bench-agent-startup-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_agent_startup_stall lines=5000 bytes=144226 input_writes=15 screen_reads=15 elapsed_ms=103 input_min_us=6 input_p50_us=30 input_p95_us=51 input_max_us=70 screen_read_min_us=12 screen_read_p50_us=18 screen_read_p95_us=25 screen_read_max_us=26
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1) raw_bytes=47
activity_process foreground=/bin/sh foreground_pid=33083 foreground_cwd=none root=/bin/sh root_pid=33083 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=16 input_bytes=50 output_chunks=20 output_bytes=47 paste_count=0 paste_text_bytes=0 screen_reads=21 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=60 dispatched_commands=60 dispatched_lifecycle=3 dispatched_input=18 dispatched_render=5 dispatched_screen=16 dispatched_background=18 waited_for_response=0 completed_without_wait=60 total_dispatch_us=66864 max_dispatch_us=60617 total_drain_us=67187 max_drain_us=60628
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_screen_read_flood lines=5000 reads=15 total_ms=105 min_us=13 p50_us=191 p95_us=650 max_us=666 text_bytes=10541
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=144238
activity_process foreground=/bin/sh foreground_pid=33090 foreground_cwd=none root=/bin/sh root_pid=33090 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=196 output_bytes=144238 paste_count=0 paste_text_bytes=0 screen_reads=21 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=43 dispatched_commands=43 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=16 dispatched_background=18 waited_for_response=0 completed_without_wait=43 total_dispatch_us=8083 max_dispatch_us=2229 total_drain_us=8287 max_drain_us=2258
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_frame rounds=1000 full_us=218 full_lines=30 empty_deltas=1000 min_us=2 p50_us=2 p95_us=2 max_us=4 dirty_rounds=50 dirty_lines=1500 dirty_min_us=175 dirty_p50_us=215 dirty_p95_us=372 dirty_max_us=374
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(10, 29) raw_bytes=4125
activity_process foreground=/bin/sh foreground_pid=33093 foreground_cwd=none root=/bin/sh root_pid=33093 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1565 output_chunks=302 output_bytes=4125 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=1267 dispatched_commands=1267 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=6 waited_for_response=0 completed_without_wait=1267 total_dispatch_us=38317 max_dispatch_us=2105 total_drain_us=40342 max_drain_us=2133
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_plan rounds=1000 glyph_runs=61 cell_runs=30 min_us=162 p50_us=194 p95_us=212 max_us=688
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1804
activity_process foreground=/bin/sh foreground_pid=33128 foreground_cwd=none root=/bin/sh root_pid=33128 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=140 output_chunks=82 output_bytes=1804 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=18 dispatched_commands=18 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=6 waited_for_response=0 completed_without_wait=18 total_dispatch_us=5499 max_dispatch_us=2767 total_drain_us=5619 max_drain_us=2799
RENDER_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
sh-3.2$ printf '%s%s\n' 'RENDER_PLAN' '_BENCH_READY'
RENDER_PLAN_BENCH_READY
```

### render geometry plan latency

- Status: ok
- Args: `--bench-render-geometry-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_geometry_plan rounds=1000 glyph_runs=61 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=6 max_us=9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 29) raw_bytes=2121
activity_process foreground=/bin/sh foreground_pid=33131 foreground_cwd=none root=/bin/sh root_pid=33131 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=158 output_chunks=75 output_bytes=2121 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=6 waited_for_response=0 completed_without_wait=17 total_dispatch_us=4391 max_dispatch_us=2269 total_drain_us=4496 max_drain_us=2297
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
sh-3.2$ printf '%s%s\n' 'RENDER_GEOMETRY_' 'PLAN_BENCH_READY'
RENDER_GEOMETRY_PLAN_BENCH_READY
```

### render submission plan latency

- Status: ok
- Args: `--bench-render-submission-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=61 cursor=true min_us=3 p50_us=4 p95_us=4 max_us=7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 29) raw_bytes=2191
activity_process foreground=/bin/sh foreground_pid=33134 foreground_cwd=none root=/bin/sh root_pid=33134 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=162 output_chunks=80 output_bytes=2191 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=6 waited_for_response=0 completed_without_wait=17 total_dispatch_us=4216 max_dispatch_us=2076 total_drain_us=4303 max_drain_us=2102
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
sh-3.2$ printf '%s%s\n' 'RENDER_SUBMISSION' '_PLAN_BENCH_READY'
RENDER_SUBMISSION_PLAN_BENCH_READY
```

### render commit plan latency

- Status: ok
- Args: `--bench-render-commit-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=61 full_min_us=373 full_p50_us=427 full_p95_us=439 full_max_us=537 skip_min_us=3 skip_p50_us=4 skip_p95_us=8 skip_max_us=28
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2051
activity_process foreground=/bin/sh foreground_pid=33140 foreground_cwd=none root=/bin/sh root_pid=33140 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=154 output_chunks=90 output_bytes=2051 paste_count=0 paste_text_bytes=0 screen_reads=2010 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=2020 dispatched_commands=2020 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=5 dispatched_background=6 waited_for_response=0 completed_without_wait=2020 total_dispatch_us=200536 max_dispatch_us=2285 total_drain_us=204912 max_drain_us=2311
RENDER_COMMIT_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
sh-3.2$ printf '%s%s\n' 'RENDER_COMMIT_P' 'LAN_BENCH_READY'
RENDER_COMMIT_PLAN_BENCH_READY
```

### render cursor move latency

- Status: ok
- Args: `--bench-render-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_cursor_move rounds=200 snapshots=399 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=10 p50_us=46 p95_us=85 max_us=159
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 2) raw_bytes=466
activity_process foreground=/bin/sh foreground_pid=33157 foreground_cwd=none root=/bin/sh root_pid=33157 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=217 output_bytes=466 paste_count=0 paste_text_bytes=0 screen_reads=614 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=820 dispatched_commands=820 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=408 dispatched_background=2 waited_for_response=0 completed_without_wait=820 total_dispatch_us=45638 max_dispatch_us=9923 total_drain_us=53590 max_drain_us=9934
```

### render application cursor move latency

- Status: ok
- Args: `--bench-render-application-cursor-moves 200 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_render_application_cursor_move rounds=200 snapshots=399 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=15 p50_us=45 p95_us=86 max_us=356
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(36, 0) raw_bytes=472
activity_process foreground=/bin/sh foreground_pid=33212 foreground_cwd=none root=/bin/sh root_pid=33212 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=228 output_bytes=472 paste_count=0 paste_text_bytes=0 screen_reads=613 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=819 dispatched_commands=819 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=407 dispatched_background=2 waited_for_response=0 completed_without_wait=819 total_dispatch_us=35113 max_dispatch_us=2163 total_drain_us=44306 max_drain_us=2189
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_focus_switch rounds=1000 sessions=4 background_bytes=589241 background_elapsed_ms=391 active_misses=0 missing_sessions=0 duplicate_sessions=0 min_us=25 p50_us=33 p95_us=1316 max_us=2580
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 0) raw_bytes=28
activity_process foreground=/bin/sh foreground_pid=33268 foreground_cwd=none root=/bin/sh root_pid=33268 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=3 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=3020 dispatched_commands=3020 dispatched_lifecycle=1007 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2004 waited_for_response=0 completed_without_wait=3020 total_dispatch_us=569004 max_dispatch_us=61170 total_drain_us=574555 max_drain_us=61180
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_session_create rounds=20 min_us=1901 p50_us=5167 p95_us=8230 max_us=10111
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1) raw_bytes=28
activity_process foreground=/bin/sh foreground_pid=33282 foreground_cwd=none root=/bin/sh root_pid=33282 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=1235024 max_dispatch_us=60218 total_drain_us=1236475 max_drain_us=60243
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_session_ready rounds=20 min_us=8070 p50_us=10021 p95_us=10313 max_us=10414
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 0) raw_bytes=28
activity_process foreground=/bin/sh foreground_pid=33353 foreground_cwd=none root=/bin/sh root_pid=33353 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=3 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=90 dispatched_commands=90 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=42 waited_for_response=0 completed_without_wait=90 total_dispatch_us=52997 max_dispatch_us=2754 total_drain_us=53903 max_drain_us=2778
```

### first session ready

- Status: ok
- Args: `--bench-first-session-ready --timeout-ms 120000 --wait-ms 0 --write exit\r -- /bin/sh`

```text
bench_first_session_ready elapsed_ms=9 create_us=2314 visible_bytes=7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 0) raw_bytes=28
activity_process foreground=/bin/sh foreground_pid=33381 foreground_cwd=none root=/bin/sh root_pid=33381 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=12 dispatched_commands=12 dispatched_lifecycle=1 dispatched_input=1 dispatched_render=5 dispatched_screen=3 dispatched_background=2 waited_for_response=0 completed_without_wait=12 total_dispatch_us=3991 max_dispatch_us=2183 total_drain_us=4061 max_drain_us=2214
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=3 p50_us=4 p95_us=29 max_us=36533 bytes_per_sec=65708.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(76, 13) raw_bytes=2620
render_frame revision=1918 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=bash foreground_pid=32771 foreground_cwd=/Users/alexlee root=bash root_pid=32771 root_cwd=/Users/alexlee child_count=1 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=867 output_bytes=2620 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=47509 max_dispatch_us=36521 total_drain_us=49405 max_drain_us=36529
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[sh-3.2$ [Cexit
sh: [Cexit: command not found
sh-3.2$
```

### key-to-screen latency

```text
bench_key_to_screen rounds=200 snapshots=400 min_us=5177 p50_us=7641 p95_us=8128 max_us=8529
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=6242
render_frame revision=760 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=32776 foreground_cwd=none root=/bin/sh root_pid=32776 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=2605 output_chunks=760 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=406 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=610 dispatched_commands=610 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=401 dispatched_background=2 waited_for_response=0 completed_without_wait=610 total_dispatch_us=60743 max_dispatch_us=5840 total_drain_us=66016 max_drain_us=5867
KTS0186
sh-3.2$ echo KTS0187
KTS0187
sh-3.2$ echo KTS0188
KTS0188
sh-3.2$ echo KTS0189
KTS0189
sh-3.2$ echo KTS0190
KTS0190
sh-3.2$ echo KTS0191
KTS0191
sh-3.2$ echo KTS0192
KTS0192
sh-3.2$ echo KTS0193
KTS0193
sh-3.2$ echo KTS0194
KTS0194
sh-3.2$ echo KTS0195
KTS0195
sh-3.2$ echo KTS0196
KTS0196
sh-3.2$ echo KTS0197
KTS0197
sh-3.2$ echo KTS0198
KTS0198
sh-3.2$ echo KTS0199
KTS0199
sh-3.2$ exit
exit

```

### input burst under output

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=1178483 background_elapsed_ms=469 min_us=3 p50_us=4 p95_us=17 max_us=523
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(88, 3) raw_bytes=1408
render_frame revision=1522 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=3
activity_process foreground=/bin/sh foreground_pid=32835 foreground_cwd=none root=/bin/sh root_pid=32835 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=524 output_bytes=1408 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=1075 dispatched_commands=1075 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=59 waited_for_response=0 completed_without_wait=1075 total_dispatch_us=132624 max_dispatch_us=59373 total_drain_us=135667 max_drain_us=59388
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C
^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[C^[[Csh-3.2$ exit
exit

```

### echo latency

```text
bench_echo rounds=200 min_us=5110 p50_us=7238 p95_us=7661 max_us=16282
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=14262
render_frame revision=3400 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=32874 foreground_cwd=none root=/bin/sh root_pid=32874 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=201 input_bytes=6605 output_chunks=3401 output_bytes=14262 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=612 dispatched_commands=612 dispatched_lifecycle=1 dispatched_input=201 dispatched_render=5 dispatched_screen=1 dispatched_background=404 waited_for_response=0 completed_without_wait=612 total_dispatch_us=15877 max_dispatch_us=2913 total_drain_us=23208 max_drain_us=2935
UNTERM_NEXT_CORE_BENCH_0186
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0187
UNTERM_NEXT_CORE_BENCH_0187
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0188
UNTERM_NEXT_CORE_BENCH_0188
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0189
UNTERM_NEXT_CORE_BENCH_0189
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0190
UNTERM_NEXT_CORE_BENCH_0190
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0191
UNTERM_NEXT_CORE_BENCH_0191
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0192
UNTERM_NEXT_CORE_BENCH_0192
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0193
UNTERM_NEXT_CORE_BENCH_0193
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0194
UNTERM_NEXT_CORE_BENCH_0194
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0195
UNTERM_NEXT_CORE_BENCH_0195
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0196
UNTERM_NEXT_CORE_BENCH_0196
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0197
UNTERM_NEXT_CORE_BENCH_0197
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0198
UNTERM_NEXT_CORE_BENCH_0198
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0199
UNTERM_NEXT_CORE_BENCH_0199
sh-3.2$ exit
exit

```

### output flood

```text
bench_flood lines=100000 bytes=1147840 elapsed_ms=2031 lines_per_sec=49216.8 bytes_per_sec=564930.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 29) raw_bytes=1147852
render_frame revision=2976 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=32928 foreground_cwd=none root=/bin/sh root_pid=32928 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=2976 output_bytes=2989259 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=226 dispatched_commands=226 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=216 waited_for_response=0 completed_without_wait=226 total_dispatch_us=11937 max_dispatch_us=2295 total_drain_us=14691 max_drain_us=2322
UNTERM_NEXT_CORE_FLOOD_99976
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
sh-3.2$ printf '%s%s\n' 'UNTERM_NEXT_CORE_F' 'LOOD_DONE_100000_1'
UNTERM_NEXT_CORE_FLOOD_DONE_100000_1
sh-3.2$ exit
exit

```

### paste 10kb

```text
bench_paste bytes=10240 elapsed_ms=8 bytes_per_sec=1170714.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 25) raw_bytes=2243
render_frame revision=105 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33000 foreground_cwd=none root=/bin/sh root_pid=33000 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10367 output_chunks=105 output_bytes=2243 paste_count=1 paste_text_bytes=10241 screen_reads=11 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=20 dispatched_commands=20 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=6 dispatched_background=5 waited_for_response=0 completed_without_wait=20 total_dispatch_us=5442 max_dispatch_us=2407 total_drain_us=5577 max_drain_us=2438
sh-3.2$ stty -icanon min 1 time 0; head -c 10240 >/dev/null; stty icanon; printf '%s%s\n' 'UNTERM_NE
XT_CORE' '_PASTE_DONE_10240'
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
23456789ABCDEFGHIJKLMNOPUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWX
YZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOP
QRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGH
IJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789
ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01
23456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRST
UVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKL
MNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCD
EFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ012345
6789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWX
YZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789UNTERM_NEXT_CORE_PASTE_DONE_10240
sh-3.2$ exit
exit

```

### paste under output flood

```text
bench_paste_under_flood bytes=10240 flood_lines=5000 flood_bytes=144225 elapsed_ms=9 write_ms=1 marker_misses=0 background_elapsed_ms=105
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 23) raw_bytes=2255
render_frame revision=83 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=3
activity_process foreground=/bin/sh foreground_pid=33005 foreground_cwd=none root=/bin/sh root_pid=33005 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10373 output_chunks=85 output_bytes=2255 paste_count=1 paste_text_bytes=10241 screen_reads=11 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=34 dispatched_commands=34 dispatched_lifecycle=3 dispatched_input=5 dispatched_render=5 dispatched_screen=6 dispatched_background=15 waited_for_response=0 completed_without_wait=34 total_dispatch_us=67804 max_dispatch_us=60235 total_drain_us=68118 max_drain_us=60244
sh-3.2$ stty -icanon min 1 time 0; head -c 10240 >/dev/null; stty icanon; printf '%s%s\n' 'UNTERM_NE
XT_CORE_PA' 'STE_FLOOD_DONE_10240'
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
23456789ABCDEFGHIJKLMNOPGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJ
KLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789AB
CDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123
456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUV
WXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMN
OPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEF
GHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567
89ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ
0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQR
STUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJ
KLMNOPQRSTUVWXYZ0123456789ABCDEFGHIJKLMNOPQRSTUVUNTERM_NEXT_CORE_PASTE_FLOOD_DONE_10240
sh-3.2$ exit
exit

```

### scrollback paging

```text
bench_flood lines=10000 bytes=289241 elapsed_ms=196 lines_per_sec=50887.1 bytes_per_sec=1471864.5
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=24 min_us=64 p50_us=68 p95_us=91 max_us=254
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=289253
render_frame revision=334 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33017 foreground_cwd=none root=/bin/sh root_pid=33017 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=133 output_chunks=334 output_bytes=289253 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=374 dispatched_commands=374 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=29 waited_for_response=0 completed_without_wait=374 total_dispatch_us=27205 max_dispatch_us=2423 total_drain_us=28233 max_drain_us=2453
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
sh-3.2$ printf '%s%s\n' 'UNTERM_NEXT_CORE_' 'FLOOD_DONE_10000_1'
UNTERM_NEXT_CORE_FLOOD_DONE_10000_1
sh-3.2$ exit
exit

```

### viewport scroll paging

```text
bench_flood lines=10000 bytes=289241 elapsed_ms=211 lines_per_sec=47224.8 bytes_per_sec=1365935.5
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=25 min_us=70 p50_us=74 p95_us=87 max_us=144
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 10005) raw_bytes=289253
render_frame revision=676 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=/bin/sh foreground_pid=33042 foreground_cwd=none root=/bin/sh root_pid=33042 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=133 output_chunks=343 output_bytes=289253 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=710 dispatched_commands=710 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=31 waited_for_response=0 completed_without_wait=710 total_dispatch_us=28358 max_dispatch_us=2329 total_drain_us=30141 max_drain_us=2358
awk 'BEGIN{for(i=1;i<=10000;i++)print "UNTERM_NEXT_CORE_FLOOD_" i ""}'
printf '%s%s\n' 'UNTERM_NEXT_CORE_' 'FLOOD_DONE_10000_1'
sh-3.2$ awk 'BEGIN{for(i=1;i<=10000;i++)print "UNTERM_NEXT_CORE_FLOOD_" i ""}'
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
UNTERM_NEXT_CORE_FLOOD_27
```

### viewport page cycle

```text
bench_flood lines=10000 bytes=289243 elapsed_ms=201 lines_per_sec=49674.4 bytes_per_sec=1436796.2
bench_viewport_page_cycle lines=10000 pages=704 rows_read=21120 reached_top=true reached_bottom=true live_tail=true boundary_misses=0 missed_pages=0 total_ms=101 min_us=65 p50_us=71 p95_us=80 max_us=288
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(10, 29) raw_bytes=289255
render_frame revision=1037 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=/bin/sh foreground_pid=33052 foreground_cwd=none root=/bin/sh root_pid=33052 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=133 output_chunks=334 output_bytes=289255 paste_count=0 paste_text_bytes=0 screen_reads=1416 viewport_scrolls=704
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=2153 dispatched_commands=2153 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=2115 dispatched_background=29 waited_for_response=0 completed_without_wait=2153 total_dispatch_us=97825 max_dispatch_us=2420 total_drain_us=102390 max_drain_us=2450
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
sh-3.2$ printf '%s%s\n' 'UNTERM_NEXT_CORE_' 'FLOOD_DONE_10000_1'
UNTERM_NEXT_CORE_FLOOD_DONE_10000_1
sh-3.2$ exit
exit

```

### viewport scroll during flood

```text
bench_viewport_scroll_flood lines=5000 scrolls=14 rows_read=393 total_ms=128 min_us=211 p50_us=2101 p95_us=4750 max_us=4832
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4534) raw_bytes=144238
render_frame revision=212 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33055 foreground_cwd=none root=/bin/sh root_pid=33055 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=198 output_bytes=144238 paste_count=0 paste_text_bytes=0 screen_reads=34 viewport_scrolls=14
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=69 dispatched_commands=69 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=43 dispatched_background=17 waited_for_response=0 completed_without_wait=69 total_dispatch_us=31234 max_dispatch_us=2782 total_drain_us=31718 max_drain_us=2800
UNTERM_NEXT_CORE_FLOOD_471
UNTERM_NEXT_CORE_FLOOD_472
UNTERM_NEXT_CORE_FLOOD_473
UNTERM_NEXT_CORE_FLOOD_474
UNTERM_NEXT_CORE_FLOOD_475
UNTERM_NEXT_CORE_FLOOD_476
UNTERM_NEXT_CORE_FLOOD_477
UNTERM_NEXT_CORE_FLOOD_478
UNTERM_NEXT_CORE_FLOOD_479
UNTERM_NEXT_CORE_FLOOD_480
UNTERM_NEXT_CORE_FLOOD_481
UNTERM_NEXT_CORE_FLOOD_482
UNTERM_NEXT_CORE_FLOOD_483
UNTERM_NEXT_CORE_FLOOD_484
UNTERM_NEXT_CORE_FLOOD_485
UNTERM_NEXT_CORE_FLOOD_486
UNTERM_NEXT_CORE_FLOOD_487
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
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5544 p50_us=7554 p95_us=7610 max_us=7611
bench_dual_agents lines_per_agent=5000 total_bytes=288450 elapsed_ms=143 combined_lines_per_sec=69922.6 combined_bytes_per_sec=2016916.0
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1482
render_frame revision=329 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33077 foreground_cwd=none root=/bin/sh root_pid=33077 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=329 output_bytes=1482 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=83 dispatched_commands=83 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=47 waited_for_response=0 completed_without_wait=83 total_dispatch_us=120163 max_dispatch_us=58436 total_drain_us=120818 max_drain_us=58445
UNTERM_NEXT_CORE_BENCH_0006
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0007
UNTERM_NEXT_CORE_BENCH_0007
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0008
UNTERM_NEXT_CORE_BENCH_0008
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0009
UNTERM_NEXT_CORE_BENCH_0009
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0010
UNTERM_NEXT_CORE_BENCH_0010
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0011
UNTERM_NEXT_CORE_BENCH_0011
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0012
UNTERM_NEXT_CORE_BENCH_0012
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0013
UNTERM_NEXT_CORE_BENCH_0013
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0014
UNTERM_NEXT_CORE_BENCH_0014
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0015
UNTERM_NEXT_CORE_BENCH_0015
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0016
UNTERM_NEXT_CORE_BENCH_0016
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0017
UNTERM_NEXT_CORE_BENCH_0017
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0018
UNTERM_NEXT_CORE_BENCH_0018
sh-3.2$ echo UNTERM_NEXT_CORE_BENCH_0019
UNTERM_NEXT_CORE_BENCH_0019
sh-3.2$ exit
exit

```

### agent startup stall

```text
bench_agent_startup_stall lines=5000 bytes=144226 input_writes=15 screen_reads=15 elapsed_ms=103 input_min_us=6 input_p50_us=30 input_p95_us=51 input_max_us=70 screen_read_min_us=12 screen_read_p50_us=18 screen_read_p95_us=25 screen_read_max_us=26
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1) raw_bytes=47
render_frame revision=34 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=/bin/sh foreground_pid=33083 foreground_cwd=none root=/bin/sh root_pid=33083 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=16 input_bytes=50 output_chunks=20 output_bytes=47 paste_count=0 paste_text_bytes=0 screen_reads=21 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=60 dispatched_commands=60 dispatched_lifecycle=3 dispatched_input=18 dispatched_render=5 dispatched_screen=16 dispatched_background=18 waited_for_response=0 completed_without_wait=60 total_dispatch_us=66864 max_dispatch_us=60617 total_drain_us=67187 max_drain_us=60628
^[[Csh-3.2$ exit
exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=15 total_ms=105 min_us=13 p50_us=191 p95_us=650 max_us=666 text_bytes=10541
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=144238
render_frame revision=196 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33090 foreground_cwd=none root=/bin/sh root_pid=33090 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=196 output_bytes=144238 paste_count=0 paste_text_bytes=0 screen_reads=21 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=43 dispatched_commands=43 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=16 dispatched_background=18 waited_for_response=0 completed_without_wait=43 total_dispatch_us=8083 max_dispatch_us=2229 total_drain_us=8287 max_drain_us=2258
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
sh-3.2$ printf '%s%s\n' 'UNTERM_NEXT_CORE_' 'FLOOD_DONE_5000_1'
UNTERM_NEXT_CORE_FLOOD_DONE_5000_1
sh-3.2$ exit
exit

```

### render frame latency

```text
bench_render_frame rounds=1000 full_us=218 full_lines=30 empty_deltas=1000 min_us=2 p50_us=2 p95_us=2 max_us=4 dirty_rounds=50 dirty_lines=1500 dirty_min_us=175 dirty_p50_us=215 dirty_p95_us=372 dirty_max_us=374
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(10, 29) raw_bytes=4125
render_frame revision=301 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=/bin/sh foreground_pid=33093 foreground_cwd=none root=/bin/sh root_pid=33093 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1565 output_chunks=302 output_bytes=4125 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=1267 dispatched_commands=1267 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=6 waited_for_response=0 completed_without_wait=1267 total_dispatch_us=38317 max_dispatch_us=2105 total_drain_us=40342 max_drain_us=2133
RENDER_FRAME_DIRTY_0036
sh-3.2$ echo RENDER_FRAME_DIRTY_0037
RENDER_FRAME_DIRTY_0037
sh-3.2$ echo RENDER_FRAME_DIRTY_0038
RENDER_FRAME_DIRTY_0038
sh-3.2$ echo RENDER_FRAME_DIRTY_0039
RENDER_FRAME_DIRTY_0039
sh-3.2$ echo RENDER_FRAME_DIRTY_0040
RENDER_FRAME_DIRTY_0040
sh-3.2$ echo RENDER_FRAME_DIRTY_0041
RENDER_FRAME_DIRTY_0041
sh-3.2$ echo RENDER_FRAME_DIRTY_0042
RENDER_FRAME_DIRTY_0042
sh-3.2$ echo RENDER_FRAME_DIRTY_0043
RENDER_FRAME_DIRTY_0043
sh-3.2$ echo RENDER_FRAME_DIRTY_0044
RENDER_FRAME_DIRTY_0044
sh-3.2$ echo RENDER_FRAME_DIRTY_0045
RENDER_FRAME_DIRTY_0045
sh-3.2$ echo RENDER_FRAME_DIRTY_0046
RENDER_FRAME_DIRTY_0046
sh-3.2$ echo RENDER_FRAME_DIRTY_0047
RENDER_FRAME_DIRTY_0047
sh-3.2$ echo RENDER_FRAME_DIRTY_0048
RENDER_FRAME_DIRTY_0048
sh-3.2$ echo RENDER_FRAME_DIRTY_0049
RENDER_FRAME_DIRTY_0049
sh-3.2$ exit
exit

```

### render draw plan latency

```text
bench_render_plan rounds=1000 glyph_runs=61 cell_runs=30 min_us=162 p50_us=194 p95_us=212 max_us=688
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=1804
render_frame revision=82 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33128 foreground_cwd=none root=/bin/sh root_pid=33128 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=140 output_chunks=82 output_bytes=1804 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=18 dispatched_commands=18 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=6 waited_for_response=0 completed_without_wait=18 total_dispatch_us=5499 max_dispatch_us=2767 total_drain_us=5619 max_drain_us=2799
RENDER_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
sh-3.2$ printf '%s%s\n' 'RENDER_PLAN' '_BENCH_READY'
RENDER_PLAN_BENCH_READY
sh-3.2$ exit
exit

```

### render geometry plan latency

```text
bench_render_geometry_plan rounds=1000 glyph_runs=61 cell_runs=30 viewport=800x480 min_us=5 p50_us=5 p95_us=6 max_us=9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 29) raw_bytes=2121
render_frame revision=75 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33131 foreground_cwd=none root=/bin/sh root_pid=33131 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=158 output_chunks=75 output_bytes=2121 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=6 waited_for_response=0 completed_without_wait=17 total_dispatch_us=4391 max_dispatch_us=2269 total_drain_us=4496 max_drain_us=2297
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
sh-3.2$ printf '%s%s\n' 'RENDER_GEOMETRY_' 'PLAN_BENCH_READY'
RENDER_GEOMETRY_PLAN_BENCH_READY
sh-3.2$ exit
exit

```

### render submission plan latency

```text
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=61 cursor=true min_us=3 p50_us=4 p95_us=4 max_us=7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 29) raw_bytes=2191
render_frame revision=80 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33134 foreground_cwd=none root=/bin/sh root_pid=33134 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=162 output_chunks=80 output_bytes=2191 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=17 dispatched_commands=17 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=6 waited_for_response=0 completed_without_wait=17 total_dispatch_us=4216 max_dispatch_us=2076 total_drain_us=4303 max_drain_us=2102
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
sh-3.2$ printf '%s%s\n' 'RENDER_SUBMISSION' '_PLAN_BENCH_READY'
RENDER_SUBMISSION_PLAN_BENCH_READY
sh-3.2$ exit
exit

```

### render commit plan latency

```text
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=61 full_min_us=373 full_p50_us=427 full_p95_us=439 full_max_us=537 skip_min_us=3 skip_p50_us=4 skip_p95_us=8 skip_max_us=28
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=2051
render_frame revision=90 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33140 foreground_cwd=none root=/bin/sh root_pid=33140 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=154 output_chunks=90 output_bytes=2051 paste_count=0 paste_text_bytes=0 screen_reads=2010 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=2020 dispatched_commands=2020 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=5 dispatched_background=6 waited_for_response=0 completed_without_wait=2020 total_dispatch_us=200536 max_dispatch_us=2285 total_drain_us=204912 max_drain_us=2311
RENDER_COMMIT_PLAN_BENCH_6 abcdefghijklmnopqrstuvwxyz
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
sh-3.2$ printf '%s%s\n' 'RENDER_COMMIT_P' 'LAN_BENCH_READY'
RENDER_COMMIT_PLAN_BENCH_READY
sh-3.2$ exit
exit

```

### render cursor move latency

```text
bench_render_cursor_move rounds=200 snapshots=399 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=10 p50_us=46 p95_us=85 max_us=159
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 2) raw_bytes=466
render_frame revision=216 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=/bin/sh foreground_pid=33157 foreground_cwd=none root=/bin/sh root_pid=33157 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=217 output_bytes=466 paste_count=0 paste_text_bytes=0 screen_reads=614 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=820 dispatched_commands=820 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=408 dispatched_background=2 waited_for_response=0 completed_without_wait=820 total_dispatch_us=45638 max_dispatch_us=9923 total_drain_us=53590 max_drain_us=9934
sh-3.2$ UNTERM_CURSOR_MOVE_BENCHMARK
sh-3.2$ exit
exit

```

### render application cursor move latency

```text
bench_render_application_cursor_move rounds=200 snapshots=399 dirty_lines=200 full_frames=0 left_moves=100 right_moves=100 missed_moves=0 min_us=15 p50_us=45 p95_us=86 max_us=356
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(36, 0) raw_bytes=472
render_frame revision=226 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=5
activity_process foreground=/bin/sh foreground_pid=33212 foreground_cwd=none root=/bin/sh root_pid=33212 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=228 output_bytes=472 paste_count=0 paste_text_bytes=0 screen_reads=613 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=819 dispatched_commands=819 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=407 dispatched_background=2 waited_for_response=0 completed_without_wait=819 total_dispatch_us=35113 max_dispatch_us=2163 total_drain_us=44306 max_drain_us=2189
sh-3.2$ UNTERM_CURSOR_MOVE_BENCHMARK
exit
sh-3.2$ exit
exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 background_bytes=589241 background_elapsed_ms=391 active_misses=0 missing_sessions=0 duplicate_sessions=0 min_us=25 p50_us=33 p95_us=1316 max_us=2580
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 0) raw_bytes=28
render_frame revision=1 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33268 foreground_cwd=none root=/bin/sh root_pid=33268 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=3 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=4 total_destroyed=3 total_marked_dead=4 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=3020 dispatched_commands=3020 dispatched_lifecycle=1007 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2004 waited_for_response=0 completed_without_wait=3020 total_dispatch_us=569004 max_dispatch_us=61170 total_drain_us=574555 max_drain_us=61180
sh-3.2$ exit
exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=1901 p50_us=5167 p95_us=8230 max_us=10111
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 1) raw_bytes=28
render_frame revision=4 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=/bin/sh foreground_pid=33282 foreground_cwd=none root=/bin/sh root_pid=33282 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=1235024 max_dispatch_us=60218 total_drain_us=1236475 max_drain_us=60243
sh-3.2$ exit
exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=8070 p50_us=10021 p95_us=10313 max_us=10414
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(8, 0) raw_bytes=28
render_frame revision=1 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=1
activity_process foreground=/bin/sh foreground_pid=33353 foreground_cwd=none root=/bin/sh root_pid=33353 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=3 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=90 dispatched_commands=90 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=42 waited_for_response=0 completed_without_wait=90 total_dispatch_us=52997 max_dispatch_us=2754 total_drain_us=53903 max_drain_us=2778
sh-3.2$ exit
exit

```

### first session ready

```text
bench_first_session_ready elapsed_ms=9 create_us=2314 visible_bytes=7
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(9, 0) raw_bytes=28
render_frame revision=4 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=/bin/sh foreground_pid=33381 foreground_cwd=none root=/bin/sh root_pid=33381 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=5 output_bytes=28 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=pty_reader_eof
health_runtime_pump drain_calls=12 dispatched_commands=12 dispatched_lifecycle=1 dispatched_input=1 dispatched_render=5 dispatched_screen=3 dispatched_background=2 waited_for_response=0 completed_without_wait=12 total_dispatch_us=3991 max_dispatch_us=2183 total_drain_us=4061 max_drain_us=2214
sh-3.2$ exit
exit

```


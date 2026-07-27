# Next-Core Benchmark Report

- Generated: 2026-07-28 00:25:09 +08:00
- Commit: `a6049ed`
- Machine: `ZHITONG-OMEN`
- OS: `Microsoft Windows NT 10.0.26200.0`
- Binary: `target\debug\unterm-next-core.exe`
- JSON smoke: `next-core 100x30 raw_bytes=330 foreground=cmd.exe cwd=C:\Users\lixd2\ profile=bench-profile proxy_keys=HTTPS_PROXY screen_reads=6 render_frame_revision=7 render_frame_lines=30 render_frame_cols=100 render_frame_grid_cells=3000 render_delta_lines=0 render_draw_plan_revision=7 render_draw_plan_glyph_runs=19 render_draw_plan_cell_runs=30 render_draw_plan_cursor=True render_draw_delta_glyph_runs=0 render_draw_delta_cell_runs=0 render_draw_delta_cursor=True render_geometry_viewport=800x480 render_geometry_glyph_runs=19 render_geometry_cell_runs=30 render_geometry_cursor=True render_submission_damage_rects=1 render_submission_text_runs=19 render_submission_background_quads=30 render_submission_cursor=True render_commit_submit=True render_commit_full_repaint=True render_commit_damage_rects=1 runtime_pump_dispatches=10 runtime_pump_lanes=lifecycle:1,input:1,render:5,screen:1,background:2 runtime_pump_max_dispatch_us=25479 runtime_pump_max_drain_us=25826 lifecycle_created=1 dead_reason=`

## Gates

| Gate | Actual | Max | Status |
| --- | ---: | ---: | --- |
| input write p95 | 11 us | 16000 us | ok |
| key-to-screen p95 | 5867 us | 16000 us | ok |
| input burst p95 | 6 us | 33000 us | ok |
| echo p95 | 5665 us | 16000 us | ok |
| dual-agent echo p95 | 5556 us | 33000 us | ok |
| agent startup input p95 | 74 us | 33000 us | ok |
| paste 10kb elapsed | 25 ms | 50 ms | ok |
| scrollback page p95 | 95 us | 1000 us | ok |
| viewport scroll p95 | 111 us | 1000 us | ok |
| viewport scroll under flood p95 | 460 us | 50000 us | ok |
| screen read under flood p95 | 192 us | 50000 us | ok |
| render frame p95 | 3 us | 1000 us | ok |
| render draw plan p95 | 254 us | 1000 us | ok |
| render geometry plan p95 | 9 us | 1000 us | ok |
| render submission plan p95 | 9 us | 1000 us | ok |
| render commit plan p95 | 695 us | 1000 us | ok |
| render dirty frame p95 | 515 us | 1000 us | ok |
| render cursor move p95 | 40 us | 1000 us | ok |
| focus switch p95 | 682 us | 100000 us | ok |
| session create p95 | 14204 us | 100000 us | ok |
| session ready p95 | 47501 us | 100000 us | ok |

## Summary

### input write latency

- Status: ok
- Args: `--bench-input-writes 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_write rounds=1000 bytes=3000 min_us=2 p50_us=3 p95_us=11 max_us=183 bytes_per_sec=693160.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
activity_process foreground=cmd.exe foreground_pid=80068 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=80068 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=47837 max_dispatch_us=24906 total_drain_us=49526 max_drain_us=24962
```

### key-to-screen latency

- Status: ok
- Args: `--bench-key-to-screen 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5234 p50_us=5540 p95_us=5867 max_us=16118
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
activity_process foreground=cmd.exe foreground_pid=84712 foreground_cwd=none root=cmd.exe root_pid=84712 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=108 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=162 dispatched_commands=162 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=103 dispatched_background=2 waited_for_response=0 completed_without_wait=162 total_dispatch_us=48243 max_dispatch_us=24479 total_drain_us=49524 max_drain_us=24514
```

### input burst under output

- Status: ok
- Args: `--bench-input-burst 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3692 min_us=3 p50_us=4 p95_us=6 max_us=150
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=89080 foreground_cwd=none root=cmd.exe root_pid=89080 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1596 dispatched_commands=1596 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=580 waited_for_response=0 completed_without_wait=1596 total_dispatch_us=280869 max_dispatch_us=76799 total_drain_us=292416 max_drain_us=76827
```

### echo latency

- Status: ok
- Args: `--bench-echo 50 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_echo rounds=50 min_us=5098 p50_us=5503 p95_us=5665 max_us=16315
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10521
activity_process foreground=cmd.exe foreground_pid=18692 foreground_cwd=none root=cmd.exe root_pid=18692 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=358 output_bytes=10521 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=163 dispatched_commands=163 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=1 dispatched_background=105 waited_for_response=0 completed_without_wait=163 total_dispatch_us=40607 max_dispatch_us=20869 total_drain_us=42181 max_drain_us=20897
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
bench_flood lines=100000 bytes=1048576 elapsed_ms=14744 lines_per_sec=6782.3 bytes_per_sec=71117.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=71816 foreground_cwd=none root=cmd.exe root_pid=71816 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=65432 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2238 dispatched_commands=2238 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2228 waited_for_response=0 completed_without_wait=2238 total_dispatch_us=771560 max_dispatch_us=28153 total_drain_us=804345 max_drain_us=28205
```

### paste 10kb

- Status: ok
- Args: `--bench-paste-kb 10 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_paste bytes=10240 elapsed_ms=25 bytes_per_sec=395972.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=3273
activity_process foreground=cmd.exe foreground_pid=78044 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=78044 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=319 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=18 dispatched_commands=18 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=18 total_dispatch_us=45271 max_dispatch_us=24328 total_drain_us=45531 max_drain_us=24358
```

### scrollback paging

- Status: ok
- Args: `--bench-scrollback-lines 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1656 lines_per_sec=6035.7 bytes_per_sec=632893.7
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=25 min_us=54 p50_us=72 p95_us=95 max_us=209
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=37756 foreground_cwd=none root=cmd.exe root_pid=37756 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25263 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=618 dispatched_commands=618 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=273 waited_for_response=0 completed_without_wait=618 total_dispatch_us=112135 max_dispatch_us=21810 total_drain_us=116625 max_drain_us=21845
```

### viewport scroll paging

- Status: ok
- Args: `--bench-viewport-scrolls 10000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1562 lines_per_sec=6401.8 bytes_per_sec=671279.8
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=28 min_us=59 p50_us=80 p95_us=111 max_us=208
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
activity_process foreground=cmd.exe foreground_pid=81616 foreground_cwd=none root=cmd.exe root_pid=81616 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25189 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=938 dispatched_commands=938 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=259 waited_for_response=0 completed_without_wait=938 total_dispatch_us=104870 max_dispatch_us=21326 total_drain_us=109777 max_drain_us=21358
```

### viewport scroll during flood

- Status: ok
- Args: `--bench-viewport-scroll-flood 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_viewport_scroll_flood lines=5000 scrolls=142 rows_read=4155 total_ms=842 min_us=41 p50_us=327 p95_us=460 max_us=594
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=55920 foreground_cwd=none root=cmd.exe root_pid=55920 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14570 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=290 viewport_scrolls=142
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=581 dispatched_commands=581 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=427 dispatched_background=145 waited_for_response=0 completed_without_wait=581 total_dispatch_us=144332 max_dispatch_us=81216 total_drain_us=148296 max_drain_us=81250
```

### dual pseudo-agent output

- Status: ok
- Args: `--bench-dual-agent-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_dual_agents_echo rounds=20 min_us=5096 p50_us=5498 p95_us=5556 max_us=5625
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=876 combined_lines_per_sec=11407.9 combined_bytes_per_sec=1490441.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
activity_process foreground=cmd.exe foreground_pid=75880 foreground_cwd=none root=cmd.exe root_pid=75880 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=152 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=219 dispatched_commands=219 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=183 waited_for_response=0 completed_without_wait=219 total_dispatch_us=130511 max_dispatch_us=62530 total_drain_us=132777 max_drain_us=62541
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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=142 screen_reads=142 elapsed_ms=807 input_min_us=11 input_p50_us=33 input_p95_us=74 input_max_us=118 screen_read_min_us=19 screen_read_p50_us=34 screen_read_p95_us=63 screen_read_max_us=161
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=87692 foreground_cwd=none root=cmd.exe root_pid=87692 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=143 input_bytes=431 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=148 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=441 dispatched_commands=441 dispatched_lifecycle=3 dispatched_input=145 dispatched_render=5 dispatched_screen=143 dispatched_background=145 waited_for_response=0 completed_without_wait=441 total_dispatch_us=119091 max_dispatch_us=76156 total_drain_us=121937 max_drain_us=76194
```

### screen read during flood

- Status: ok
- Args: `--bench-screen-read-lines 5000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_screen_read_flood lines=5000 reads=127 total_ms=724 min_us=44 p50_us=131 p95_us=192 max_us=239 text_bytes=95255
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
activity_process foreground=cmd.exe foreground_pid=69440 foreground_cwd=none root=cmd.exe root_pid=69440 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14704 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=133 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=267 dispatched_commands=267 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=128 dispatched_background=130 waited_for_response=0 completed_without_wait=267 total_dispatch_us=61244 max_dispatch_us=24253 total_drain_us=63320 max_drain_us=24283
```

### render frame latency

- Status: ok
- Args: `--bench-render-frames 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_frame rounds=1000 full_us=547 full_lines=30 empty_deltas=1000 min_us=2 p50_us=2 p95_us=3 max_us=49 dirty_rounds=50 dirty_lines=1500 dirty_min_us=300 dirty_p50_us=422 dirty_p95_us=515 dirty_max_us=600
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=13784
activity_process foreground=cmd.exe foreground_pid=76136 foreground_cwd=none root=cmd.exe root_pid=76136 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=464 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1270 dispatched_commands=1270 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=9 waited_for_response=0 completed_without_wait=1270 total_dispatch_us=84607 max_dispatch_us=24462 total_drain_us=87734 max_drain_us=24490
```

### render draw plan latency

- Status: ok
- Args: `--bench-render-plans 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=143 p50_us=185 p95_us=254 max_us=467
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=5667
activity_process foreground=cmd.exe foreground_pid=31156 foreground_cwd=none root=cmd.exe root_pid=31156 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=111 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=20 dispatched_commands=20 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=20 total_dispatch_us=38286 max_dispatch_us=19586 total_drain_us=38497 max_drain_us=19617
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=6 p50_us=8 p95_us=9 max_us=47
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
activity_process foreground=cmd.exe foreground_pid=75792 foreground_cwd=none root=cmd.exe root_pid=75792 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=81 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=46677 max_dispatch_us=28495 total_drain_us=46895 max_drain_us=28535
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
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=5 p50_us=7 p95_us=9 max_us=38
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
activity_process foreground=cmd.exe foreground_pid=11288 foreground_cwd=none root=cmd.exe root_pid=11288 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=111 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=21 dispatched_commands=21 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=21 total_dispatch_us=105862 max_dispatch_us=85663 total_drain_us=106152 max_drain_us=85710
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=367 full_p50_us=525 full_p95_us=695 full_max_us=998 skip_min_us=2 skip_p50_us=6 skip_p95_us=20 skip_max_us=104
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6116
activity_process foreground=cmd.exe foreground_pid=36316 foreground_cwd=none root=cmd.exe root_pid=36316 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=113 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2006 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2020 dispatched_commands=2020 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=2020 total_dispatch_us=379199 max_dispatch_us=75272 total_drain_us=386927 max_drain_us=75306
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
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 min_us=14 p50_us=22 p95_us=40 max_us=101
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=1709
activity_process foreground=cmd.exe foreground_pid=55884 foreground_cwd=none root=cmd.exe root_pid=55884 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=213 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=611 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=817 dispatched_commands=817 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=405 dispatched_background=2 waited_for_response=0 completed_without_wait=817 total_dispatch_us=62387 max_dispatch_us=23397 total_drain_us=67046 max_drain_us=23433
```

### focus switch latency

- Status: ok
- Args: `--bench-focus-switches 1000 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_focus_switch rounds=1000 sessions=4 min_us=208 p50_us=431 p95_us=682 max_us=22361
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=87976 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=87976 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
health_runtime_pump drain_calls=2016 dispatched_commands=2016 dispatched_lifecycle=1007 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=1002 waited_for_response=0 completed_without_wait=2016 total_dispatch_us=615626 max_dispatch_us=76421 total_drain_us=625535 max_drain_us=76439
```

### session create latency

- Status: ok
- Args: `--bench-session-create 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_create rounds=20 min_us=8174 p50_us=11370 p95_us=14204 max_us=18004
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=33028 foreground_cwd=none root=cmd.exe root_pid=33028 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=265636 max_dispatch_us=25401 total_drain_us=266132 max_drain_us=25431
```

### session ready latency

- Status: ok
- Args: `--bench-session-ready 20 --timeout-ms 120000 --wait-ms 0 --write exit\r -- cmd.exe`

```text
bench_session_ready rounds=20 min_us=34276 p50_us=41390 p95_us=47501 max_us=89573
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
activity_process foreground=cmd.exe foreground_pid=51372 foreground_cwd=none root=cmd.exe root_pid=51372 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=138 dispatched_commands=138 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=90 waited_for_response=0 completed_without_wait=138 total_dispatch_us=524133 max_dispatch_us=72645 total_drain_us=526086 max_drain_us=72659
```

## Raw Output

### input write latency

```text
bench_input_write rounds=1000 bytes=3000 min_us=2 p50_us=3 p95_us=11 max_us=183 bytes_per_sec=693160.8
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 0) raw_bytes=33
render_frame revision=3 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=80068 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=80068 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=3 output_bytes=33 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=1010 dispatched_commands=1010 dispatched_lifecycle=1 dispatched_input=1001 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=1010 total_dispatch_us=47837 max_dispatch_us=24906 total_drain_us=49526 max_drain_us=24962

```

### key-to-screen latency

```text
bench_key_to_screen rounds=50 snapshots=102 min_us=5234 p50_us=5540 p95_us=5867 max_us=16118
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=7545
render_frame revision=368 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=84712 foreground_cwd=none root=cmd.exe root_pid=84712 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=655 output_chunks=368 output_bytes=7545 paste_count=0 paste_text_bytes=0 screen_reads=108 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=162 dispatched_commands=162 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=103 dispatched_background=2 waited_for_response=0 completed_without_wait=162 total_dispatch_us=48243 max_dispatch_us=24479 total_drain_us=49524 max_drain_us=24514

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
bench_input_burst rounds=1000 background_sessions=2 background_lines_per_session=20000 background_bytes=2097152 background_elapsed_ms=3692 min_us=3 p50_us=4 p95_us=6 max_us=150
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=89080 foreground_cwd=none root=cmd.exe root_pid=89080 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1001 input_bytes=3005 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1596 dispatched_commands=1596 dispatched_lifecycle=5 dispatched_input=1005 dispatched_render=5 dispatched_screen=1 dispatched_background=580 waited_for_response=0 completed_without_wait=1596 total_dispatch_us=280869 max_dispatch_us=76799 total_drain_us=292416 max_drain_us=76827
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### echo latency

```text
bench_echo rounds=50 min_us=5098 p50_us=5503 p95_us=5665 max_us=16315
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 29) raw_bytes=10521
render_frame revision=358 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=18692 foreground_cwd=none root=cmd.exe root_pid=18692 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=51 input_bytes=1655 output_chunks=358 output_bytes=10521 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=163 dispatched_commands=163 dispatched_lifecycle=1 dispatched_input=51 dispatched_render=5 dispatched_screen=1 dispatched_background=105 waited_for_response=0 completed_without_wait=163 total_dispatch_us=40607 max_dispatch_us=20869 total_drain_us=42181 max_drain_us=20897

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
bench_flood lines=100000 bytes=1048576 elapsed_ms=14744 lines_per_sec=6782.3 bytes_per_sec=71117.9
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=65432 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=71816 foreground_cwd=none root=cmd.exe root_pid=71816 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=108 output_chunks=65432 output_bytes=13278366 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2238 dispatched_commands=2238 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=2228 waited_for_response=0 completed_without_wait=2238 total_dispatch_us=771560 max_dispatch_us=28153 total_drain_us=804345 max_drain_us=28205
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
bench_paste bytes=10240 elapsed_ms=25 bytes_per_sec=395972.2
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=3273
render_frame revision=14 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=78044 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=78044 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=10322 output_chunks=14 output_bytes=319 paste_count=1 paste_text_bytes=10241 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=1 total_destroyed=0 total_marked_dead=0 last_dead_reason=none
health_runtime_pump drain_calls=18 dispatched_commands=18 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=18 total_dispatch_us=45271 max_dispatch_us=24328 total_drain_us=45531 max_drain_us=24358
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
```

### scrollback paging

```text
bench_flood lines=10000 bytes=1048576 elapsed_ms=1656 lines_per_sec=6035.7 bytes_per_sec=632893.7
bench_scrollback lines=10000 pages=334 rows_read=10020 total_ms=25 min_us=54 p50_us=72 p95_us=95 max_us=209
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25263 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=37756 foreground_cwd=none root=cmd.exe root_pid=37756 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25263 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=618 dispatched_commands=618 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=336 dispatched_background=273 waited_for_response=0 completed_without_wait=618 total_dispatch_us=112135 max_dispatch_us=21810 total_drain_us=116625 max_drain_us=21845
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
bench_flood lines=10000 bytes=1048576 elapsed_ms=1562 lines_per_sec=6401.8 bytes_per_sec=671279.8
bench_viewport_scroll lines=10000 pages=334 rows_read=10020 total_ms=28 min_us=59 p50_us=80 p95_us=111 max_us=208
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=1048576
render_frame revision=25523 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=81616 foreground_cwd=none root=cmd.exe root_pid=81616 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=106 output_chunks=25189 output_bytes=1308360 paste_count=0 paste_text_bytes=0 screen_reads=341 viewport_scrolls=334
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=938 dispatched_commands=938 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=670 dispatched_background=259 waited_for_response=0 completed_without_wait=938 total_dispatch_us=104870 max_dispatch_us=21326 total_drain_us=109777 max_drain_us=21358
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
bench_viewport_scroll_flood lines=5000 scrolls=142 rows_read=4155 total_ms=842 min_us=41 p50_us=327 p95_us=460 max_us=594
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14712 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=55920 foreground_cwd=none root=cmd.exe root_pid=55920 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14570 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=290 viewport_scrolls=142
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=581 dispatched_commands=581 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=427 dispatched_background=145 waited_for_response=0 completed_without_wait=581 total_dispatch_us=144332 max_dispatch_us=81216 total_drain_us=148296 max_drain_us=81250
UNTERM_NEXT_CORE_FLOOD_314
UNTERM_NEXT_CORE_FLOOD_315
UNTERM_NEXT_CORE_FLOOD_316
UNTERM_NEXT_CORE_FLOOD_317
UNTERM_NEXT_CORE_FLOOD_318
UNTERM_NEXT_CORE_FLOOD_319
UNTERM_NEXT_CORE_FLOOD_320
UNTERM_NEXT_CORE_FLOOD_321
UNTERM_NEXT_CORE_FLOOD_322
UNTERM_NEXT_CORE_FLOOD_323
UNTERM_NEXT_CORE_FLOOD_324
UNTERM_NEXT_CORE_FLOOD_325
UNTERM_NEXT_CORE_FLOOD_326
UNTERM_NEXT_CORE_FLOOD_327
UNTERM_NEXT_CORE_FLOOD_328
UNTERM_NEXT_CORE_FLOOD_329
UNTERM_NEXT_CORE_FLOOD_330
UNTERM_NEXT_CORE_FLOOD_331
UNTERM_NEXT_CORE_FLOOD_332
UNTERM_NEXT_CORE_FLOOD_333
UNTERM_NEXT_CORE_FLOOD_334
UNTERM_NEXT_CORE_FLOOD_335
UNTERM_NEXT_CORE_FLOOD_336
UNTERM_NEXT_CORE_FLOOD_337
UNTERM_NEXT_CORE_FLOOD_338
UNTERM_NEXT_CORE_FLOOD_339
UNTERM_NEXT_CORE_FLOOD_340
UNTERM_NEXT_CORE_FLOOD_341
UNTERM_NEXT_CORE_FLOOD_342
UNTERM_NEXT_CORE_FLOOD_343
```

### dual pseudo-agent output

```text
bench_dual_agents_echo rounds=20 min_us=5096 p50_us=5498 p95_us=5556 max_us=5625
bench_dual_agents lines_per_agent=5000 total_bytes=1306502 elapsed_ms=876 combined_lines_per_sec=11407.9 combined_bytes_per_sec=1490441.4
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=4372
render_frame revision=152 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=75880 foreground_cwd=none root=cmd.exe root_pid=75880 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=21 input_bytes=665 output_chunks=152 output_bytes=4372 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=3 total_destroyed=2 total_marked_dead=3 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=219 dispatched_commands=219 dispatched_lifecycle=5 dispatched_input=25 dispatched_render=5 dispatched_screen=1 dispatched_background=183 waited_for_response=0 completed_without_wait=219 total_dispatch_us=130511 max_dispatch_us=62530 total_drain_us=132777 max_drain_us=62541

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
bench_agent_startup_stall lines=5000 bytes=653251 input_writes=142 screen_reads=142 elapsed_ms=807 input_min_us=11 input_p50_us=33 input_p95_us=74 input_max_us=118 screen_read_min_us=19 screen_read_p50_us=34 screen_read_p95_us=63 screen_read_max_us=161
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=87692 foreground_cwd=none root=cmd.exe root_pid=87692 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=143 input_bytes=431 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=148 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=2 total_destroyed=1 total_marked_dead=2 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=441 dispatched_commands=441 dispatched_lifecycle=3 dispatched_input=145 dispatched_render=5 dispatched_screen=143 dispatched_background=145 waited_for_response=0 completed_without_wait=441 total_dispatch_us=119091 max_dispatch_us=76156 total_drain_us=121937 max_drain_us=76194
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### screen read during flood

```text
bench_screen_read_flood lines=5000 reads=127 total_ms=724 min_us=44 p50_us=131 p95_us=192 max_us=239 text_bytes=95255
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=653354
render_frame revision=14704 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=69440 foreground_cwd=none root=cmd.exe root_pid=69440 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=104 output_chunks=14704 output_bytes=653354 paste_count=0 paste_text_bytes=0 screen_reads=133 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=267 dispatched_commands=267 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=5 dispatched_screen=128 dispatched_background=130 waited_for_response=0 completed_without_wait=267 total_dispatch_us=61244 max_dispatch_us=24253 total_drain_us=63320 max_drain_us=24283
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
bench_render_frame rounds=1000 full_us=547 full_lines=30 empty_deltas=1000 min_us=2 p50_us=2 p95_us=3 max_us=49 dirty_rounds=50 dirty_lines=1500 dirty_min_us=300 dirty_p50_us=422 dirty_p95_us=515 dirty_max_us=600
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=13784
render_frame revision=464 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=76136 foreground_cwd=none root=cmd.exe root_pid=76136 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=53 input_bytes=1538 output_chunks=464 output_bytes=13784 paste_count=0 paste_text_bytes=0 screen_reads=1207 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=1270 dispatched_commands=1270 dispatched_lifecycle=1 dispatched_input=53 dispatched_render=1106 dispatched_screen=101 dispatched_background=9 waited_for_response=0 completed_without_wait=1270 total_dispatch_us=84607 max_dispatch_us=24462 total_drain_us=87734 max_drain_us=24490

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
bench_render_plan rounds=1000 glyph_runs=54 cell_runs=30 min_us=143 p50_us=185 p95_us=254 max_us=467
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=5667
render_frame revision=110 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=30
activity_process foreground=cmd.exe foreground_pid=31156 foreground_cwd=none root=cmd.exe root_pid=31156 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=113 output_chunks=111 output_bytes=5667 paste_count=0 paste_text_bytes=0 screen_reads=8 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=20 dispatched_commands=20 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=7 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=20 total_dispatch_us=38286 max_dispatch_us=19586 total_drain_us=38497 max_drain_us=19617
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
bench_render_geometry_plan rounds=1000 glyph_runs=54 cell_runs=30 viewport=800x480 min_us=6 p50_us=8 p95_us=9 max_us=47
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6242
render_frame revision=81 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=75792 foreground_cwd=none root=cmd.exe root_pid=75792 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=131 output_chunks=81 output_bytes=6242 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=19 dispatched_commands=19 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=8 waited_for_response=0 completed_without_wait=19 total_dispatch_us=46677 max_dispatch_us=28495 total_drain_us=46895 max_drain_us=28535
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
bench_render_submission_plan rounds=1000 damage_rects=1 background_quads=30 text_runs=54 cursor=true min_us=5 p50_us=7 p95_us=9 max_us=38
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6370
render_frame revision=111 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=11288 foreground_cwd=none root=cmd.exe root_pid=11288 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=135 output_chunks=111 output_bytes=6370 paste_count=0 paste_text_bytes=0 screen_reads=7 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=21 dispatched_commands=21 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=6 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=21 total_dispatch_us=105862 max_dispatch_us=85663 total_drain_us=106152 max_drain_us=85710
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
bench_render_commit_plan rounds=1000 damage_rects=1 text_runs=54 full_min_us=367 full_p50_us=525 full_p95_us=695 full_max_us=998 skip_min_us=2 skip_p50_us=6 skip_p95_us=20 skip_max_us=104
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 29) raw_bytes=6116
render_frame revision=113 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=36316 foreground_cwd=none root=cmd.exe root_pid=36316 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=3 input_bytes=127 output_chunks=113 output_bytes=6116 paste_count=0 paste_text_bytes=0 screen_reads=2006 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=2020 dispatched_commands=2020 dispatched_lifecycle=1 dispatched_input=3 dispatched_render=2005 dispatched_screen=1 dispatched_background=10 waited_for_response=0 completed_without_wait=2020 total_dispatch_us=379199 max_dispatch_us=75272 total_drain_us=386927 max_drain_us=75306
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
bench_render_cursor_move rounds=200 snapshots=400 dirty_lines=200 full_frames=0 min_us=14 p50_us=22 p95_us=40 max_us=101
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(0, 4) raw_bytes=1709
render_frame revision=212 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=3
activity_process foreground=cmd.exe foreground_pid=55884 foreground_cwd=none root=cmd.exe root_pid=55884 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=203 input_bytes=634 output_chunks=213 output_bytes=1709 paste_count=0 paste_text_bytes=0 screen_reads=611 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=1 total_destroyed=0 total_marked_dead=1 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=817 dispatched_commands=817 dispatched_lifecycle=1 dispatched_input=203 dispatched_render=206 dispatched_screen=405 dispatched_background=2 waited_for_response=0 completed_without_wait=817 total_dispatch_us=62387 max_dispatch_us=23397 total_drain_us=67046 max_drain_us=23433
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>UNTERM_CURSOR_MOVE_BENCHMARK

C:\Users\lixd2>exit

```

### focus switch latency

```text
bench_focus_switch rounds=1000 sessions=4 min_us=208 p50_us=431 p95_us=682 max_us=22361
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(15, 3) raw_bytes=255
render_frame revision=10 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=87976 foreground_cwd=C:\Users\lixd2\ root=cmd.exe root_pid=87976 root_cwd=C:\Users\lixd2\ child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=15 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=1 dead_sessions=0 total_created=4 total_destroyed=3 total_marked_dead=3 last_dead_reason=destroyed
health_runtime_pump drain_calls=2016 dispatched_commands=2016 dispatched_lifecycle=1007 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=1002 waited_for_response=0 completed_without_wait=2016 total_dispatch_us=615626 max_dispatch_us=76421 total_drain_us=625535 max_drain_us=76439
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session create latency

```text
bench_session_create rounds=20 min_us=8174 p50_us=11370 p95_us=14204 max_us=18004
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=2
activity_process foreground=cmd.exe foreground_pid=33028 foreground_cwd=none root=cmd.exe root_pid=33028 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=12 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=50 dispatched_commands=50 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=2 waited_for_response=0 completed_without_wait=50 total_dispatch_us=265636 max_dispatch_us=25401 total_drain_us=266132 max_drain_us=25431
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```

### session ready latency

```text
bench_session_ready rounds=20 min_us=34276 p50_us=41390 p95_us=47501 max_us=89573
session id=1 cols=100 rows=30 dead=false dead_reason=none cursor=(19, 3) raw_bytes=255
render_frame revision=11 full=true dirty_rows=Some(DirtyRows { start: 0, end: 29 }) lines=30 render_delta_lines=0
activity_process foreground=cmd.exe foreground_pid=51372 foreground_cwd=none root=cmd.exe root_pid=51372 root_cwd=none child_count=0 detected_agent=none
health_io input_writes=1 input_bytes=5 output_chunks=11 output_bytes=255 paste_count=0 paste_text_bytes=0 screen_reads=6 viewport_scrolls=0
health_lifecycle live_sessions=0 dead_sessions=1 total_created=21 total_destroyed=20 total_marked_dead=21 last_dead_reason=process_exited:Success
health_runtime_pump drain_calls=138 dispatched_commands=138 dispatched_lifecycle=41 dispatched_input=1 dispatched_render=5 dispatched_screen=1 dispatched_background=90 waited_for_response=0 completed_without_wait=138 total_dispatch_us=524133 max_dispatch_us=72645 total_drain_us=526086 max_drain_us=72659
Microsoft Windows [版本 10.0.26200.8875]
(c) Microsoft Corporation。保留所有权利。

C:\Users\lixd2>exit

```


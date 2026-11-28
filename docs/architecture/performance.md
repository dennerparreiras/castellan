# Performance

No budget in the plan has been measured. The release profile is set (fat LTO, one codegen unit, static CRT) so a later benchmark has a stable binary. There are no criterion benches in this build.

The TUI polls every 40 ms while motion is on, and every 200 ms when `CASTELLAN_NO_ANIM` is set. Idle redraws still happen. A 60 fps claim is not made.

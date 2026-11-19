# Performance

No budget in the plan has been measured. The release profile is set (fat LTO, one codegen unit, static CRT) so a later benchmark has a stable binary. There are no criterion benches in this build.

The TUI redraws on a 200 ms poll while it is open. Idle-redraw avoidance and a 60 fps claim are not implemented, and this page does not claim them.

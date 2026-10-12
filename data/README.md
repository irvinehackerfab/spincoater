# Data

Test logs and measurements from the spin coater. Older tests were run on earlier firmware and hardware, so check the
date before comparing them with the current build.

Each test gets its own folder named `YYYY-MM-DD-short-description/`, with the raw files and a README on how the test
was run.

| Folder | Date | What it measured |
|---|---|---|
| [2026-10-09-controller-comparison](2026-10-09-controller-comparison) | 2026-08-14 and 2026-10-09 | Speed error, settling and overshoot at 500-6000 RPM, 10 runs per speed |

When adding a test, note:
- the model and any hardware changes (chuck, pulleys, motor)
- the firmware version or commit
- the motion profile used
- anything that went wrong during the run

TODO: raw host_tui logs for the plots in [docs/calibration.md](../docs/calibration.md).

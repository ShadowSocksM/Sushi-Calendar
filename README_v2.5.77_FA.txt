Sushi Calendar v2.5.77 - experimental calendar occasions update

Based on user-tested v2.5.76. Personal notes, tray behavior and startup code retained.
Supports displaying fixed-date solar occasions in years 1400-1453.
IMPORTANT: this release does NOT contain a complete validated 54-year dataset from vaqto.ir.
Lunar dates, variable holidays and full official calendars need separately validated annual data.

Optional external file: SushiCalendar_Events.json beside the executable.
Format: {"1405/01/01":[{"title":"Nowruz","holiday":true}]}
An entry for a date replaces the built-in entries for that date.
Do not store personal notes in this file; notes remain in SushiCalendar_Config.json.

Build on Windows: build.bat (requires Rust toolchain and cached dependencies).

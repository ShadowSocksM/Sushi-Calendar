Sushi Calendar v2.5.81 - Windows source test build

FIXES:
- Window restore code now matches current window title (was hardcoded v2.5.76).
- Duplicate --startup launch no longer sends an event that opens the existing main window.
- Installer uses a NEW executable basename to avoid old MR desktop icon cache.
- Desktop/Start shortcuts explicitly use the new executable icon.
- Remove legacy installed executable and legacy MRCalendar Run key.
- Build fails loudly if embedding Windows icon fails.

IMPORTANT: Source only, compile on Windows with build.bat. No Windows runtime test performed.
Do not delete SushiCalendar_Config.json or saved notes. Stable v2.5.76 untouched.
Check Windows Task Manager > Startup apps and shell:startup for duplicate old shortcuts if main window still opens on boot.

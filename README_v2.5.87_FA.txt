Sushi Calendar v2.5.87 - Win10 popup position and tray glyph height fix

1. Popup placement is calculated when opened, using the current Windows work area (SPI_GETWORKAREA). This avoids stale screen dimensions from early startup.
2. Persian tray digits reduced from 17px to 14px to avoid tall/clipped numerals.
3. Filled Persian zero stays intact and is aligned relative to the other glyphs rather than a fixed 5px offset.
4. Startup-hidden logic and other calendar features are unchanged.

Run build.bat on Windows to produce portable EXE and setup. Not compiled/tested on Windows here.

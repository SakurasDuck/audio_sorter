# Third-Party Notices

This project depends on external components. Their licenses and usage notes are summarized below. When redistributing, include this file alongside LICENSE.

## Chromaprint / fpcalc
- Project: Chromaprint (AcoustID) — `fpcalc` utility
- Upstream: https://github.com/acoustid/chromaprint
- License: LGPL-2.1-or-later for the library and typical fpcalc builds
- Exception: If fpcalc is built with GPL-only dependencies (e.g., GPL FFmpeg/FFTW3), the resulting binary becomes GPL-licensed. Use LGPL-friendly builds (FFmpeg built without GPL components) to keep fpcalc under LGPL.
- Integration guidance:
  - Treat fpcalc as an external, replaceable runtime dependency; do not statically link it into the MIT binary.
  - When distributing binaries, ship this notice and the upstream license text from Chromaprint alongside fpcalc, and allow users to replace fpcalc.
  - Provide attribution and source link above; for full terms see the upstream license.

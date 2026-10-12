---
status: rejected
---

# Real-space NASA skies

Use real astronomical imagery (NASA public-domain) as per-system backgrounds.

Rejected after a working prototype: source images have fixed aspect ratios and
content regions that don't fit a panning top-down viewport (the Spitzer GLIMPSE
panorama rendered ~half black), and hunting for "perfect" images doesn't scale
to 100 systems. The generated sky (`crates/client/src/sky_gen.rs`) covers the
same design goal (deterministic, shared per-place skies) with perfect fit.

Revisit if: a seamless, high-resolution, license-clean sky survey or a star
catalog renderer (Hipparcos) becomes worth wiring in as an optional override.

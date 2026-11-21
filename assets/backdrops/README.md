# Backdrops

Pictures behind the modern interface's character selection (the Stage) and
character creation (the Studio), shown in a random order, half a minute
each (`crates/ac-ui-window/src/ui/modern/pregame/backdrops.rs`).

Each is a view of the world drawn by the OpenAC client itself (the
`holtburg` render example with a free camera, at a chosen place, time of
day and sky), cropped to 16:9, upscaled to 2560×1440 and given a light
painterly finish (edge-preserving smoothing, local contrast, a soft bloom,
a gentle grade) with ImageMagick, then saved as lossy WebP (quality 84).

They are built into the native client only; the page shows the drawn skies.

# Backdrops

Pictures behind the modern interface's character selection (the Stage) and
character creation (the Studio), shown in a random order, half a minute
each (`crates/ac-ui-window/src/ui/modern/pregame/backdrops.rs`).

Each is a view of the world drawn by the OpenAC client itself (the
`holtburg` render example with a free camera, at a chosen place, time of
day and sky), drawn at 5120×2880 with the client's highest settings,
shrunk to 3840×2160 and given a light painterly finish (edge-preserving
smoothing, local contrast, a soft bloom, a gentle grade) with ImageMagick,
then saved as lossy WebP (quality 80). Each is framed with its subject in
the middle; a screen of another shape keeps the part around the subject
(`FOCUS` in `backdrops.rs`).

They are built into the native client only; the page shows the drawn skies.

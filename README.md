# qr-score

Scores QR code scannability from an SVG. Reads SVG from stdin, writes JSON to stdout.

```
cat qr.svg | qr-score
```

```json
{
  "score": 87,
  "grade": "A",
  "decodable": true,
  "content": "https://example.com",
  "results": {
    "blur_heavy": true,
    "blur_light": true,
    "contrast_down": true,
    "contrast_strict_down": true,
    "contrast_strict_up": true,
    "contrast_up": true,
    "downscale_1x": true,
    "downscale_2x": true,
    "downscale_3x": true,
    "downscale_4x": true,
    "hue_down": true,
    "hue_strict_down": true,
    "hue_strict_up": true,
    "hue_up": true,
    "luminance_down": true,
    "luminance_strict_down": true,
    "luminance_strict_up": true,
    "luminance_up": true,
    "saturation_down": true,
    "saturation_strict_down": true,
    "saturation_strict_up": true,
    "saturation_up": true
  },
  "contrast_ratio": 94,
  "error_correction": "M"
}
```

Scores 0–100. Grade boundaries: A ≥ 80, B ≥ 60, C ≥ 40, D ≥ 20, F < 20.

`contrast_ratio` (0–100) is the raw luminance spread across the image (p5–p95 percentile range), scaled to 0–100. The 0.7 clamp only applies during scoring — the output always reflects the actual measurement.

If the QR can't be decoded at all, the response is:

```json
{
  "score": 0,
  "grade": "F",
  "decodable": false,
  "error": "No QR code found in image"
}
```

## Scoring

Each stress test has a configurable weight. The final score is:

```
score = (sum of weights for passing tests + contrast_score) / total_weight * 100
```

The contrast score is not pass/fail — it's continuous. `contrast_ratio` measures the luminance spread across the image (p5–p95 percentile range, 0–1). It's clamped and normalized against a target of 0.7:

```
contrast_score = clamp(contrast_ratio / 0.7, 0, 1) * contrast_ratio_weight
```

So a QR with a contrast ratio of 0.35 gets half the contrast weight, not zero. The default contrast weight is 70 out of 100, meaning contrast dominates the score for QRs that pass all stress tests but have poor color contrast.

If the QR isn't decodable at all, the score is 0 regardless of contrast.

## Stress tests

- **Downscale** — shrinks the image to the QR's native module size ×1, ×2, ×3, ×4. Tests whether the QR survives low-resolution rendering.
- **Blur** — applies gaussian blur at σ=1.0 (light) and σ=2.0 (heavy).
- **Contrast** — adjusts contrast by ±30 (normal) or ±50 (strict).
- **Luminance** — shifts brightness by ±20 (normal) or ±40 (strict). Catches QRs that break in dark or washed-out environments.
- **Hue** — rotates hue by ±45° (normal) or ±90° (strict). Mainly relevant for coloured QRs.
- **Saturation** — scales saturation by ±30% (normal) or ±50% (strict).

All thresholds are configurable. See `qr-score.toml`.

## How it works

Renders the SVG to PNG, then runs a battery of stress tests in parallel — downscaling, blur, contrast/luminance/hue/saturation shifts — and checks whether the QR is still decodable after each. The final score is a weighted sum of passing tests plus a contrast ratio component.

Uses rxing + rqrr as decoders (both are tried, handles inverted/dark-background QRs).

## Options

```
qr-score [--config <path>] [--render-size <px>]
         [--render] [--zoom <factor>] [--dump-png <path>]
```

- `--config` — path to a TOML config file (see `qr-score.toml` for all options)
- `--render-size` — override the rasterization size (default 400px)
- `--render` — render SVG to PNG and write to stdout instead of scoring
- `--zoom` — zoom factor for `--render` mode
- `--dump-png` — render and save PNG to disk instead of scoring

## Config

All stress test thresholds and weights are configurable via TOML:

```toml
render_size = 400
contrast = 30.0
contrast_strict = 50.0

[weights]
contrast_ratio = 70
downscale_1x = 1
# ...
```

Weights must sum to 100 for scores to be meaningful. See `qr-score.toml` for the full list.

## Build

```
cargo build --release
```

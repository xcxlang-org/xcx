# mathlib

Standard math library for [XCX](https://xcxlang.com). Provides constants, arithmetic, trigonometry, statistics, geometry, combinatorics, unit conversion, and bit operations — all implemented in pure XCX with no external dependencies.

## Installation

```sh
xcx pax add mathlib@latest
```

## Usage

```xcx
include "mathlib/math.xcx" as math;

f: area = math.circle_area(5.0);
f: root = math.sqrt_f(144.0);
f: angle = math.sin_f(math.PI / 4.0);
```

## Contents

| Module | Functions |
|---|---|
| Constants | `PI`, `E`, `TAU`, `PHI`, `SQRT2`, `LN2`, `INF`, `EPSILON`, `INT_MAX`, `INT_MIN` |
| Integer math | `abs_i`, `min_i`, `max_i`, `clamp_i`, `sign_i`, `wrap_i`, `same_sign` |
| Float math | `abs_f`, `min_f`, `max_f`, `clamp_f`, `floor_f`, `ceil_f`, `round_f`, `round_to`, `lerp`, `lerp_clamped`, `smoothstep`, `remap`, `nearly_equal` |
| Powers & roots | `pow_i`, `pow_f`, `sqrt_f`, `cbrt_f`, `nroot_f`, `hypot`, `hypot3` |
| Logarithms & exp | `exp_f`, `ln_f`, `log2_f`, `log10_f`, `log_base` |
| Trigonometry | `sin_f`, `cos_f`, `tan_f`, `asin_f`, `acos_f`, `atan_f`, `atan2_f`, `deg_to_rad`, `rad_to_deg` |
| Number theory | `gcd`, `lcm`, `is_prime`, `factorial`, `fibonacci` |
| Statistics | `sum`, `mean`, `variance`, `std_dev`, `median`, `min_arr`, `max_arr`, `range_arr`, `weighted_mean`, `dot_product`, `norm_vec`, `correlation` |
| Unit conversion | length, weight, temperature, volume, speed, data |
| Geometry | `circle_area`, `rect_area`, `triangle_area`, `triangle_area_heron`, `sphere_volume`, `dist2d`, `dist3d` |
| Combinatorics | `combinations`, `permutations`, `factorial`, `catalan`, `derangements`, `surjections` |
| Bit operations | `is_pow2`, `popcount` |

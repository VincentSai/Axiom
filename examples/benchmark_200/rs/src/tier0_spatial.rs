//! Tier 0 spatial geometry and grid-math primitives.
//!
//! Every function is pure. `assert!` statements encode the preconditions from
//! the benchmark specification and its canonical implementation.

/// Calculates Manhattan distance between two grid coordinates.
pub fn calc_manhattan_dist(x1: i64, y1: i64, x2: i64, y2: i64) -> i64 {
    (x1 - x2).abs() + (y1 - y2).abs()
}

/// Calculates Chebyshev (king-move) distance between two grid coordinates.
pub fn calc_chebyshev_dist(x1: i64, y1: i64, x2: i64, y2: i64) -> i64 {
    (x1 - x2).abs().max((y1 - y2).abs())
}

/// Calculates squared Euclidean distance without taking a square root.
pub fn calc_euclidean_sq(x1: i64, y1: i64, x2: i64, y2: i64) -> i64 {
    let dx = x1 - x2;
    let dy = y1 - y2;
    dx * dx + dy * dy
}

/// Checks whether a squared distance lies within a circular radius.
pub fn check_in_radius_sq(dist_sq: i64, radius: i64) -> bool {
    assert!(dist_sq >= 0);
    assert!(radius >= 0);
    dist_sq <= radius * radius
}

/// Returns the horizontal sign step used by Bresenham rasterization.
pub fn bresenham_step_x(x1: i64, x2: i64) -> i64 {
    match x2.cmp(&x1) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
    }
}

/// Returns the vertical sign step used by Bresenham rasterization.
pub fn bresenham_step_y(y1: i64, y2: i64) -> i64 {
    match y2.cmp(&y1) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
    }
}

/// Packs X and the low 16 bits of Y into one nonnegative coordinate hash.
pub fn pack_coords_to_hash(x: i64, y: i64) -> i64 {
    assert!(x >= 0);
    assert!(y >= 0);
    x * 65_536 + y % 65_536
}

/// Unpacks the high-coordinate X component from a coordinate hash.
pub fn unpack_hash_x(hash: i64) -> i64 {
    assert!(hash >= 0);
    hash / 65_536
}

/// Unpacks the low 16-bit Y component from a coordinate hash.
pub fn unpack_hash_y(hash: i64) -> i64 {
    assert!(hash >= 0);
    hash % 65_536
}

/// Wraps an X coordinate into `[0, width)`.
pub fn wrap_map_coord_x(x: i64, width: i64) -> i64 {
    assert!(width > 0);
    x.rem_euclid(width)
}

/// Wraps a Y coordinate into `[0, height)`.
pub fn wrap_map_coord_y(y: i64, height: i64) -> i64 {
    assert!(height > 0);
    y.rem_euclid(height)
}

/// Clamps an X coordinate into `[0, width)`.
pub fn clamp_to_bounds_x(x: i64, width: i64) -> i64 {
    assert!(width > 0);
    x.clamp(0, width - 1)
}

/// Clamps a Y coordinate into `[0, height)`.
pub fn clamp_to_bounds_y(y: i64, height: i64) -> i64 {
    assert!(height > 0);
    y.clamp(0, height - 1)
}

/// Checks inclusive overlap between two valid axis-aligned bounding boxes.
#[allow(clippy::too_many_arguments)]
pub fn check_aabb_overlap(
    min_x1: i64,
    min_y1: i64,
    max_x1: i64,
    max_y1: i64,
    min_x2: i64,
    min_y2: i64,
    max_x2: i64,
    max_y2: i64,
) -> bool {
    assert!(min_x1 <= max_x1);
    assert!(min_y1 <= max_y1);
    assert!(min_x2 <= max_x2);
    assert!(min_y2 <= max_y2);

    max_x1 >= min_x2 && min_x1 <= max_x2 && max_y1 >= min_y2 && min_y1 <= max_y2
}

/// Flattens a nonnegative two-dimensional grid coordinate.
pub fn calc_grid_index(x: i64, y: i64, map_width: i64) -> i64 {
    assert!(x >= 0);
    assert!(y >= 0);
    assert!(map_width > 0);
    y * map_width + x
}

/// Calculates terrain-weighted line-of-sight energy cost.
pub fn calc_line_of_sight_cost(dist: i64, terrain_density: i64) -> i64 {
    assert!(dist >= 0);
    assert!(terrain_density >= 0);
    dist * terrain_density / 100
}

/// Returns a Cartesian quadrant code: NE=0, NW=1, SW=2, SE=3.
pub fn spatial_subsys_017(a: i64, b: i64) -> i64 {
    match (a >= 0, b >= 0) {
        (true, true) => 0,
        (false, true) => 1,
        (false, false) => 2,
        (true, false) => 3,
    }
}

/// Calculates the diamond-field mask distance from the origin.
pub fn spatial_subsys_018(a: i64, b: i64) -> i64 {
    calc_manhattan_dist(0, 0, a, b)
}

/// Returns the farthest nonnegative ray-intersection depth.
pub fn spatial_subsys_019(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    calc_chebyshev_dist(0, 0, a, b)
}

/// Calculates a quadtree child-cell index.
pub fn spatial_subsys_020(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!((0..4).contains(&b));
    a * 4 + b
}

/// Calculates the squared displacement of a grid vector.
pub fn spatial_subsys_021(a: i64, b: i64) -> i64 {
    a * a + b * b
}

/// Calculates a nonnegative projection-scale product.
pub fn spatial_subsys_022(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a * b
}

/// Returns the smaller nonnegative axis extent.
pub fn spatial_subsys_023(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a.min(b)
}

/// Sums the normalized directions of two axes.
pub fn spatial_subsys_024(a: i64, b: i64) -> i64 {
    a.signum() + b.signum()
}

/// Calculates an inclusive-axis span delta.
pub fn spatial_subsys_025(a: i64, b: i64) -> i64 {
    (a - b).abs()
}

/// Calculates the midpoint of an ordered nonnegative interval.
pub fn spatial_subsys_026(a: i64, b: i64) -> i64 {
    assert!(a >= 0 && b >= 0);
    (a + b) / 2
}

/// Packs a bounded cell-local coordinate.
pub fn spatial_subsys_027(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!((0..65_536).contains(&b));
    a * 65_536 + b
}

/// Calculates the Manhattan ring number for an offset.
pub fn spatial_subsys_028(a: i64, b: i64) -> i64 {
    calc_manhattan_dist(0, 0, a, b)
}

/// Calculates the area of a nonnegative grid rectangle.
pub fn spatial_subsys_029(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a * b
}

/// Calculates the squared diagonal length of a grid cell.
pub fn spatial_subsys_030(a: i64, b: i64) -> i64 {
    a * a + b * b
}

/// Advances a ray by a nonnegative origin and step.
pub fn spatial_subsys_031(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a + b
}

/// Calculates remaining visibility after nonnegative occlusion.
pub fn spatial_subsys_032(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    if a >= b {
        a - b
    } else {
        0
    }
}

/// Calculates the shared length of two origin-aligned intervals.
pub fn spatial_subsys_033(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a.min(b)
}

/// Finds the grid bucket containing coordinate `a` at resolution `b`.
pub fn spatial_subsys_034(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b > 0);
    a / b
}

/// Calculates a nonnegative cell-local coordinate modulo `b`.
pub fn spatial_subsys_035(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b > 0);
    a % b
}

/// Returns a binary half-plane/sector selector.
pub fn spatial_subsys_036(a: i64, b: i64) -> i64 {
    i64::from(a < b)
}

/// Calculates the width of a row in a radius-`a` diamond at offset `b`.
pub fn spatial_subsys_037(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= -a);
    assert!(b <= a);
    2 * (a - b.abs()) + 1
}

/// Calculates the start offset for quadtree range `a` with stride `b + 1`.
pub fn spatial_subsys_038(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a * (b + 1)
}

/// Returns collision penetration bounded by both axis depths.
pub fn spatial_subsys_039(a: i64, b: i64) -> i64 {
    assert!(a >= 0);
    assert!(b >= 0);
    a.min(b)
}

/// Performs a Bresenham-style doubled raster-error update.
pub fn spatial_subsys_040(a: i64, b: i64) -> i64 {
    2 * a - b
}

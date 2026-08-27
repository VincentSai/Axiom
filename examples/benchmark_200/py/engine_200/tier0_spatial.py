"""Tier 0 spatial geometry and grid-math primitives.

The functions in this module are pure: they only calculate and return values.
Assertions document and enforce the input domains required by the benchmark
specification.
"""


_COORD_COMPONENT_BASE = 1 << 16
_COORD_COMPONENT_MASK = _COORD_COMPONENT_BASE - 1


def calc_manhattan_dist(x1: int, y1: int, x2: int, y2: int) -> int:
    """Return the Manhattan distance between two grid coordinates."""
    return abs(x1 - x2) + abs(y1 - y2)


def calc_chebyshev_dist(x1: int, y1: int, x2: int, y2: int) -> int:
    """Return the Chebyshev (king-move) distance between two points."""
    return max(abs(x1 - x2), abs(y1 - y2))


def calc_euclidean_sq(x1: int, y1: int, x2: int, y2: int) -> int:
    """Return squared Euclidean distance without computing a square root."""
    dx = x1 - x2
    dy = y1 - y2
    return dx * dx + dy * dy


def check_in_radius_sq(dist_sq: int, radius: int) -> bool:
    """Return whether a nonnegative squared distance is within ``radius``."""
    assert dist_sq >= 0, "dist_sq must be nonnegative"
    assert radius >= 0, "radius must be nonnegative"
    return dist_sq <= radius * radius


def bresenham_step_x(x1: int, x2: int) -> int:
    """Return the horizontal sign step from ``x1`` toward ``x2``."""
    return (x2 > x1) - (x2 < x1)


def bresenham_step_y(y1: int, y2: int) -> int:
    """Return the vertical sign step from ``y1`` toward ``y2``."""
    return (y2 > y1) - (y2 < y1)


def pack_coords_to_hash(x: int, y: int) -> int:
    """Pack nonnegative ``x`` and the low 16 bits of ``y`` into an integer."""
    assert x >= 0, "x must be nonnegative"
    assert y >= 0, "y must be nonnegative"
    return (x << 16) | (y & _COORD_COMPONENT_MASK)


def unpack_hash_x(hash_val: int) -> int:
    """Extract the high-coordinate X component from a packed hash."""
    assert hash_val >= 0, "hash_val must be nonnegative"
    return hash_val >> 16


def unpack_hash_y(hash_val: int) -> int:
    """Extract the low 16-bit Y component from a packed hash."""
    assert hash_val >= 0, "hash_val must be nonnegative"
    return hash_val & _COORD_COMPONENT_MASK


def wrap_map_coord_x(x: int, width: int) -> int:
    """Wrap ``x`` into the half-open interval ``[0, width)``."""
    assert width > 0, "width must be positive"
    return x % width


def wrap_map_coord_y(y: int, height: int) -> int:
    """Wrap ``y`` into the half-open interval ``[0, height)``."""
    assert height > 0, "height must be positive"
    return y % height


def clamp_to_bounds_x(x: int, width: int) -> int:
    """Clamp ``x`` into the half-open map interval ``[0, width)``."""
    assert width > 0, "width must be positive"
    return min(max(x, 0), width - 1)


def clamp_to_bounds_y(y: int, height: int) -> int:
    """Clamp ``y`` into the half-open map interval ``[0, height)``."""
    assert height > 0, "height must be positive"
    return min(max(y, 0), height - 1)


def check_aabb_overlap(
    min_x1: int,
    min_y1: int,
    max_x1: int,
    max_y1: int,
    min_x2: int,
    min_y2: int,
    max_x2: int,
    max_y2: int,
) -> bool:
    """Return whether two valid axis-aligned boxes overlap inclusively."""
    assert min_x1 <= max_x1, "first box must have min_x <= max_x"
    assert min_y1 <= max_y1, "first box must have min_y <= max_y"
    assert min_x2 <= max_x2, "second box must have min_x <= max_x"
    assert min_y2 <= max_y2, "second box must have min_y <= max_y"
    return (
        max_x1 >= min_x2
        and min_x1 <= max_x2
        and max_y1 >= min_y2
        and min_y1 <= max_y2
    )


def calc_grid_index(x: int, y: int, map_width: int) -> int:
    """Flatten a nonnegative grid coordinate into a row-major index."""
    assert x >= 0, "x must be nonnegative"
    assert y >= 0, "y must be nonnegative"
    assert map_width > 0, "map_width must be positive"
    return y * map_width + x


def calc_line_of_sight_cost(dist: int, terrain_density: int) -> int:
    """Return terrain-weighted line-of-sight energy cost, rounded down."""
    assert dist >= 0, "dist must be nonnegative"
    assert terrain_density >= 0, "terrain_density must be nonnegative"
    return dist * terrain_density // 100


def spatial_subsys_017(a: int, b: int) -> int:
    """Return a Cartesian quadrant code: NE=0, NW=1, SW=2, SE=3."""
    if a >= 0:
        return 0 if b >= 0 else 3
    return 1 if b >= 0 else 2


def spatial_subsys_018(a: int, b: int) -> int:
    """Return the diamond-field mask distance from the origin."""
    return calc_manhattan_dist(0, 0, a, b)


def spatial_subsys_019(a: int, b: int) -> int:
    """Return the farthest nonnegative ray-intersection depth."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return calc_chebyshev_dist(0, 0, a, b)


def spatial_subsys_020(a: int, b: int) -> int:
    """Return quadtree child ``b`` within parent-cell range ``a``."""
    assert a >= 0, "a must be nonnegative"
    assert 0 <= b < 4, "b must be a quadtree child index in [0, 4)"
    return a * 4 + b


def spatial_subsys_021(a: int, b: int) -> int:
    """Return the squared displacement of grid vector ``(a, b)``."""
    return a * a + b * b


def spatial_subsys_022(a: int, b: int) -> int:
    """Return the product of two nonnegative projection scales."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return a * b


def spatial_subsys_023(a: int, b: int) -> int:
    """Return the smaller of two nonnegative axis extents."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return min(a, b)


def spatial_subsys_024(a: int, b: int) -> int:
    """Return the sum of the normalized directions of two axes."""
    step_a = (a > 0) - (a < 0)
    step_b = (b > 0) - (b < 0)
    return step_a + step_b


def spatial_subsys_025(a: int, b: int) -> int:
    """Return the absolute span delta between two axis positions."""
    return abs(a - b)


def spatial_subsys_026(a: int, b: int) -> int:
    """Return the floor midpoint of a nonnegative interval."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return (a + b) // 2


def spatial_subsys_027(a: int, b: int) -> int:
    """Pack parent cell ``a`` and bounded local coordinate ``b``."""
    assert a >= 0, "a must be nonnegative"
    assert 0 <= b < _COORD_COMPONENT_BASE, "b must fit in 16 bits"
    return a * _COORD_COMPONENT_BASE + b


def spatial_subsys_028(a: int, b: int) -> int:
    """Return the Manhattan ring number for offset ``(a, b)``."""
    return calc_manhattan_dist(0, 0, a, b)


def spatial_subsys_029(a: int, b: int) -> int:
    """Return the area of a nonnegative grid rectangle."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return a * b


def spatial_subsys_030(a: int, b: int) -> int:
    """Return the squared diagonal length of a grid cell."""
    return a * a + b * b


def spatial_subsys_031(a: int, b: int) -> int:
    """Advance a nonnegative ray origin ``a`` by step ``b``."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return a + b


def spatial_subsys_032(a: int, b: int) -> int:
    """Return remaining visibility after nonnegative occlusion ``b``."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return max(a - b, 0)


def spatial_subsys_033(a: int, b: int) -> int:
    """Return the shared length of two origin-aligned intervals."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return min(a, b)


def spatial_subsys_034(a: int, b: int) -> int:
    """Return the grid bucket containing coordinate ``a`` at resolution ``b``."""
    assert a >= 0, "a must be nonnegative"
    assert b > 0, "b must be positive"
    return a // b


def spatial_subsys_035(a: int, b: int) -> int:
    """Return nonnegative cell-local coordinate ``a`` modulo ``b``."""
    assert a >= 0, "a must be nonnegative"
    assert b > 0, "b must be positive"
    return a % b


def spatial_subsys_036(a: int, b: int) -> int:
    """Return 1 when ``a`` lies below half-plane boundary ``b``, else 0."""
    return int(a < b)


def spatial_subsys_037(a: int, b: int) -> int:
    """Return row width in a radius-``a`` diamond at vertical offset ``b``."""
    assert a >= 0, "a must be nonnegative"
    assert -a <= b <= a, "b must lie within the diamond radius"
    return 2 * (a - abs(b)) + 1


def spatial_subsys_038(a: int, b: int) -> int:
    """Return the start offset for range ``a`` with stride ``b + 1``."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return a * (b + 1)


def spatial_subsys_039(a: int, b: int) -> int:
    """Return collision penetration bounded by both nonnegative depths."""
    assert a >= 0, "a must be nonnegative"
    assert b >= 0, "b must be nonnegative"
    return min(a, b)


def spatial_subsys_040(a: int, b: int) -> int:
    """Return a Bresenham-style doubled raster-error update."""
    return 2 * a - b

/**
 * Tier 0 spatial geometry and grid-math primitives.
 *
 * All public functions are pure and enforce the integer domains from the
 * benchmark contracts. Invalid inputs throw a RangeError.
 */

const COORD_MIN = -1_000_000;
const COORD_MAX = 1_000_000;
const COORD_COMPONENT_BASE = 65_536;
const COORD_COMPONENT_MAX = COORD_COMPONENT_BASE - 1;
const HASH_MAX = 4_294_967_295;
const DISTANCE_SQUARED_MAX = 1_000_000_000_000;

function requireIntegerInRange(
  value: number,
  name: string,
  minimum: number,
  maximum: number,
): void {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(
      `${name} must be an integer in [${minimum}, ${maximum}]`,
    );
  }
}

function requireCoordinate(value: number, name: string): void {
  requireIntegerInRange(value, name, COORD_MIN, COORD_MAX);
}

function requireNonnegativeCoordinate(value: number, name: string): void {
  requireIntegerInRange(value, name, 0, COORD_MAX);
}

function requirePositiveExtent(value: number, name: string): void {
  requireIntegerInRange(value, name, 1, COORD_MAX);
}

export function calc_manhattan_dist(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): number {
  requireCoordinate(x1, "x1");
  requireCoordinate(y1, "y1");
  requireCoordinate(x2, "x2");
  requireCoordinate(y2, "y2");
  return Math.abs(x1 - x2) + Math.abs(y1 - y2);
}

export function calc_chebyshev_dist(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): number {
  requireCoordinate(x1, "x1");
  requireCoordinate(y1, "y1");
  requireCoordinate(x2, "x2");
  requireCoordinate(y2, "y2");
  return Math.max(Math.abs(x1 - x2), Math.abs(y1 - y2));
}

export function calc_euclidean_sq(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
): number {
  requireCoordinate(x1, "x1");
  requireCoordinate(y1, "y1");
  requireCoordinate(x2, "x2");
  requireCoordinate(y2, "y2");
  const dx = x1 - x2;
  const dy = y1 - y2;
  return dx * dx + dy * dy;
}

export function check_in_radius_sq(
  dist_sq: number,
  radius: number,
): boolean {
  requireIntegerInRange(dist_sq, "dist_sq", 0, DISTANCE_SQUARED_MAX);
  requireNonnegativeCoordinate(radius, "radius");
  return dist_sq <= radius * radius;
}

export function bresenham_step_x(x1: number, x2: number): number {
  requireCoordinate(x1, "x1");
  requireCoordinate(x2, "x2");
  return x2 > x1 ? 1 : x2 < x1 ? -1 : 0;
}

export function bresenham_step_y(y1: number, y2: number): number {
  requireCoordinate(y1, "y1");
  requireCoordinate(y2, "y2");
  return y2 > y1 ? 1 : y2 < y1 ? -1 : 0;
}

export function pack_coords_to_hash(x: number, y: number): number {
  requireIntegerInRange(x, "x", 0, COORD_COMPONENT_MAX);
  requireIntegerInRange(y, "y", 0, COORD_COMPONENT_MAX);
  return x * COORD_COMPONENT_BASE + y;
}

export function unpack_hash_x(hash_val: number): number {
  requireIntegerInRange(hash_val, "hash_val", 0, HASH_MAX);
  return Math.floor(hash_val / COORD_COMPONENT_BASE);
}

export function unpack_hash_y(hash_val: number): number {
  requireIntegerInRange(hash_val, "hash_val", 0, HASH_MAX);
  return hash_val % COORD_COMPONENT_BASE;
}

export function wrap_map_coord_x(x: number, width: number): number {
  requireCoordinate(x, "x");
  requirePositiveExtent(width, "width");
  const remainder = x % width;
  return remainder < 0 ? remainder + width : remainder === 0 ? 0 : remainder;
}

export function wrap_map_coord_y(y: number, height: number): number {
  requireCoordinate(y, "y");
  requirePositiveExtent(height, "height");
  const remainder = y % height;
  return remainder < 0 ? remainder + height : remainder === 0 ? 0 : remainder;
}

export function clamp_to_bounds_x(x: number, width: number): number {
  requireCoordinate(x, "x");
  requirePositiveExtent(width, "width");
  return Math.min(Math.max(x, 0), width - 1);
}

export function clamp_to_bounds_y(y: number, height: number): number {
  requireCoordinate(y, "y");
  requirePositiveExtent(height, "height");
  return Math.min(Math.max(y, 0), height - 1);
}

export function check_aabb_overlap(
  min_x1: number,
  min_y1: number,
  max_x1: number,
  max_y1: number,
  min_x2: number,
  min_y2: number,
  max_x2: number,
  max_y2: number,
): boolean {
  requireCoordinate(min_x1, "min_x1");
  requireCoordinate(min_y1, "min_y1");
  requireCoordinate(max_x1, "max_x1");
  requireCoordinate(max_y1, "max_y1");
  requireCoordinate(min_x2, "min_x2");
  requireCoordinate(min_y2, "min_y2");
  requireCoordinate(max_x2, "max_x2");
  requireCoordinate(max_y2, "max_y2");

  if (min_x1 > max_x1 || min_y1 > max_y1) {
    throw new RangeError("the first AABB minimum must not exceed its maximum");
  }
  if (min_x2 > max_x2 || min_y2 > max_y2) {
    throw new RangeError("the second AABB minimum must not exceed its maximum");
  }

  return (
    max_x1 >= min_x2 &&
    min_x1 <= max_x2 &&
    max_y1 >= min_y2 &&
    min_y1 <= max_y2
  );
}

export function calc_grid_index(
  x: number,
  y: number,
  map_width: number,
): number {
  requirePositiveExtent(map_width, "map_width");
  requireIntegerInRange(x, "x", 0, map_width - 1);
  requireNonnegativeCoordinate(y, "y");
  return y * map_width + x;
}

export function calc_line_of_sight_cost(
  dist: number,
  terrain_density: number,
): number {
  requireNonnegativeCoordinate(dist, "dist");
  requireNonnegativeCoordinate(terrain_density, "terrain_density");
  return Math.floor((dist * terrain_density) / 100);
}

export function spatial_subsys_017(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  if (a >= 0) {
    return b >= 0 ? 0 : 3;
  }
  return b >= 0 ? 1 : 2;
}

export function spatial_subsys_018(a: number, b: number): number {
  return calc_manhattan_dist(0, 0, a, b);
}

export function spatial_subsys_019(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return Math.max(a, b);
}

export function spatial_subsys_020(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireIntegerInRange(b, "b", 0, 3);
  return a * 4 + b;
}

export function spatial_subsys_021(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  return a * a + b * b;
}

export function spatial_subsys_022(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return a * b;
}

export function spatial_subsys_023(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return Math.min(a, b);
}

export function spatial_subsys_024(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  return Math.sign(a) + Math.sign(b);
}

export function spatial_subsys_025(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  return Math.abs(a - b);
}

export function spatial_subsys_026(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return Math.floor((a + b) / 2);
}

export function spatial_subsys_027(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireIntegerInRange(b, "b", 0, COORD_COMPONENT_MAX);
  return a * COORD_COMPONENT_BASE + b;
}

export function spatial_subsys_028(a: number, b: number): number {
  return calc_manhattan_dist(0, 0, a, b);
}

export function spatial_subsys_029(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return a * b;
}

export function spatial_subsys_030(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  return a * a + b * b;
}

export function spatial_subsys_031(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return a + b;
}

export function spatial_subsys_032(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return Math.max(a - b, 0);
}

export function spatial_subsys_033(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return Math.min(a, b);
}

export function spatial_subsys_034(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requirePositiveExtent(b, "b");
  return Math.floor(a / b);
}

export function spatial_subsys_035(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requirePositiveExtent(b, "b");
  return a % b;
}

export function spatial_subsys_036(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  return a < b ? 1 : 0;
}

export function spatial_subsys_037(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireIntegerInRange(b, "b", -a, a);
  return 2 * (a - Math.abs(b)) + 1;
}

export function spatial_subsys_038(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return a * (b + 1);
}

export function spatial_subsys_039(a: number, b: number): number {
  requireNonnegativeCoordinate(a, "a");
  requireNonnegativeCoordinate(b, "b");
  return Math.min(a, b);
}

export function spatial_subsys_040(a: number, b: number): number {
  requireCoordinate(a, "a");
  requireCoordinate(b, "b");
  return 2 * a - b;
}

# Graph Report - G:\Workspace\Axiom\examples\benchmark_200\py  (2026-08-27)

## Corpus Check
- Corpus is ~5,487 words - fits in a single context window. You may not need a graph.

## Summary
- 299 nodes · 521 edges · 25 communities (24 shown, 1 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 8 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- World Simulation
- Spatial Geometry
- AI and Loot
- Combat and Status
- Domain Models
- Spatial Geometry
- Engine Test Suite
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry
- Spatial Geometry

## God Nodes (most connected - your core abstractions)
1. `_validate_world_clock()` - 17 edges
2. `wrap_map_coord_x()` - 11 edges
3. `calc_manhattan_dist()` - 9 edges
4. `world_engine_194()` - 8 edges
5. `world_engine_195()` - 8 edges
6. `world_engine_197()` - 8 edges
7. `clamp_to_bounds_x()` - 7 edges
8. `simulate_single_turn_exchange()` - 7 edges
9. `world_engine_192()` - 7 edges
10. `world_engine_199()` - 7 edges

## Surprising Connections (you probably didn't know these)
- `resolve_crit_hit()` --calls--> `calc_armor_damage_reduction()`  [EXTRACTED]
  engine_200/tier2_combat.py → engine_200/tier1_stats.py
- `world_engine_187()` --calls--> `pack_coords_to_hash()`  [EXTRACTED]
  engine_200/tier4_world.py → engine_200/tier0_spatial.py
- `stat_subsys_060()` --calls--> `wrap_map_coord_x()`  [EXTRACTED]
  engine_200/tier1_stats.py → engine_200/tier0_spatial.py
- `world_engine_189()` --calls--> `wrap_map_coord_y()`  [EXTRACTED]
  engine_200/tier4_world.py → engine_200/tier0_spatial.py
- `world_engine_195()` --calls--> `clamp_to_bounds_x()`  [EXTRACTED]
  engine_200/tier4_world.py → engine_200/tier0_spatial.py

## Import Cycles
- None detected.

## Communities (25 total, 1 thin omitted)

### Community 0 - "World Simulation"
Cohesion: 0.06
Nodes (62): calc_grid_index(), calc_manhattan_dist(), Flatten a nonnegative grid coordinate into a row-major index., Return the Manhattan distance between two grid coordinates., Wrap ``x`` into the half-open interval ``[0, width)``., wrap_map_coord_x(), calc_armor_damage_reduction(), calc_lifesteal_conversion() (+54 more)

### Community 1 - "Spatial Geometry"
Cohesion: 0.05
Nodes (40): calc_chebyshev_dist(), clamp_to_bounds_x(), Tier 0 spatial geometry and grid-math primitives. The functions in this module…, Return the farthest nonnegative ray-intersection depth., Return the smaller of two nonnegative axis extents., Return the Chebyshev (king-move) distance between two points., Advance a nonnegative ray origin ``a`` by step ``b``., Return the grid bucket containing coordinate ``a`` at resolution ``b``. (+32 more)

### Community 2 - "AI and Loot"
Cohesion: 0.08
Nodes (23): check_in_radius_sq(), Return quadtree child ``b`` within parent-cell range ``a``., Return the squared displacement of grid vector ``(a, b)``., Return whether a nonnegative squared distance is within ``radius``., spatial_subsys_020(), spatial_subsys_021(), stat_subsys_054(), stat_subsys_060() (+15 more)

### Community 3 - "Combat and Status"
Cohesion: 0.08
Nodes (20): Return the squared diagonal length of a grid cell., Return the start offset for range ``a`` with stride ``b + 1``., Return collision penetration bounded by both nonnegative depths., Return a Bresenham-style doubled raster-error update., spatial_subsys_030(), spatial_subsys_038(), spatial_subsys_039(), spatial_subsys_040() (+12 more)

### Community 4 - "Domain Models"
Cohesion: 0.13
Nodes (17): Public API for the pure five-tier roguelike engine benchmark., BaseAttributes, DerivedStats, Entity, Point, Domain data models for the roguelike world simulation engine., A two-dimensional velocity vector., An inclusive axis-aligned rectangle. (+9 more)

### Community 5 - "Spatial Geometry"
Cohesion: 0.12
Nodes (17): pack_coords_to_hash(), Pack nonnegative ``x`` and the low 16 bits of ``y`` into an integer., Extract the high-coordinate X component from a packed hash., Extract the low 16-bit Y component from a packed hash., unpack_hash_x(), unpack_hash_y(), stat_subsys_057(), stat_subsys_058() (+9 more)

### Community 7 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): bresenham_step_x(), Return the horizontal sign step from ``x1`` toward ``x2``., stat_subsys_055(), combat_subsys_105(), ai_loot_subsys_152()

### Community 8 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): bresenham_step_y(), Return the vertical sign step from ``y1`` toward ``y2``., stat_subsys_056(), combat_subsys_106(), ai_loot_subsys_153()

### Community 9 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): calc_euclidean_sq(), Return squared Euclidean distance without computing a square root., stat_subsys_053(), combat_subsys_103(), ai_loot_subsys_150()

### Community 10 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): calc_line_of_sight_cost(), Return terrain-weighted line-of-sight energy cost, rounded down., stat_subsys_066(), combat_subsys_116(), ai_loot_subsys_163()

### Community 11 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): check_aabb_overlap(), Return whether two valid axis-aligned boxes overlap inclusively., stat_subsys_064(), combat_subsys_114(), ai_loot_subsys_161()

### Community 12 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): clamp_to_bounds_y(), Clamp ``y`` into the half-open map interval ``[0, height)``., stat_subsys_063(), combat_subsys_113(), ai_loot_subsys_160()

### Community 13 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return a Cartesian quadrant code: NE=0, NW=1, SW=2, SE=3., spatial_subsys_017(), stat_subsys_067(), combat_subsys_117(), ai_loot_subsys_164()

### Community 14 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the diamond-field mask distance from the origin., spatial_subsys_018(), stat_subsys_068(), combat_subsys_118(), ai_loot_subsys_165()

### Community 15 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the product of two nonnegative projection scales., spatial_subsys_022(), stat_subsys_072(), combat_subsys_122(), ai_loot_subsys_169()

### Community 16 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the sum of the normalized directions of two axes., spatial_subsys_024(), stat_subsys_074(), combat_subsys_124(), ai_loot_subsys_171()

### Community 17 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the absolute span delta between two axis positions., spatial_subsys_025(), stat_subsys_075(), combat_subsys_125(), ai_loot_subsys_172()

### Community 18 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the floor midpoint of a nonnegative interval., spatial_subsys_026(), stat_subsys_076(), combat_subsys_126(), ai_loot_subsys_173()

### Community 19 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Pack parent cell ``a`` and bounded local coordinate ``b``., spatial_subsys_027(), stat_subsys_077(), combat_subsys_127(), ai_loot_subsys_174()

### Community 20 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the Manhattan ring number for offset ``(a, b)``., spatial_subsys_028(), stat_subsys_078(), combat_subsys_128(), ai_loot_subsys_175()

### Community 21 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the area of a nonnegative grid rectangle., spatial_subsys_029(), stat_subsys_079(), combat_subsys_129(), ai_loot_subsys_176()

### Community 22 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return remaining visibility after nonnegative occlusion ``b``., spatial_subsys_032(), stat_subsys_082(), combat_subsys_132(), ai_loot_subsys_179()

### Community 23 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Return the shared length of two origin-aligned intervals., spatial_subsys_033(), stat_subsys_083(), combat_subsys_133(), ai_loot_subsys_180()

### Community 24 - "Spatial Geometry"
Cohesion: 0.40
Nodes (5): Wrap ``y`` into the half-open interval ``[0, height)``., wrap_map_coord_y(), stat_subsys_061(), combat_subsys_111(), ai_loot_subsys_158()

## Knowledge Gaps
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `DomainModelTests` connect `Domain Models` to `Engine Test Suite`?**
  _High betweenness centrality (0.065) - this node is a cross-community bridge._
- **Should `World Simulation` be split into smaller, more focused modules?**
  _Cohesion score 0.06093189964157706 - nodes in this community are weakly interconnected._
- **Should `Spatial Geometry` be split into smaller, more focused modules?**
  _Cohesion score 0.05410628019323672 - nodes in this community are weakly interconnected._
- **Should `AI and Loot` be split into smaller, more focused modules?**
  _Cohesion score 0.07692307692307693 - nodes in this community are weakly interconnected._
- **Should `Combat and Status` be split into smaller, more focused modules?**
  _Cohesion score 0.07692307692307693 - nodes in this community are weakly interconnected._
- **Should `Domain Models` be split into smaller, more focused modules?**
  _Cohesion score 0.12857142857142856 - nodes in this community are weakly interconnected._
- **Should `Spatial Geometry` be split into smaller, more focused modules?**
  _Cohesion score 0.11764705882352941 - nodes in this community are weakly interconnected._
use roguelike_200_rs::*;

#[test]
fn domain_models_are_public_value_types() {
    let entity = Entity {
        id: 7,
        pos: Point { x: 3, y: 4 },
        base_attr: BaseAttributes {
            strength: 10,
            agility: 11,
            intelligence: 12,
            vitality: 13,
        },
        stats: DerivedStats {
            max_hp: 100,
            max_mana: 80,
            max_stamina: 60,
            phys_atk: 20,
            magic_atk: 21,
            armor: 22,
            magic_res: 23,
            crit_rate: 500,
            crit_dmg_pct: 150,
            dodge_rate: 300,
            move_speed: 10,
        },
        status: StatusEffect {
            bleed_stacks: 1,
            poison_duration: 2,
            burn_damage: 3,
            freeze_stacks: 4,
            shield_hp: 5,
        },
        curr_hp: 90,
        curr_mana: 70,
        curr_stamina: 50,
        gold: 40,
        level: 3,
        exp: 200,
        weight: 9,
        max_weight: 20,
        is_alive: true,
    };

    assert_eq!(entity.pos, Point { x: 3, y: 4 });
    assert_eq!(Velocity { dx: 1, dy: -1 }.dy, -1);
    assert_eq!(
        Rect {
            min_x: 0,
            min_y: 1,
            max_x: 2,
            max_y: 3
        }
        .max_y,
        3
    );
}

#[test]
fn tier0_covers_functions_001_through_040() {
    assert_eq!(calc_manhattan_dist(1, 2, 4, 6), 7);
    assert_eq!(calc_chebyshev_dist(1, 2, 4, 6), 4);
    assert_eq!(calc_euclidean_sq(1, 2, 4, 6), 25);
    assert!(check_in_radius_sq(25, 5));
    assert_eq!(bresenham_step_x(1, 4), 1);
    assert_eq!(bresenham_step_y(4, 1), -1);
    assert_eq!(pack_coords_to_hash(12, 34), 786_466);
    assert_eq!(unpack_hash_x(786_466), 12);
    assert_eq!(unpack_hash_y(786_466), 34);
    assert_eq!(wrap_map_coord_x(-1, 10), 9);
    assert_eq!(wrap_map_coord_y(11, 10), 1);
    assert_eq!(clamp_to_bounds_x(-3, 10), 0);
    assert_eq!(clamp_to_bounds_y(12, 10), 9);
    assert!(check_aabb_overlap(0, 0, 5, 5, 5, 5, 8, 8));
    assert_eq!(calc_grid_index(3, 2, 10), 23);
    assert_eq!(calc_line_of_sight_cost(10, 25), 2);

    let functions: [fn(i64, i64) -> i64; 24] = [
        spatial_subsys_017,
        spatial_subsys_018,
        spatial_subsys_019,
        spatial_subsys_020,
        spatial_subsys_021,
        spatial_subsys_022,
        spatial_subsys_023,
        spatial_subsys_024,
        spatial_subsys_025,
        spatial_subsys_026,
        spatial_subsys_027,
        spatial_subsys_028,
        spatial_subsys_029,
        spatial_subsys_030,
        spatial_subsys_031,
        spatial_subsys_032,
        spatial_subsys_033,
        spatial_subsys_034,
        spatial_subsys_035,
        spatial_subsys_036,
        spatial_subsys_037,
        spatial_subsys_038,
        spatial_subsys_039,
        spatial_subsys_040,
    ];
    let expected = [
        0, 6, 4, 18, 20, 8, 2, 2, 2, 3, 262_146, 6, 8, 20, 6, 2, 2, 2, 0, 0, 5, 12, 2, 6,
    ];
    let mut inputs = [(4, 2); 24];
    inputs[9] = (2, 4);

    for ((function, expected), (a, b)) in functions.into_iter().zip(expected).zip(inputs) {
        assert_eq!(function(a, b), expected);
    }
}

#[test]
fn tier1_covers_functions_041_through_090() {
    assert_eq!(calc_max_hp_from_vit(100, 10, 5), 210);
    assert_eq!(calc_max_mana_from_int(100, 10), 250);
    assert_eq!(calc_phys_atk_from_str(10, 6), 23);
    assert_eq!(calc_armor_damage_reduction(100, 100), 50);
    assert_eq!(calc_magic_res_reduction(200, 100), 100);
    assert_eq!(calc_elemental_multiplier(0, 1, 100), 150);
    assert_eq!(calc_crit_chance_clamped(9_500, 100), 10_000);
    assert_eq!(calc_effective_penetration(100, 20, 10), 70);
    assert_eq!(calc_overburden_penalty(11, 10, 100), 50);
    assert_eq!(calc_lifesteal_conversion(200, 25), 50);

    assert_eq!(stat_subsys_051(4, 2), 6);
    assert_eq!(stat_subsys_052(4, 2), 4);
    assert_eq!(stat_subsys_053(4, 2), 20);
    assert_eq!(stat_subsys_054(4, 2), 1);
    assert_eq!(stat_subsys_055(4, 2), 4);
    assert_eq!(stat_subsys_056(4, 2), 4);
    assert_eq!(stat_subsys_057(4, 2), 262_146);
    assert_eq!(stat_subsys_058(4, 2), 2);
    assert_eq!(stat_subsys_059(4, 2), 6);
    assert_eq!(stat_subsys_060(4, 2), 1);
    assert_eq!(stat_subsys_061(4, 2), 1);
    assert_eq!(stat_subsys_062(4, 2), 2);
    assert_eq!(stat_subsys_063(4, 2), 2);
    assert_eq!(stat_subsys_064(4, 2), 0);
    assert_eq!(stat_subsys_065(4, 2), 14);
    assert_eq!(stat_subsys_066(4, 2), 0);
    assert_eq!(stat_subsys_067(4, 2), 4);
    assert_eq!(stat_subsys_068(4, 2), 10);
    assert_eq!(stat_subsys_069(4, 2), 8);
    assert_eq!(stat_subsys_070(4, 2), 22);
    assert_eq!(stat_subsys_071(4, 2), 24);
    assert_eq!(stat_subsys_072(4, 2), 12);
    assert_eq!(stat_subsys_073(4, 2), 6);
    assert_eq!(stat_subsys_074(4, 2), 6);
    assert_eq!(stat_subsys_075(4, 2), 6);
    assert_eq!(stat_subsys_076(2, 4), 5);
    assert_eq!(stat_subsys_077(4, 2), 262_150);
    assert_eq!(stat_subsys_078(4, 2), 10);
    assert_eq!(stat_subsys_079(4, 2), 12);
    assert_eq!(stat_subsys_080(4, 2), 24);
    assert_eq!(stat_subsys_081(4, 2), 10);
    assert_eq!(stat_subsys_082(4, 2), 6);
    assert_eq!(stat_subsys_083(4, 2), 6);
    assert_eq!(stat_subsys_084(4, 2), 5);
    assert_eq!(stat_subsys_085(4, 2), 5);
    assert_eq!(stat_subsys_086(4, 2), 4);
    assert_eq!(stat_subsys_087(4, 2), 9);
    assert_eq!(stat_subsys_088(4, 2), 16);
    assert_eq!(stat_subsys_089(4, 2), 6);
    assert_eq!(stat_subsys_090(4, 2), 10);
}

#[test]
fn tier2_covers_functions_091_through_140() {
    assert_eq!(resolve_physical_hit(100, 100), 50);
    assert_eq!(resolve_crit_hit(100, true, 150), 150);
    assert_eq!(apply_dual_shield_absorption(100, 30), 70);
    assert_eq!(apply_bleed_stack_tick(3, false), 15);
    assert_eq!(apply_poison_exponential_decay(10, 3), 20);
    assert_eq!(apply_burn_max_hp_tick(1_000, 5), 35);
    assert!(check_freeze_threshold(5));
    assert_eq!(calc_stamina_skill_cost(100, 25), 75);
    assert_eq!(calc_mana_overload_burst(99), 9);
    assert!(calc_poise_break_stun(10, 10));

    let functions: [fn(i64, i64) -> i64; 40] = [
        combat_subsys_101,
        combat_subsys_102,
        combat_subsys_103,
        combat_subsys_104,
        combat_subsys_105,
        combat_subsys_106,
        combat_subsys_107,
        combat_subsys_108,
        combat_subsys_109,
        combat_subsys_110,
        combat_subsys_111,
        combat_subsys_112,
        combat_subsys_113,
        combat_subsys_114,
        combat_subsys_115,
        combat_subsys_116,
        combat_subsys_117,
        combat_subsys_118,
        combat_subsys_119,
        combat_subsys_120,
        combat_subsys_121,
        combat_subsys_122,
        combat_subsys_123,
        combat_subsys_124,
        combat_subsys_125,
        combat_subsys_126,
        combat_subsys_127,
        combat_subsys_128,
        combat_subsys_129,
        combat_subsys_130,
        combat_subsys_131,
        combat_subsys_132,
        combat_subsys_133,
        combat_subsys_134,
        combat_subsys_135,
        combat_subsys_136,
        combat_subsys_137,
        combat_subsys_138,
        combat_subsys_139,
        combat_subsys_140,
    ];

    for function in functions {
        assert!(function(10, 0) >= 1);
    }
}

#[test]
fn tier3_covers_functions_141_through_180() {
    assert_eq!(calc_threat_decay_by_dist(100, 5), 66);
    assert_eq!(evaluate_ai_behavior_state(19, 0), 1);
    assert_eq!(calc_weighted_loot_tier(9_900), 5);
    assert_eq!(scale_drop_by_luck(9_000, 100), 9_200);
    assert_eq!(calc_exp_gain_with_level_diff(100, 20, 9), 10);
    assert_eq!(calc_exp_to_next_level(3), 2_400);
    assert_eq!(calc_shop_inflation_price(100, 5), 110);

    let functions: [fn(i64, i64) -> i64; 33] = [
        ai_loot_subsys_148,
        ai_loot_subsys_149,
        ai_loot_subsys_150,
        ai_loot_subsys_151,
        ai_loot_subsys_152,
        ai_loot_subsys_153,
        ai_loot_subsys_154,
        ai_loot_subsys_155,
        ai_loot_subsys_156,
        ai_loot_subsys_157,
        ai_loot_subsys_158,
        ai_loot_subsys_159,
        ai_loot_subsys_160,
        ai_loot_subsys_161,
        ai_loot_subsys_162,
        ai_loot_subsys_163,
        ai_loot_subsys_164,
        ai_loot_subsys_165,
        ai_loot_subsys_166,
        ai_loot_subsys_167,
        ai_loot_subsys_168,
        ai_loot_subsys_169,
        ai_loot_subsys_170,
        ai_loot_subsys_171,
        ai_loot_subsys_172,
        ai_loot_subsys_173,
        ai_loot_subsys_174,
        ai_loot_subsys_175,
        ai_loot_subsys_176,
        ai_loot_subsys_177,
        ai_loot_subsys_178,
        ai_loot_subsys_179,
        ai_loot_subsys_180,
    ];

    for function in functions {
        assert!(function(10, 5) >= 10);
    }
}

#[test]
fn tier4_covers_functions_181_through_200() {
    assert_eq!(determine_turn_initiative(10, 5), 25);
    assert_eq!(resolve_boss_phase_transition(67), 1);
    assert_eq!(resolve_boss_phase_transition(66), 2);
    assert_eq!(resolve_boss_phase_transition(33), 2);
    assert_eq!(resolve_boss_phase_transition(32), 3);
    assert_eq!(simulate_single_turn_exchange(100, 100, true), 105);
    assert_eq!(resolve_terrain_hazard_tick(0, 1_000), 0);
    assert_eq!(resolve_terrain_hazard_tick(1, 1_000), 60);
    assert_eq!(resolve_terrain_hazard_tick(2, 1_000), 0);
    assert_eq!(resolve_terrain_hazard_tick(3, 1_000), 0);

    assert_eq!(world_engine_185(25, 10), 2);
    assert_eq!(world_engine_186(25, 10), 3);
    assert_eq!(world_engine_187(25, 10), 1_638_410);
    assert_eq!(world_engine_188(25, 10), 2);
    assert_eq!(world_engine_189(25, 10), 5);
    assert_eq!(world_engine_190(25, 10), 10);
    assert_eq!(world_engine_191(25, 10), 2);
    assert_eq!(world_engine_192(25, 10), 11);
    assert_eq!(world_engine_193(25, 10), 9);
    assert_eq!(world_engine_194(25, 10), 8);
    assert_eq!(world_engine_195(25, 10), 1);
    assert_eq!(world_engine_196(25, 10), 2_400);
    assert_eq!(world_engine_197(25, 10), 21);
    assert_eq!(world_engine_198(25, 10), 35);
    assert_eq!(world_engine_199(25, 10), 11);
    assert_eq!(world_engine_200(25, 10), 52);
}

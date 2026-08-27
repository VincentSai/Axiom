use roguelike_200_rs::*;

type EngineFunction = fn(i64, i64) -> i64;

fn assert_panics(function: EngineFunction, first: i64, second: i64) {
    assert!(std::panic::catch_unwind(|| function(first, second)).is_err());
}

const MATH_FUNCTIONS: &[EngineFunction] = &[
    math_func_001,
    math_func_002,
    math_func_003,
    math_func_004,
    math_func_005,
    math_func_006,
    math_func_007,
    math_func_008,
    math_func_009,
    math_func_010,
    math_func_011,
    math_func_012,
    math_func_013,
    math_func_014,
    math_func_015,
    math_func_016,
    math_func_017,
    math_func_018,
    math_func_019,
    math_func_020,
    math_func_021,
    math_func_022,
    math_func_023,
    math_func_024,
    math_func_025,
    math_func_026,
    math_func_027,
    math_func_028,
    math_func_029,
    math_func_030,
    math_func_031,
    math_func_032,
    math_func_033,
    math_func_034,
    math_func_035,
    math_func_036,
    math_func_037,
    math_func_038,
    math_func_039,
    math_func_040,
];

const STAT_FUNCTIONS: &[EngineFunction] = &[
    stat_func_041,
    stat_func_042,
    stat_func_043,
    stat_func_044,
    stat_func_045,
    stat_func_046,
    stat_func_047,
    stat_func_048,
    stat_func_049,
    stat_func_050,
    stat_func_051,
    stat_func_052,
    stat_func_053,
    stat_func_054,
    stat_func_055,
    stat_func_056,
    stat_func_057,
    stat_func_058,
    stat_func_059,
    stat_func_060,
    stat_func_061,
    stat_func_062,
    stat_func_063,
    stat_func_064,
    stat_func_065,
    stat_func_066,
    stat_func_067,
    stat_func_068,
    stat_func_069,
    stat_func_070,
    stat_func_071,
    stat_func_072,
    stat_func_073,
    stat_func_074,
    stat_func_075,
    stat_func_076,
    stat_func_077,
    stat_func_078,
    stat_func_079,
    stat_func_080,
    stat_func_081,
    stat_func_082,
    stat_func_083,
    stat_func_084,
    stat_func_085,
    stat_func_086,
    stat_func_087,
    stat_func_088,
    stat_func_089,
    stat_func_090,
];

const COMBAT_FUNCTIONS: &[EngineFunction] = &[
    combat_func_091,
    combat_func_092,
    combat_func_093,
    combat_func_094,
    combat_func_095,
    combat_func_096,
    combat_func_097,
    combat_func_098,
    combat_func_099,
    combat_func_100,
    combat_func_101,
    combat_func_102,
    combat_func_103,
    combat_func_104,
    combat_func_105,
    combat_func_106,
    combat_func_107,
    combat_func_108,
    combat_func_109,
    combat_func_110,
    combat_func_111,
    combat_func_112,
    combat_func_113,
    combat_func_114,
    combat_func_115,
    combat_func_116,
    combat_func_117,
    combat_func_118,
    combat_func_119,
    combat_func_120,
    combat_func_121,
    combat_func_122,
    combat_func_123,
    combat_func_124,
    combat_func_125,
    combat_func_126,
    combat_func_127,
    combat_func_128,
    combat_func_129,
    combat_func_130,
    combat_func_131,
    combat_func_132,
    combat_func_133,
    combat_func_134,
    combat_func_135,
    combat_func_136,
    combat_func_137,
    combat_func_138,
    combat_func_139,
    combat_func_140,
];

const WORLD_FUNCTIONS: &[EngineFunction] = &[
    world_func_141,
    world_func_142,
    world_func_143,
    world_func_144,
    world_func_145,
    world_func_146,
    world_func_147,
    world_func_148,
    world_func_149,
    world_func_150,
    world_func_151,
    world_func_152,
    world_func_153,
    world_func_154,
    world_func_155,
    world_func_156,
    world_func_157,
    world_func_158,
    world_func_159,
    world_func_160,
    world_func_161,
    world_func_162,
    world_func_163,
    world_func_164,
    world_func_165,
    world_func_166,
    world_func_167,
    world_func_168,
    world_func_169,
    world_func_170,
    world_func_171,
    world_func_172,
    world_func_173,
    world_func_174,
    world_func_175,
    world_func_176,
    world_func_177,
    world_func_178,
    world_func_179,
    world_func_180,
];

const ENGINE_FUNCTIONS: &[EngineFunction] = &[
    engine_func_181,
    engine_func_182,
    engine_func_183,
    engine_func_184,
    engine_func_185,
    engine_func_186,
    engine_func_187,
    engine_func_188,
    engine_func_189,
    engine_func_190,
    engine_func_191,
    engine_func_192,
    engine_func_193,
    engine_func_194,
    engine_func_195,
    engine_func_196,
    engine_func_197,
    engine_func_198,
    engine_func_199,
    engine_func_200,
];

#[test]
fn verifies_all_math_functions() {
    assert_eq!(MATH_FUNCTIONS.len(), 40);
    for &function in MATH_FUNCTIONS {
        assert_eq!(function(7, 5), 12);
        assert_eq!(function(0, 0), 0);
        assert_panics(function, -1, 0);
        assert_panics(function, 0, -1);
    }
}

#[test]
fn verifies_all_stat_functions() {
    assert_eq!(STAT_FUNCTIONS.len(), 50);
    for &function in STAT_FUNCTIONS {
        assert_eq!(function(10, 4), 14);
        assert_eq!(function(0, 0), 0);
        assert_panics(function, -1, 0);
        assert_panics(function, 0, -1);
    }
}

#[test]
fn verifies_all_combat_functions() {
    assert_eq!(COMBAT_FUNCTIONS.len(), 50);
    for &function in COMBAT_FUNCTIONS {
        assert_eq!(function(12, 5), 7);
        assert_eq!(function(5, 5), 1);
        assert_eq!(function(3, 8), 1);
        assert_panics(function, -1, 0);
        assert_panics(function, 0, -1);
    }
}

#[test]
fn verifies_all_world_functions() {
    assert_eq!(WORLD_FUNCTIONS.len(), 40);
    for &function in WORLD_FUNCTIONS {
        assert_eq!(function(10, 4), 14);
        assert_eq!(function(10, 0), 11);
        assert_panics(function, -1, 0);
        assert_panics(function, 0, -1);
    }
}

#[test]
fn verifies_all_engine_functions() {
    assert_eq!(ENGINE_FUNCTIONS.len(), 20);
    for &function in ENGINE_FUNCTIONS {
        assert_eq!(function(17, 5), 2);
        assert_eq!(function(0, 1), 0);
        assert_panics(function, -1, 5);
        assert_panics(function, 5, 0);
        assert_panics(function, 5, -1);
    }
}

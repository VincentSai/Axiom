use roguelike_benchmark_rs::*;

#[test]
fn test_movement() {
    let p = Point { x: 0, y: 0 };
    assert_eq!(try_move(&p, 1, 0, 10, 10), Point { x: 1, y: 0 });
    assert_eq!(try_move(&p, -1, 0, 10, 10), Point { x: 0, y: 0 });
}

#[test]
fn test_combat() {
    assert_eq!(calculate_damage(10, 3), 7);
    assert_eq!(calculate_damage(2, 5), 1);
    assert_eq!(apply_damage(20, 7), 13);
    assert_eq!(apply_damage(5, 10), 0);
}

#[test]
fn test_level_up() {
    let p = Player {
        id: 1,
        pos: Point { x: 0, y: 0 },
        stats: Stats { hp: 20, max_hp: 20, attack: 5, defense: 2 },
        exp: 0,
        level: 1,
    };
    let next_p = level_up(&p, 120);
    assert_eq!(next_p.level, 2);
    assert_eq!(next_p.stats.max_hp, 30);
}

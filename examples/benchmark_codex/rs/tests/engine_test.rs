use roguelike_benchmark_rs::{
    apply_damage, calculate_damage, level_up, try_move, Item, Monster, Player, Point, Stats,
};

#[test]
fn domain_entities_expose_the_specified_data() {
    let item = Item {
        id: 7,
        name: "Health Potion".to_owned(),
        value: 25,
    };
    let monster = Monster {
        id: 11,
        pos: Point { x: 3, y: 4 },
        stats: Stats {
            hp: 12,
            max_hp: 12,
            attack: 4,
            defense: 1,
        },
        is_alive: true,
    };
    let player = Player {
        id: 1,
        pos: Point { x: 0, y: 0 },
        stats: Stats {
            hp: 20,
            max_hp: 20,
            attack: 5,
            defense: 2,
        },
        exp: 0,
        level: 1,
    };

    assert_eq!(item.name, "Health Potion");
    assert_eq!(monster.pos, Point { x: 3, y: 4 });
    assert!(monster.is_alive);
    assert_eq!(player.stats.max_hp, 20);
}

#[test]
fn movement_returns_the_destination_when_it_is_in_bounds() {
    let position = Point { x: 2, y: 2 };

    assert_eq!(try_move(&position, 1, 0, 5, 5), Point { x: 3, y: 2 });
}

#[test]
fn movement_keeps_the_position_when_the_destination_is_out_of_bounds() {
    let position = Point { x: 0, y: 0 };

    assert_eq!(try_move(&position, -1, 0, 5, 5), position);
}

#[test]
fn damage_uses_defense_with_a_minimum_of_one() {
    assert_eq!((calculate_damage(10, 3), calculate_damage(2, 5)), (7, 1));
}

#[test]
fn applied_damage_reduces_hp_without_going_below_zero() {
    assert_eq!((apply_damage(20, 7), apply_damage(5, 10)), (13, 0));
}

#[test]
fn enough_experience_levels_up_and_improves_stats() {
    let player = Player {
        id: 1,
        pos: Point { x: 0, y: 0 },
        stats: Stats {
            hp: 12,
            max_hp: 20,
            attack: 5,
            defense: 2,
        },
        exp: 0,
        level: 1,
    };

    assert_eq!(
        level_up(&player, 120),
        Player {
            id: 1,
            pos: Point { x: 0, y: 0 },
            stats: Stats {
                hp: 30,
                max_hp: 30,
                attack: 7,
                defense: 3,
            },
            exp: 20,
            level: 2,
        }
    );
}

#[test]
fn insufficient_experience_accumulates_without_leveling_up() {
    let player = Player {
        id: 1,
        pos: Point { x: 0, y: 0 },
        stats: Stats {
            hp: 20,
            max_hp: 20,
            attack: 5,
            defense: 2,
        },
        exp: 30,
        level: 1,
    };

    assert_eq!(level_up(&player, 40), Player { exp: 70, ..player });
}

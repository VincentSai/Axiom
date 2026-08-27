export interface Point {
    readonly x: number;
    readonly y: number;
}

export interface Stats {
    readonly hp: number;
    readonly max_hp: number;
    readonly attack: number;
    readonly defense: number;
}

export interface Item {
    readonly id: number;
    readonly name: string;
    readonly value: number;
}

export interface Player {
    readonly id: number;
    readonly pos: Point;
    readonly stats: Stats;
    readonly exp: number;
    readonly level: number;
}

export function tryMove(pos: Point, dx: number, dy: number, mapWidth: number, mapHeight: number): Point {
    const newX = pos.x + dx;
    const newY = pos.y + dy;
    if (newX >= 0 && newX < mapWidth && newY >= 0 && newY < mapHeight) {
        return { x: newX, y: newY };
    }
    return pos;
}

export function calculateDamage(attackerAtk: number, defenderDef: number): number {
    const raw = attackerAtk - defenderDef;
    return raw > 1 ? raw : 1;
}

export function applyDamage(currentHp: number, damage: number): number {
    const remaining = currentHp - damage;
    return remaining > 0 ? remaining : 0;
}

export function levelUp(player: Player, expGained: number): Player {
    const newExp = player.exp + expGained;
    if (newExp >= player.level * 100) {
        return {
            id: player.id,
            pos: player.pos,
            stats: {
                hp: player.stats.max_hp + 10,
                max_hp: player.stats.max_hp + 10,
                attack: player.stats.attack + 2,
                defense: player.stats.defense + 1,
            },
            exp: newExp - player.level * 100,
            level: player.level + 1,
        };
    }
    return {
        ...player,
        exp: newExp,
    };
}

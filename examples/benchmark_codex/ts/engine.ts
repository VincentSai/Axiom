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

export interface Monster {
  readonly id: number;
  readonly pos: Point;
  readonly stats: Stats;
  readonly is_alive: boolean;
}

export function tryMove(
  pos: Point,
  dx: number,
  dy: number,
  mapWidth: number,
  mapHeight: number,
): Point {
  const newX = pos.x + dx;
  const newY = pos.y + dy;

  if (newX >= 0 && newX < mapWidth && newY >= 0 && newY < mapHeight) {
    return { x: newX, y: newY };
  }

  return {
    x: pos.x >= 0 && pos.x < mapWidth ? pos.x : 0,
    y: pos.y >= 0 && pos.y < mapHeight ? pos.y : 0,
  };
}

export function calculateDamage(
  attackerAtk: number,
  defenderDef: number,
): number {
  return Math.max(1, attackerAtk - defenderDef);
}

export function applyDamage(currentHp: number, damage: number): number {
  return Math.max(0, currentHp - damage);
}

export function levelUp(player: Player, expGained: number): Player {
  const newExp = player.exp + expGained;
  const threshold = player.level * 100;

  if (newExp < threshold) {
    return {
      ...player,
      exp: newExp,
    };
  }

  const maxHp = player.stats.max_hp + 10;

  return {
    ...player,
    stats: {
      hp: maxHp,
      max_hp: maxHp,
      attack: player.stats.attack + 2,
      defense: player.stats.defense + 1,
    },
    exp: newExp - threshold,
    level: player.level + 1,
  };
}

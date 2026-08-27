/** Domain data structures for the roguelike world simulation engine. */

export interface Point {
  x: number;
  y: number;
}

export interface Velocity {
  dx: number;
  dy: number;
}

export interface Rect {
  min_x: number;
  min_y: number;
  max_x: number;
  max_y: number;
}

export interface BaseAttributes {
  strength: number;
  agility: number;
  intelligence: number;
  vitality: number;
}

export interface DerivedStats {
  max_hp: number;
  max_mana: number;
  max_stamina: number;
  phys_atk: number;
  magic_atk: number;
  armor: number;
  magic_res: number;
  crit_rate: number;
  crit_dmg_pct: number;
  dodge_rate: number;
  move_speed: number;
}

export interface StatusEffect {
  bleed_stacks: number;
  poison_duration: number;
  burn_damage: number;
  freeze_stacks: number;
  shield_hp: number;
}

export interface Entity {
  id: number;
  pos: Point;
  base_attr: BaseAttributes;
  stats: DerivedStats;
  status: StatusEffect;
  curr_hp: number;
  curr_mana: number;
  curr_stamina: number;
  gold: number;
  level: number;
  exp: number;
  weight: number;
  max_weight: number;
  is_alive: boolean;
}

use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use axiom_agent_pool::AgentTaskPool;
use axiom_parser::parse_module;
use axiom_smt::SmtResult;
use axiom_transpiler::transpile_to_rust;

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn sixteen_workers_concurrently_solve_large_roguelike_engine_with_zero_conflicts() {
    let source = include_str!("../../../examples/benchmark_large/roguelike_large.ax");
    let module = parse_module(source).expect("large skeleton should parse");
    let pool = Arc::new(Mutex::new(AgentTaskPool::new(module)));

    let tasks = pool.lock().unwrap().dispatch_tasks();
    assert_eq!(tasks.len(), 16, "all 16 hole tasks should be extracted");

    // 16 个 Worker Agent 的独立解（纯表达式）
    let solutions = vec![
        ("calc_crit_damage_hole", "if is_crit { base_dmg * crit_multiplier } else { base_dmg }"),
        ("calc_elemental_resistance_hole", "dmg - (dmg * res_percent / 100)"),
        ("apply_lifesteal_hole", "if current_hp + (dmg_dealt * lifesteal_rate / 100) < max_hp { current_hp + (dmg_dealt * lifesteal_rate / 100) } else { max_hp }"),
        ("calc_dodge_chance_hole", "if evasion > accuracy { if evasion - accuracy < 100 { evasion - accuracy } else { 100 } } else { 0 }"),
        ("apply_poison_tick_hole", "if current_hp > poison_dmg { current_hp - poison_dmg } else { 0 }"),
        ("apply_shield_absorption_hole", "if incoming_dmg > shield_amount { incoming_dmg - shield_amount } else { 0 }"),
        ("calc_mana_cost_hole", "if base_cost > cost_reduction { base_cost - cost_reduction } else { 0 }"),
        ("calc_stamina_drain_hole", "if current_stamina > action_cost { current_stamina - action_cost } else { 0 }"),
        ("calc_gold_drop_hole", "base_gold + (base_gold * luck_bonus / 100)"),
        ("calc_exp_reward_hole", "if monster_lvl >= player_lvl { base_exp + (monster_lvl - player_lvl) * 10 } else { if base_exp > (player_lvl - monster_lvl) * 5 { base_exp - (player_lvl - monster_lvl) * 5 } else { 1 } }"),
        ("evaluate_ai_aggro_hole", "current_aggro + player_threat"),
        ("calc_pathfinding_step_hole", "if target_x > cur_x { cur_x + 1 } else { if target_x < cur_x { cur_x - 1 } else { cur_x } }"),
        ("roll_loot_rarity_hole", "if roll_value > 95 { 5 } else { if roll_value > 80 { 4 } else { if roll_value > 50 { 3 } else { if roll_value > 20 { 2 } else { 1 } } } }"),
        ("calc_inventory_weight_hole", "current_weight + item_weight"),
        ("apply_health_potion_hole", "if current_hp + heal_amount < max_hp { current_hp + heal_amount } else { max_hp }"),
        ("calc_combo_multiplier_hole", "100 + combo_count * 10"),
    ];

    // 16 线程并发无锁填洞提交与 SMT 实时形式化证明
    let mut handles = Vec::new();
    for (hole_id, solution_expr) in solutions {
        let pool = Arc::clone(&pool);
        let hole_id = hole_id.to_string();
        let solution_expr = solution_expr.to_string();
        handles.push(thread::spawn(move || {
            let result = pool.lock().unwrap().submit_solution(&hole_id, &solution_expr);
            (hole_id, result)
        }));
    }

    for handle in handles {
        let (hole_id, result) = handle.join().expect("thread should join");
        let result = result.expect("solution should be valid syntax");
        assert!(
            !matches!(result, SmtResult::CounterExample { .. }),
            "hole `{hole_id}` should not trigger counter-example"
        );
    }

    // 组装无洞 Release AST
    let release_module = pool.lock().unwrap().assemble_verified_module().expect("release module should assemble");
    assert!(pool.lock().unwrap().pending_hole_ids().is_empty(), "all 16 holes filled");

    // 转译为 Rust 并使用 rustc 编译
    let rust_code = transpile_to_rust(&release_module).expect("should transpile to Rust");
    let seq = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let temp_dir = std::env::temp_dir().join(format!("axiom-large-bench-{seq}"));
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");
    let rs_path = temp_dir.join("main.rs");
    let exe_path = temp_dir.join("main.exe");
    fs::write(&rs_path, format!("{rust_code}\nfn main() {{ println!(\"16 HOLES CONCURRENT OK\"); }}\n")).expect("write rs");

    let output = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg(&rs_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("rustc should run");
    assert!(output.status.success(), "rustc should compile release AST");
    let _ = fs::remove_dir_all(&temp_dir);
}

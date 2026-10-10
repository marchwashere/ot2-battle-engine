use ot2_battle_engine::combat::{self, Attack, DamageInput, DamageKind, Element};
use std::collections::BTreeMap;

fn floor_sum(mut n: u64, mut m: u64, mut a: u64, mut b: u64) -> u64 {
    let mut result = 0;
    loop {
        if a >= m {
            result += (n - 1) * n * (a / m) / 2;
            a %= m;
        }
        if b >= m {
            result += n * (b / m);
            b %= m;
        }
        let y = a * n + b;
        if y < m {
            return result;
        }
        n = y / m;
        b = y % m;
        std::mem::swap(&mut m, &mut a);
    }
}

fn main() {
    let spell = Attack {
        kind: DamageKind::Magical,
        weapon: None,
        element: Some(Element::Ice),
        invocation: 105,
        ability_ratio: 100,
        hit_bonus: 0,
        critical_bonus: 0,
        critical_power: 125,
        normal_attack: false,
    };
    let input = DamageInput {
        attack: 108,
        defense: 85,
        growth: 80,
        guard: false,
        weapon_weak: false,
        element_weak: true,
        broken: false,
        critical: false,
        attack_up: false,
        defense_down: false,
        defense_up: false,
    };
    let mut ice = BTreeMap::<u32, u64>::new();
    let mut fire = BTreeMap::<u32, u64>::new();
    let mut ice_rolls = Vec::new();
    let mut fire_rolls = Vec::new();
    for roll in 0..32768 {
        let variance = 1.01_f32 + (1.05_f32 - 1.01_f32) * (roll as f32 * 3.051_851e-5_f32);
        let i = combat::damage(input, &spell, variance);
        let f = combat::damage(
            DamageInput {
                broken: true,
                element_weak: false,
                ..input
            },
            &Attack {
                ability_ratio: 280,
                element: Some(Element::Fire),
                ..spell.clone()
            },
            variance,
        );
        *ice.entry(i).or_default() += 1;
        *fire.entry(f).or_default() += 1;
        ice_rolls.push(i);
        fire_rolls.push(f);
    }
    let alive: u64 = ice
        .iter()
        .map(|(&a, &n)| {
            fire.iter()
                .filter(|(b, _)| a + **b < 280)
                .map(|(_, m)| n * m)
                .sum::<u64>()
        })
        .sum();
    println!(
        "Icewind {ice:?}; Fireball {fire:?}; independent-roll survival {alive}/{} = {:.8}%",
        32768_u64 * 32768,
        100.0 * alive as f64 / (32768.0 * 32768.0)
    );
    // The imported routine always separates its two variance draws by29 calls.
    // Count all 2^31 distinguishable CRT states, preserving their correlation.
    let (mut mul, mut add) = (1u32, 0u32);
    for _ in 0..29 {
        mul = mul.wrapping_mul(214013);
        add = add.wrapping_mul(214013).wrapping_add(2531011);
    }
    let modulus = 1u64 << 31;
    let a = u64::from(mul) & (modulus - 1);
    let mut exact = 0u64;
    for (h, &damage) in ice_rolls.iter().enumerate() {
        let k = (fire_rolls.partition_point(|&f| damage + f < 280) as u64) << 16;
        let b = (a * ((h as u64) << 16) + u64::from(add)) % (modulus);
        let n = 65536;
        exact += n - (floor_sum(n, modulus, a, b + modulus - k) - floor_sum(n, modulus, a, b));
    }
    println!(
        "CRT-correlated survival {exact}/{modulus} = {:.9}%",
        100.0 * exact as f64 / modulus as f64
    );
}

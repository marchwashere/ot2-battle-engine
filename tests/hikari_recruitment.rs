use ot2_battle_engine::{ActorId, CrtRng, Event, Outcome, PlayerCharacter, Resource,
    combat::{self, DamageInput}, content::Effect, field_escape::route_player,
    hikari_recruitment::{self, Prediction}, knowledge::{EnemyHistory,EnemyKnowledge}};

fn party() -> Vec<PlayerCharacter> {
    [(7,1530),(6,1467),(8,0),(4,0)].into_iter().map(|(id,exp)|route_player(id,exp).unwrap()).collect()
}
fn check(p:&Prediction) {
    assert_eq!(p.outcome,Outcome::Victory);
    assert_eq!(p.menus.len(),2);
    assert!(p.menus.iter().all(|m|m.round==1));
    assert_eq!(p.ice_soulstones_used,1);
    let player_steps:Vec<_>=p.steps.iter().filter(|s|s.actor.0<4).collect();
    assert!(matches!(&player_steps[0].events[0],Event::Action {name,..} if name=="ice_soulstone"));
    let damages:Vec<_>=player_steps[0].events.iter().filter_map(|e|match e {Event::Damage {target,amount,..}=>Some((target.0,*amount)),_=>None}).collect();
    assert_eq!(damages,[(4,260),(5,200),(6,200)]);
    assert_eq!(player_steps[1].events.iter().filter(|e|matches!(e,Event::Damage {..})).count(),1);
    assert!(player_steps[1].events.iter().any(|e|matches!(e,Event::ResourceChanged {actor:ActorId(4),resource:Resource::Hp,before:20,after:0})));
    assert!(!player_steps[1].events.iter().any(|e|matches!(e,Event::Miss {..})));
    assert_eq!(p.trace.first().unwrap().before,p.before);
    for pair in p.trace.windows(2) {assert_eq!(pair[0].after,pair[1].before,"{} -> {}",pair[0].reason,pair[1].reason);}
    assert_eq!(p.trace.last().unwrap().after,p.after);
    let initial:Vec<_>=p.trace.iter().filter(|s|s.reason=="actor_initialization").collect();
    assert_eq!(initial.len(),7);
    for (i,s) in initial.iter().enumerate(){assert_eq!(s.actor,Some(ActorId(i)));assert_eq!(s.after.draws-s.before.draws,if i<4 {1}else{3});}
    assert_eq!(p.trace.iter().filter(|s|s.reason=="enemy_drop").count(),3);
    assert_eq!(p.drops.get("ITM_CSM_0060"),Some(&1));
    assert_eq!(p.drops.get("ITM_CSM_0290"),Some(&2));
    assert_eq!((p.base_exp,p.base_jp,p.base_money),(61,60,750));
    assert_eq!(p.rewards.jp,66);
    assert_eq!(p.rewards.money,if p.party_took_damage {750}else{825});
    assert_eq!(p.enemy_knowledge.len(),2); // duplicate grunts share the same type
}

#[test]
fn carried_route_wins_across_all_four_first_and_second_actors() {
    let players=party();
    let original=serde_json::to_value(&players).unwrap();
    let mut first=[false;4];let mut second=[false;4];let mut enemy_turn=false;
    for seed in 0..1024 {
        let p=hikari_recruitment::predict(CrtRng {state:seed,draws:12345},&players,&EnemyHistory::new()).unwrap();
        check(&p);first[p.menus[0].actor]=true;second[p.menus[1].actor]=true;
        enemy_turn|=p.steps.iter().any(|s|s.actor==ActorId(4));
    }
    assert_eq!(first,[true;4]);assert_eq!(second,[true;4]);
    assert!(enemy_turn,"Corpus must cover a reachable leader action between player commands");
    assert_eq!(serde_json::to_value(&players).unwrap(),original);
}

#[test]
fn starter_agnea_and_partitio_minimum_normal_hits_guarantee_twenty_hp_finisher() {
    let p=hikari_recruitment::predict(CrtRng::new(0),&party(),&EnemyHistory::new()).unwrap();
    let enemy=p.battle_config.enemies[0].combat.as_ref().unwrap();
    for (index,id) in [(0,"attack_dagger"),(1,"attack_staff"),(2,"attack_dagger"),(3,"attack_lance")] {
        let c=p.players_before[index].combat.as_ref().unwrap();
        let skill=p.battle_config.skills.iter().find(|s|s.id==id).unwrap();
        let Effect::Damage {attack}=&skill.effects[0] else {panic!()};
        assert_eq!(attack.hit_bonus,20);
        assert_eq!(combat::hit_probability(c.accuracy,enemy.evasion,attack.hit_bonus),1.0);
        let weapon=c.weapons.iter().find(|w|Some(w.weapon)==attack.weapon).unwrap();
        let minimum=combat::damage(DamageInput {attack:c.attack+weapon.attack,defense:enemy.defense,growth:c.growth_attack,
            guard:false,weapon_weak:enemy.weapon_weaknesses.contains(&weapon.weapon),element_weak:false,
            broken:false,critical:false,attack_up:false,defense_down:false,defense_up:false},attack,1.01);
        assert!(minimum>=20,"{} minimum damage {minimum}",p.players_before[index].name);
    }
}

#[test]
fn leader_damage_charges_carried_latent_points_up_to_each_character_cap_without_extra_rng() {
    let base=party();let mut players=base.clone();
    let maxima=[80_u16,100,110,100];
    for (p,max) in players.iter_mut().zip(maxima) {p.resources.as_mut().unwrap().latent=max-1;}
    let mut charged=[false;4];
    for seed in 0..1024 {
        let p=hikari_recruitment::predict(CrtRng::new(seed),&players,&EnemyHistory::new()).unwrap();
        let mut expected=maxima.map(|m|m-1);
        for event in p.steps.iter().flat_map(|s|&s.events) {
            if let Event::Damage {target,amount,..}=*event {if target.0<4 {
                let gain=((amount as f32 / players[target.0].stats.max_hp as f32)*1.3_f32*100.0).floor() as u16;
                expected[target.0]=expected[target.0].saturating_add(gain).min(maxima[target.0]);
                charged[target.0]|=gain>0;
            }}
        }
        assert_eq!(p.party_resources_after.iter().map(|r|r.latent).collect::<Vec<_>>(),expected);
        if p.party_took_damage {
            let low=hikari_recruitment::predict(CrtRng::new(seed),&base,&EnemyHistory::new()).unwrap();
            assert_eq!(p.trace,low.trace,"latent point and gauge-full updates add no CRT calls");
        }
        if charged==[true;4] {return;}
    }
    assert_eq!(charged,[true;4]);
}

#[test]
fn lower_level_failures_are_reported_and_grown_party_keeps_the_same_routine() {
    let mut lower_failures=0;
    for levels in [[5,1,1,1],[30,30,12,12]] {
        let players=[7,6,8,4].into_iter().zip(levels).map(|(id,level)|
            route_player(id,ot2_battle_engine::field_escape::experience_for_level(level).unwrap()).unwrap()).collect::<Vec<_>>();
        for seed in 0..128 {
            match hikari_recruitment::predict(CrtRng::new(seed),&players,&EnemyHistory::new()) {
                Ok(p)=>check(&p),
                Err(e)=>{
                    assert_eq!(levels,[5,1,1,1]);
                    assert_eq!(e.0,"Recruitment routine did not win within two player commands");
                    lower_failures+=1;
                }
            }
        }
    }
    assert!(lower_failures>0,"Lv1 Osvald's neutral staff attack is not guaranteed to finish");
}

#[test]
fn leader_cannot_ko_a_full_health_route_actor_before_the_finisher() {
    let p=hikari_recruitment::predict(CrtRng::new(0),&party(),&EnemyHistory::new()).unwrap();
    let enemy=p.battle_config.enemies[0].combat.as_ref().unwrap();
    // At FirstBP=0, neither random maxBP (2..4) nor the first-round BP gain
    // enables the charged slot. NormalEnemy chooses attack or low-HP triple.
    assert_eq!(p.battle_config.enemies[0].resources.as_ref().unwrap().bp,0);
    assert_eq!(enemy.enemy_bp_base,2);
    for player in &p.players_before {
        let c=player.combat.as_ref().unwrap();
        for id in ["ABI_ENE_0010","ABI_ENE_0190"] {
            let skill=p.battle_config.skills.iter().find(|s|s.id==id).unwrap();
            let Effect::Damage {attack}=&skill.effects[0] else {panic!()};
            let maximum=combat::damage(DamageInput {attack:enemy.attack,defense:c.defense,growth:enemy.growth_attack,
                guard:false,weapon_weak:false,element_weak:false,broken:false,critical:true,attack_up:false,
                defense_down:false,defense_up:false},attack,1.05)*u32::from(skill.hits);
            assert!(maximum<player.resources.as_ref().unwrap().hp,"{} worst {id}: {maximum}",player.name);
        }
    }
}

#[test]
fn hikari_original_starter_and_growth_preserve_original_equipment() {
    let first=route_player(1,0).unwrap();
    assert_eq!((first.id.as_str(),first.stats.max_hp,first.stats.max_sp,first.stats.speed),("hikari",325,40,80));
    let combat=first.combat.as_ref().unwrap();
    assert_eq!(combat.main_weapon,Some(combat::Weapon::Sword));
    assert_eq!(combat.weapons.iter().map(|w|w.weapon).collect::<Vec<_>>(),[combat::Weapon::Sword,combat::Weapon::Lance]);
    assert_eq!((combat.defense,combat.magic_defense),(106,73));
    let grown=route_player(1,6).unwrap();
    assert_eq!(grown.stats.max_hp,339);
    assert_eq!(serde_json::to_value(&grown.combat.as_ref().unwrap().weapons).unwrap(),serde_json::to_value(&combat.weapons).unwrap());
}

#[test]
fn history_and_actual_resources_are_carried_without_mutating_inputs() {
    let mut players=party();
    for p in &mut players {let r=p.resources.as_mut().unwrap();r.hp-=10;r.sp-=7;r.latent=31;}
    let history=EnemyHistory::from([
        ("ENE_EVE_WAR_DST_090".into(),EnemyKnowledge {weapons:u8::MAX,elements:u8::MAX,studied:true}),
        ("ENE_EVE_WAR_DST_080".into(),EnemyKnowledge {weapons:u8::MAX,elements:u8::MAX,studied:true}),
    ]);
    let input=serde_json::to_value((&players,&history)).unwrap();
    let p=hikari_recruitment::predict(CrtRng::new(123),&players,&history).unwrap();
    assert!(!p.trace.iter().any(|s|s.reason=="study_foe_weakness"));
    for (before,after) in players.iter().zip(&p.party_resources_after) {
        let r=before.resources.as_ref().unwrap();assert!(after.hp<=r.hp);assert_eq!(after.sp,r.sp);assert_eq!(after.latent,r.latent);
    }
    assert_eq!(serde_json::to_value((&players,&history)).unwrap(),input);
    assert_eq!(serde_json::to_value(&p.players_before).unwrap(),serde_json::to_value(&players).unwrap());
    let fresh=hikari_recruitment::predict(CrtRng::new(123),&players,&EnemyHistory::new()).unwrap();
    assert_eq!(fresh.trace.iter().filter(|s|s.reason=="study_foe_weakness").count(),2);
}

#[test]
fn invalid_or_insufficient_party_is_rejected_without_fabricated_continuation() {
    let mut players=party();players[0].resources.as_mut().unwrap().hp=0;
    assert!(hikari_recruitment::predict(CrtRng::new(0),&players,&EnemyHistory::new()).is_err());
    let mut players=party();players.swap(0,1);
    assert!(hikari_recruitment::predict(CrtRng::new(0),&players,&EnemyHistory::new()).is_err());
    let mut players=party();
    for p in &mut players {let c=p.combat.as_mut().unwrap();c.attack=0;c.accuracy=0;for w in &mut c.weapons {w.attack=0;}}
    assert!(hikari_recruitment::predict(CrtRng::new(0),&players,&EnemyHistory::new()).is_err());
}

#[test]
fn boost_plans_replay_both_menus_and_last_menu_keeps_every_earlier_step() {
    let players=party();let original=serde_json::to_value(&players).unwrap();let history=EnemyHistory::new();
    let mut first=[false;4];let mut second=[false;4];
    for seed in 0..128 {
        let before=CrtRng {state:seed,draws:7100};
        let base=hikari_recruitment::predict(before,&players,&history).unwrap();
        let empty=hikari_recruitment::predict_with_boost_inputs(before,&players,&history,&[],&[]).unwrap();
        assert_eq!(serde_json::to_value(&base).unwrap(),serde_json::to_value(empty).unwrap());
        for (a,b) in [(vec![],vec![1,0]),(vec![1,0],vec![]),(vec![1,0,1,0],vec![1,0,1,0,1,0])] {
            let p=hikari_recruitment::predict_with_boost_inputs(before,&players,&history,&a,&b).unwrap();check(&p);
            first[p.menus[0].actor]=true;second[p.menus[1].actor]=true;
            assert_eq!(p.menus[0].boost_inputs,a);assert_eq!(p.menus[1].boost_inputs,b);
            let rises=p.trace.iter().filter(|s|s.reason=="boost_voice").collect::<Vec<_>>();
            assert_eq!(rises.len(),(a.len()+b.len())/2);
            assert!(rises.iter().all(|s|s.after.draws-s.before.draws==1));
            assert!(p.trace.iter().filter(|s|s.reason=="boost_reduce").all(|s|s.before==s.after));
            if a.is_empty() {
                assert_eq!(p.menus[1].rng,base.menus[1].rng);assert_eq!(p.menus[1].actor,base.menus[1].actor);
                let earlier=p.steps.iter().take_while(|s|s.rng_before.draws<base.menus[1].rng.draws).collect::<Vec<_>>();
                let expected=base.steps.iter().take_while(|s|s.rng_before.draws<base.menus[1].rng.draws).collect::<Vec<_>>();
                assert_eq!(serde_json::to_value(earlier).unwrap(),serde_json::to_value(expected).unwrap());
            }
        }
    }
    assert_eq!(first,[true;4]);assert_eq!(second,[true;4]);assert_eq!(serde_json::to_value(players).unwrap(),original);
}

#[test]
fn battle_init_resets_bp_and_invalid_paths_cannot_spend_unavailable_boost() {
    let mut players=party();let history=EnemyHistory::new();
    for p in &mut players {p.resources.as_mut().unwrap().bp=5;}
    let before=serde_json::to_value(&players).unwrap();
    let p=hikari_recruitment::predict_with_boost_inputs(CrtRng::new(0),&players,&history,&[],&[1,0]).unwrap();
    assert!(p.players_before.iter().all(|p|p.resources.as_ref().unwrap().bp==5));
    assert!(p.battle_config.players.iter().all(|p|p.resources.as_ref().unwrap().bp==1));
    for path in [&[1][..],&[2,1,0][..],&[1,2,1,0][..],&[1,0,0][..]] {
        assert!(hikari_recruitment::predict_with_boost_inputs(CrtRng::new(0),&players,&history,&[],path).is_err());
    }
    assert_eq!(serde_json::to_value(&players).unwrap(),before);
}

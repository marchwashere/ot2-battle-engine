use ot2_battle_engine::{combat::{Weapon,WeaponStats},field_escape::route_player,jobs,progression};

#[test]
fn scholar_revises_character_base_before_flat_gear_and_preserves_resources() {
    let mut p=route_player(7,1597).unwrap();
    assert_eq!(progression::growth(7,1597).unwrap().level,15);
    p.stats.max_sp+=9;
    let c=p.combat.as_mut().unwrap();c.magic+=17;c.magic_defense+=23;
    let before=p.clone();
    let change=jobs::change_secondary_job(&mut p,7,15,None,Some(6)).unwrap();
    assert_eq!(change.stat_delta,[("MATK".into(),10),("MDEF".into(),4),("MP".into(),7)].into());
    assert_eq!(p.stats.max_sp,before.stats.max_sp+7);
    assert_eq!(p.combat.as_ref().unwrap().magic,before.combat.as_ref().unwrap().magic+10);
    assert_eq!(p.combat.as_ref().unwrap().magic_defense,before.combat.as_ref().unwrap().magic_defense+4);
    assert_eq!(p.resources,before.resources);
    assert_eq!(p.stats.max_hp,before.stats.max_hp);assert_eq!(p.stats.speed,before.stats.speed);
    let c=p.combat.as_ref().unwrap();let old=before.combat.as_ref().unwrap();
    assert_eq!((c.attack,c.defense,c.accuracy,c.evasion,c.critical,c.growth_attack,c.growth_magic),
        (old.attack,old.defense,old.accuracy,old.evasion,old.critical,old.growth_attack,old.growth_magic));
    assert_eq!(c.main_weapon,Some(Weapon::Dagger));
    assert_eq!(change.added.len(),1);assert_eq!(change.added[0].source_id,"ITM_EQP_ROD_010");
    assert_eq!((change.added[0].attack,change.added[0].magic),(0,0));
    assert_eq!(serde_json::to_value(&c.weapons[..old.weapons.len()]).unwrap(),serde_json::to_value(&old.weapons).unwrap());
    assert!(p.skills.iter().any(|s|s=="attack_staff"));
    assert!(before.skills.iter().all(|s|p.skills.contains(s)));
}

#[test]
fn assignment_is_idempotent_and_removal_clamps_without_healing_or_losing_learned_ids() {
    let mut p=route_player(7,1597).unwrap();let base=p.clone();
    jobs::change_secondary_job(&mut p,7,15,None,Some(6)).unwrap();
    p.skills.extend(jobs::scholar_abilities().iter().map(|a|a.id.clone()));
    p.resources.as_mut().unwrap().sp=p.stats.max_sp;
    p.resources.as_mut().unwrap().hp=123;
    p.resources.as_mut().unwrap().bp=4;p.resources.as_mut().unwrap().latent=51;
    let assigned=serde_json::to_value(&p).unwrap();
    let repeat=jobs::change_secondary_job(&mut p,7,15,Some(6),Some(6)).unwrap();
    assert!(repeat.stat_delta.is_empty()&&repeat.added.is_empty()&&repeat.removed.is_empty());
    assert_eq!(serde_json::to_value(&p).unwrap(),assigned);
    let removal=jobs::change_secondary_job(&mut p,7,15,Some(6),None).unwrap();
    assert_eq!(removal.removed.len(),1);assert_eq!(removal.removed[0].weapon,Weapon::Staff);
    assert_eq!(p.stats,base.stats);
    assert_eq!(serde_json::to_value(&p.combat).unwrap(),serde_json::to_value(&base.combat).unwrap());
    assert_eq!((p.resources.as_ref().unwrap().hp,p.resources.as_ref().unwrap().sp),(123,base.stats.max_sp));
    assert_eq!((p.resources.as_ref().unwrap().bp,p.resources.as_ref().unwrap().latent),(4,51));
    assert!(!p.skills.iter().any(|s|s=="attack_staff"));
    assert!(jobs::scholar_abilities().iter().all(|a|p.skills.contains(&a.id)),"caller owns learned ability ledger");
}

#[test]
fn job_aware_growth_commutes_with_assignment_for_every_supported_level() {
    for id in [7,8,4,1] {
        for from in 1..99 {
            let mut before=route_player(id,progression::growth_at_level(id,from).unwrap().total_exp).unwrap();
            let mut after=before.clone();
            // Non-original flat equipment contributions must survive both paths.
            before.combat.as_mut().unwrap().magic+=3;after.combat.as_mut().unwrap().magic+=3;
            jobs::change_secondary_job(&mut before,id,from,None,Some(6)).unwrap();
            jobs::apply_growth(&mut before,id,from,from+1,Some(6)).unwrap();
            progression::apply_growth(&mut after,id,from,from+1).unwrap();
            jobs::change_secondary_job(&mut after,id,from+1,None,Some(6)).unwrap();
            assert_eq!(serde_json::to_value(&before).unwrap(),serde_json::to_value(&after).unwrap(),"{id} Lv{from}");
            jobs::change_secondary_job(&mut before,id,from+1,Some(6),None).unwrap();
            let mut plain=route_player(id,progression::growth_at_level(id,from+1).unwrap().total_exp).unwrap();
            plain.combat.as_mut().unwrap().magic+=3;
            assert_eq!(serde_json::to_value(&before.combat).unwrap(),serde_json::to_value(&plain.combat).unwrap());
        }
    }
}

#[test]
fn source_abilities_keep_all_four_tiers_and_do_not_claim_barrage_execution() {
    let definitions=jobs::scholar_abilities();
    assert_eq!(definitions.iter().map(|a|(a.id.as_str(),a.name.as_str(),a.sp_cost,a.default_on_assignment)).collect::<Vec<_>>(),[
        ("ABI_SET_SCH_010","Fireball",14,true),("ABI_SET_SCH_020","Icewind",14,true),
        ("ABI_SET_SCH_030","Lightning Bolt",14,false),("ABI_SET_SCH_060","Elemental Barrage",10,false)]);
    for d in definitions {assert_eq!(d.tiers.len(),4);for (id,tier) in d.tier_ids.iter().zip(&d.tiers){assert_eq!(tier["id"].as_str(),Some(id.as_str()));}}
    assert_eq!(definitions[3].tiers[0]["Ailment"][0]["AilmentName"],"RANDOM_TARGET_MAGIC_DAMAGE_FIRE");
    assert_eq!(jobs::permitted_weapons(7,Some(6)).unwrap(),[Weapon::Sword,Weapon::Dagger,Weapon::Staff]);
}

#[test]
fn already_equipped_staff_is_kept_and_invalid_changes_are_atomic() {
    let mut p=route_player(7,1597).unwrap();
    p.combat.as_mut().unwrap().weapons.push(WeaponStats {weapon:Weapon::Staff,attack:42,magic:81});
    let before=serde_json::to_value(&p).unwrap();
    assert!(jobs::change_secondary_job(&mut p,7,15,None,Some(12)).is_err());
    assert_eq!(serde_json::to_value(&p).unwrap(),before);
    let change=jobs::change_secondary_job(&mut p,7,15,None,Some(6)).unwrap();
    assert!(change.added.is_empty());
    assert!(p.skills.iter().any(|s|s=="attack_staff"));
    let staff=p.combat.as_ref().unwrap().weapons.iter().find(|w|w.weapon==Weapon::Staff).unwrap();
    assert_eq!((staff.attack,staff.magic),(42,81));
    let before=serde_json::to_value(&p).unwrap();
    assert!(jobs::apply_growth(&mut p,7,15,100,Some(6)).is_err());
    assert_eq!(serde_json::to_value(&p).unwrap(),before);
}

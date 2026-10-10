use ot2_battle_engine::{ActorId,CrtRng,PlayerCharacter,escape::Situation,
    field_escape::{FieldEscapeMenu,route_player,experience_for_level},jobs,knowledge::EnemyHistory};

fn carried_party()->Vec<PlayerCharacter> {
    let mut p=[(7,15),(1,1),(8,5),(4,5)].map(|(id,l)|route_player(id,experience_for_level(l).unwrap()).unwrap()).to_vec();
    jobs::change_secondary_job(&mut p[0],7,15,None,Some(6)).unwrap();
    p[0].skills.extend(jobs::scholar_abilities().iter().map(|a|a.id.clone()));
    for (p,max) in p.iter_mut().zip([80,125,110,100]) {let r=p.resources.as_mut().unwrap();r.hp-=5;r.sp-=4;r.latent=max;r.bp=0;}
    p
}

#[test]
fn carried_original_parties_match_every_legacy_first_menu_and_escape_draw() {
    let enemies=["ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_110","ENE_NML_FLD_SNW_150"];
    for levels in [&[5][..],&[5,1][..],&[15,15,1][..]] {
        let players=[7,6,8].into_iter().zip(levels).map(|(id,&l)|route_player(id,experience_for_level(l).unwrap()).unwrap()).collect::<Vec<_>>();
        for situation in [Situation::Normal,Situation::Preemptive] {for seed in 0..64 {
            let entry=CrtRng {state:seed,draws:6800};
            let legacy=FieldEscapeMenu::with_history(entry,situation,levels,&enemies,&EnemyHistory::new()).unwrap();
            let carried=FieldEscapeMenu::with_players(entry,situation,&players,levels,&enemies,&EnemyHistory::new()).unwrap();
            assert_eq!((carried.rng,carried.first_actor,&carried.current_order,&carried.next_order),
                (legacy.rng,legacy.first_actor,&legacy.current_order,&legacy.next_order));
            let actual=carried.attempt(&[1,0,1,0],14).unwrap();let expected=legacy.attempt(&[1,0,1,0],14).unwrap();
            assert_eq!(serde_json::to_value(actual).unwrap(),serde_json::to_value(expected).unwrap());
        }}
    }
}

#[test]
fn all_water_groups_and_harbor_octopuff_support_four_carried_actors_without_study_foe() {
    let imported:serde_json::Value=serde_json::from_str(include_str!("../data/field_carried.json")).unwrap();
    let mut groups=imported["groups"].as_object().unwrap().values().map(|g|g.as_array().unwrap().iter().map(|s|s.as_str().unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>();
    groups.push(vec!["ENE_NML_WTR_550","ENE_NML_FLD_SEA_170"]);
    assert!(groups.iter().any(|g|g.len()==5));
    let players=carried_party();let original=serde_json::to_value(&players).unwrap();
    let mut actors=[false;4];let mut failed=0;
    for enemies in groups {for situation in [Situation::Normal,Situation::Preemptive] {for seed in 0..64 {
        let entry=CrtRng {state:seed,draws:7200};
        let menu=FieldEscapeMenu::with_players(entry,situation,&players,&[15,1,5,5],&enemies,&EnemyHistory::new()).unwrap();
        actors[menu.first_actor]=true;
        assert_eq!(menu.current_order[0],menu.first_actor);
        assert_eq!(menu.current_order.len(),if situation==Situation::Preemptive {4}else{4+enemies.len()});
        assert_eq!(menu.next_order.len(),4+enemies.len());
        assert!(menu.enemy_knowledge.is_empty());
        assert!(!menu.trace.iter().any(|s|s.reason=="study_foe_weakness"));
        let init=menu.trace.iter().filter(|s|s.reason=="actor_initialization").collect::<Vec<_>>();
        assert_eq!(init.len(),enemies.len());
        for (slot,span) in init.iter().enumerate() {assert_eq!(span.actor,Some(ActorId(4+slot)));assert_eq!(span.after.draws-span.before.draws,3);}
        let mut cursor=menu.clone();let mut path=Vec::new();
        for _ in 0..4 {
            let attempt=cursor.clone().attempt(&[],8).unwrap();failed+=usize::from(!attempt.success);
            let replay=menu.clone().attempt(&path,8).unwrap();
            assert_eq!(serde_json::to_value(&attempt).unwrap(),serde_json::to_value(replay).unwrap());
            assert!(attempt.trace.windows(2).all(|p|p[0].after==p[1].before));
            assert_eq!(attempt.trace.first().unwrap().before,entry);
            if situation==Situation::Preemptive {assert!(attempt.success);assert_eq!(attempt.before_check,attempt.after);}
            let before=cursor.rng;cursor.boost_cancel_cycle();path.extend([1,0]);
            assert_eq!(cursor.rng.draws-before.draws,1,"Every actor's ordinary Lv1 boost voice consumes one draw");
        }
    }}}
    assert_eq!(actors,[true;4]);assert!(failed>0);
    assert_eq!(serde_json::to_value(players).unwrap(),original);
}

#[test]
fn identity_drives_study_foe_and_caps_while_carried_speed_and_pinch_drive_menu() {
    let mut party=carried_party();let enemy=["ENE_NML_FLD_SEA_170"];
    party[1]=route_player(6,0).unwrap();
    let with=FieldEscapeMenu::with_players(CrtRng::new(0),Situation::Normal,&party,&[15,1,5,5],&enemy,&EnemyHistory::new()).unwrap();
    assert_eq!(with.trace.iter().filter(|s|s.reason=="study_foe_weakness").count(),1);
    let mut changed_order=false;let mut changed_pinch=false;
    for seed in 0..64 {
        let p=carried_party();let entry=CrtRng::new(seed);
        let base=FieldEscapeMenu::with_players(entry,Situation::Normal,&p,&[15,1,5,5],&enemy,&EnemyHistory::new()).unwrap();
        let mut slow=p.clone();slow[0].stats.speed=1;slow[1].stats.speed=999;
        let altered=FieldEscapeMenu::with_players(entry,Situation::Normal,&slow,&[15,1,5,5],&enemy,&EnemyHistory::new()).unwrap();
        changed_order|=base.current_order!=altered.current_order;
        let mut pinch=p.clone();pinch[base.first_actor].resources.as_mut().unwrap().hp=1;
        let altered=FieldEscapeMenu::with_players(entry,Situation::Normal,&pinch,&[15,1,5,5],&enemy,&EnemyHistory::new()).unwrap();
        changed_pinch|=base.rng!=altered.rng;
        assert_eq!(base.current_order,altered.current_order);
    }
    assert!(changed_order&&changed_pinch);
    let mut p=carried_party();p[1].resources.as_mut().unwrap().latent=126;
    assert!(FieldEscapeMenu::with_players(CrtRng::new(0),Situation::Normal,&p,&[15,1,5,5],&enemy,&EnemyHistory::new()).is_err());
    assert!(FieldEscapeMenu::with_players(CrtRng::new(0),Situation::Surprise,&carried_party(),&[15,1,5,5],&enemy,&EnemyHistory::new()).is_err());
    let mut p=carried_party();p[1]=p[0].clone();
    assert!(FieldEscapeMenu::with_players(CrtRng::new(0),Situation::Normal,&p,&[15,1,5,5],&enemy,&EnemyHistory::new()).is_err());
}

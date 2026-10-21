//! BG primitive execution. The parent scheduler owns target selection and time.
use super::*;
use crate::bg_effects::{EffectKind,Operation};

// Audited route profile: 120 FPS, 2x battle speed, 24000 action ticks/second.
// TickManualSequencerUpdate evaluates keys on a frame; SetPlaybackPosition
// defers marker jumps until evaluation finishes. Barrage must rebase the next
// branch at that evaluated time, not the earlier authored key time. Otherwise
// successive jumps underestimate elapsed time and suppress late weakness voices.
// Keep this scoped to Barrage; other action clocks retain their audited behavior.
const BARRAGE_FRAME_TICKS:f64=24_000.0*2.0/120.0;

impl Battle {
    pub(super) fn grace_enemies_alive(&self)->bool {(0..self.state.actors.len()).any(|i|self.side(ActorId(i))==Some(Side::Enemy)&&self.state.actors[i].alive())}
    pub(super) fn grace_step(&mut self,command:Command,levels:Option<&[u8]>)->Result<StepReport> {
        ensure(self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead),"Summon/dance requires the carried Gravell battle profile")?;
        let actor=self.current_actor().ok_or_else(||SimError("Battle ended".into()))?;
        let (id,bp)=match command {Command::Skill {id,..}=>(id,0),Command::Boost {id,bp,..}=>(id,bp),_=>unreachable!()};
        let index=self.catalog().skill_id(&id)?;
        ensure(self.scenario.actors[actor.0].skills.contains(&index),"Skill not learned")?;
        ensure(!self.state.actors[actor.0].has_status(StatusKind::Silence),"Silence prevents this skill")?;
        let cost=self.catalog().skills[index].sp_cost;
        ensure(bp<=3&&self.state.actors[actor.0].resources.bp>=bp,"Not enough BP")?;
        ensure(self.state.actors[actor.0].resources.sp>=cost,"Not enough SP")?;
        let before=self.state.rng;
        let standard:Vec<_>=(1..=bp).collect();let inputs=levels.unwrap_or(&standard);
        if !inputs.is_empty() {self.validate_boost_inputs(inputs,bp)?;self.apply_boost_inputs(inputs);}
        let mut events=vec![Event::Action {actor,name:id.clone()}];
        let state=&mut self.state.actors[actor.0];state.guard=false;
        let old=state.resources.bp;state.resources.bp-=bp;state.boosted_this_round|=bp>0;
        if bp>0 {events.push(Event::ResourceChanged {actor,resource:Resource::Bp,before:old.into(),after:state.resources.bp.into()});}
        let old=state.resources.sp;state.resources.sp-=cost;
        if cost>0 {events.push(Event::ResourceChanged {actor,resource:Resource::Sp,before:old,after:state.resources.sp});}
        self.state.opening_crt.as_mut().unwrap().settled_damage.fill(false);
        if id=="bewildering_grace" {
            let mut dance=crate::bewildering_grace::Dance::new(bp)?;
            let mut first=true;
            // BG children are SetSummonAbility(..., owner=parent). The native
            // recovery flag's getter/setter delegate to that owner, so it spans
            // the whole dance, including nested summon commands.
            let mut recovery_settled=false;
            while let Some(selection)=dance.select(&mut self.state.rng)? {
                self.trace_rng("bewildering_grace_lottery",Some(actor),selection.before);
                // SetupActionCommand selects the first child before the parent
                // ability voice. Later children follow completion of all child callbacks.
                if first {self.voice(actor,18,VoiceCondition {ability:Some("ABI_DAN_070_01"),..Default::default()});first=false;}
                self.grace_child_with_recovery(actor,crate::bg_effects::child(selection.ability)?,&mut events,&mut recovery_settled)?;
                dance.complete_child(selection.ability,self.state.actors[actor.0].alive(),self.grace_enemies_alive())?;
            }
        } else if matches!(id.as_str(),"critical_scope"|"aggressive_slash"|"elemental_barrage"|"holy_light") {
            ensure(self.state.opening_crt.as_ref().is_some_and(|c|c.lord_sands),"New actions require Lord Sands profile")?;
            ensure((matches!(id.as_str(),"critical_scope"|"holy_light")&&bp==0)||(id=="aggressive_slash"&&(bp==2||bp==3))||(id=="elemental_barrage"&&bp==3),"Unsupported route skill boost")?;
            if id=="aggressive_slash" {self.state.actors[actor.0].main_weapon=Some(combat::Weapon::Sword);}
            if matches!(id.as_str(),"elemental_barrage"|"holy_light") {self.state.actors[actor.0].main_weapon=self.scenario.actors[actor.0].combat.as_ref().unwrap().weapons.iter().max_by_key(|w|w.magic).map(|w|w.weapon);}
            let child=crate::bg_effects::child(match id.as_str() {"critical_scope"=>"ABI_INV_030_01","holy_light"=>"ABI_CLE_020_01","elemental_barrage"=>"ABI_SCH_060_04",_=>if bp==3 {"ABI_WAR_010_04"}else{"ABI_WAR_010_03"}})?;
            self.grace_common_child(actor,child,&[ActorId(4)],&mut events)?;
        } else {
            let beast=id=="hired_help_beastlings";
            let summons=if beast {crate::hired_help::prepare_beastlings(&mut self.state.rng,bp)?}else{crate::hired_help::prepare_guards(&mut self.state.rng,bp)?};
            self.trace_rng(if beast {"beastlings_variant_preparation"}else{"guards_variant_preparation"},Some(actor),summons.before);
            self.voice(actor,18,VoiceCondition {ability:Some(if beast {"ABI_MER_070_E_1"}else{"ABI_MER_070_D_1"}),..Default::default()});
            let targets:Vec<_>=(0..self.state.actors.len()).map(ActorId).filter(|id|self.side(*id)==Some(Side::Enemy)&&self.state.actors[id.0].alive()).collect();
            let mut jobs=vec![];
            for (i,id) in summons.variants.iter().take(summons.count as usize).enumerate() {
                let profile=crate::bg_effects::summon(id)?;
                ensure(profile.initial_idle_frames==0,"Guard sprite initialization changed")?;
                jobs.push(([53280.0,64778.0,76778.0,88778.0][i],crate::bg_effects::child(if beast {"ABI_MER_YOHEI_05_01"}else{"ABI_MER_YOHEI_04_01"})?,targets.clone(),Some(profile)));
            }
            self.grace_commands(actor,&jobs,&mut events)?;
        }
        Ok(self.complete_player_action(actor,events,before))
    }
    #[cfg(test)]
    fn grace_child(&mut self,actor:ActorId,child:&'static crate::bg_effects::Child,events:&mut Vec<Event>)->Result<()> {
        self.grace_child_with_recovery(actor,child,events,&mut false)
    }
    fn grace_child_with_recovery(&mut self,actor:ActorId,child:&'static crate::bg_effects::Child,events:&mut Vec<Event>,recovery_settled:&mut bool)->Result<()> {
        // Transform selects its replacement in SetupActionCommand; target
        // selection uses the complete enemy array in TransformEnemyStart.
        if child.id=="ABI_DAN_MAKA_003" {return self.grace_transform(child,events);}
        let actors=(0..self.state.actors.len()).map(|i|crate::bg_effects::TargetActor {id:ActorId(i),side:self.scenario.actors[i].side,alive:self.state.actors[i].alive()}).collect::<Vec<_>>();
        let before=self.state.rng;let targets=child.select_targets(actor,&actors,&mut self.state.rng)?;
        self.trace_rng("bg_child_target",Some(actor),before);
        events.push(Event::Action {actor,name:child.id.clone()});
        if matches!(child.id.as_str(),"ABI_DAN_MAKA_006"|"ABI_DAN_MAKA_017") {
            let summon=crate::bg_effects::summon(&child.operations[0].source)?;
            if summon.initial_idle_frames>0 {let before=self.state.rng;self.state.rng.index(summon.initial_idle_frames);self.trace_rng("bg_summon_initial_frame",Some(actor),before);}
            let nested=crate::bg_effects::child(if child.id=="ABI_DAN_MAKA_006" {"ABI_DAN_MAKA_006_INV"}else{"ABI_DAN_MAKA_017_INV"})?;
            self.grace_commands_with_recovery(actor,&[(0.0,nested,targets,Some(summon))],events,recovery_settled)
        } else {self.grace_commands_with_recovery(actor,&[(0.0,child,targets,None)],events,recovery_settled)}
    }
    fn grace_transform(&mut self,child:&crate::bg_effects::Child,events:&mut Vec<Event>)->Result<()> {
        let before=self.state.rng;let roll=self.state.rng.index(100);let mut sum=0;
        let replacement=child.operations.iter().find(|op|{sum+=op.value;roll<sum}).ok_or_else(||SimError("Invalid transformation weights".into()))?;
        self.trace_rng("bg_transform_species",None,before);
        let mut rows:Vec<_>=(0..self.state.actors.len()).map(ActorId).filter(|id|self.side(*id)==Some(Side::Enemy)).collect();
        let before=self.state.rng;for i in 0..rows.len() {let j=i+self.state.rng.index((rows.len()-i) as u32) as usize;rows.swap(i,j);}
        self.trace_rng("bg_transform_target_shuffle",None,before);
        let Some(target)=rows.into_iter().find(|id|self.state.actors[id.0].alive()&&!crate::bg_effects::transform_immune(&self.scenario.actors[id.0].id)) else {return Ok(())};
        let enemy=crate::field_escape::transformed_enemy(&replacement.source)?;
        let combat=enemy.combat.clone().ok_or_else(||SimError("Missing transform combat profile".into()))?;
        let known=self.enemy_knowledge().get(&replacement.source).copied().unwrap_or_default();
        let old_broken=self.state.actors[target.0].broken();
        let before=self.state.rng;
        if let Some(init)=&combat.enemy_initialization {if init.initial_idle_frames>0 {self.state.rng.index(init.initial_idle_frames);}}
        self.trace_rng("bg_transform_resource_frame",Some(target),before);
        let before=self.state.rng;let charge=combat.enemy_bp_base+self.state.rng.index(3) as u8;self.trace_rng("bg_transform_bp",Some(target),before);
        let def=&mut Arc::make_mut(&mut self.scenario).actors[target.0];
        def.id=enemy.id;def.name=enemy.name;def.stats=enemy.stats;def.combat=Some(combat.clone());
        let state=&mut self.state.actors[target.0];state.resources=enemy.resources.unwrap_or_else(||Resources::full(&def.stats));
        state.shields=combat.shields;state.break_until_round=0;state.statuses.clear();state.enemy_charge_max=charge;
        state.revealed_weapon_weaknesses=known.weapons;state.revealed_element_weaknesses=known.elements;
        if old_broken {let mut i=0;self.state.queue.retain(|id|{let keep=i<=self.state.cursor||*id!=target;i+=1;keep});}
        self.grace_insert_next(target,"bg_transform_next_order_insert");
        events.push(Event::Action {actor:target,name:format!("transformed_to_{}",replacement.source)});
        Ok(())
    }
    fn grace_insert_next(&mut self,target:ActorId,reason:&'static str) {
        self.state.next_queue.retain(|id|*id!=target);
        let mut gaps:Vec<_>=(0..=self.state.next_queue.len()).collect();
        for (i,&id) in self.state.next_queue.iter().enumerate() {let s=&self.state.actors[id.0];if s.broken() {if s.break_until_round<=self.state.round+1 {gaps.retain(|g|*g!=i);}else {gaps.retain(|g|*g!=i+1);}}}
        let before=self.state.rng;let index=gaps[self.state.rng.index(gaps.len() as u32) as usize];
        self.trace_rng(reason,Some(target),before);self.state.next_queue.insert(index,target);
    }
    pub(super) fn grace_post_action(&mut self,actor:ActorId,events:&mut Vec<Event>) {
        if !self.state.actors[actor.0].alive()||!self.state.actors[actor.0].has_status(StatusKind::Poison) {return;}
        // ExecDisease_Poison -> SetDamageForPlayer(IsPoisonDamage=true):
        // no damage reaction/latent gain, but the damage UI timer still runs.
        let damage=(self.scenario.actors[actor.0].stats.max_hp as f32*0.167_f32).ceil() as u32;
        let state=&mut self.state.actors[actor.0];let before=state.resources.hp;state.resources.hp=before.saturating_sub(damage);
        events.push(Event::ResourceChanged {actor,resource:Resource::Hp,before,after:state.resources.hp});
        if !state.alive() {self.grace_player_death(actor);}
        self.timer_rng("poison_damage_notification_timer",Some(actor));
    }
    pub(super) fn grace_last_act(&mut self) {
        if !self.state.opening_crt.as_ref().is_some_and(|c|c.step_ahead) {return;}
        // CheckCondition_LastAct excludes the opening retail turn of a preemptive battle.
        if self.state.round<=2 && self.state.opening_crt.as_ref().is_some_and(|c|c.mansion_situation==Some(crate::escape::Situation::Preemptive)) {return;}
        let modifiers=self.state.grace.get_or_insert_with(Default::default);
        if modifiers.last_act_round==Some(self.state.round) {return;}
        modifiers.last_act_round=Some(self.state.round);
        for (i,state) in self.state.actors.iter_mut().enumerate() {
            if state.alive()&&!state.broken()&&state.has_status(StatusKind::LastAct) {
                state.statuses.retain(|s|s.kind!=StatusKind::LastAct);
                self.state.queue.push(ActorId(i));
            }
        }
    }
    /// Executes the authored timeline for an ordinary BG child. The caller
    /// selects its targets during child preparation, before any authored event.
    /// Specialized summon/transform/random-count commands have their own setup.
    pub(super) fn grace_common_child(&mut self,actor:ActorId,child:&'static crate::bg_effects::Child,targets:&[ActorId],events:&mut Vec<Event>)->Result<()> {
        self.grace_common_command(actor,child,targets,None,events)
    }
    fn grace_common_command(&mut self,actor:ActorId,child:&'static crate::bg_effects::Child,targets:&[ActorId],summon:Option<&crate::bg_effects::SummonProfile>,events:&mut Vec<Event>)->Result<()> {
        self.grace_commands(actor,&[(0.0,child,targets.to_vec(),summon)],events)
    }
    fn grace_commands(&mut self,actor:ActorId,jobs:&[(f64,&'static crate::bg_effects::Child,Vec<ActorId>,Option<&crate::bg_effects::SummonProfile>)],events:&mut Vec<Event>)->Result<()> {
        self.grace_commands_with_recovery(actor,jobs,events,&mut false)
    }
    fn grace_commands_with_recovery(&mut self,actor:ActorId,jobs:&[(f64,&'static crate::bg_effects::Child,Vec<ActorId>,Option<&crate::bg_effects::SummonProfile>)],events:&mut Vec<Event>,recovery_settled:&mut bool)->Result<()> {
        use crate::bg_effects::{EffectKind::*,Step};
        #[derive(Default)] struct Runtime {pending:Option<(f64,ActorId)>,hits:Vec<ActorId>,random:Option<ActorId>,counter:u32,count:u32,voice_busy_until:f64}
        let mut states:Vec<Runtime>=(0..jobs.len()).map(|_|Runtime::default()).collect();
        // All overlapping summons consult the same BattleCamera.Playing flag.
        let mut break_camera_busy_until=f64::NEG_INFINITY;
        let party:Vec<_>=(0..self.state.actors.len()).map(ActorId).filter(|id|self.side(*id)==Some(Side::Player)).collect();
        let mut schedule=vec![];
        for (i,(start,child,_,_)) in jobs.iter().enumerate() {
            schedule.push((*start,i,None));
            for step in &child.steps {
                let tick=step.tick();
                schedule.push((start+tick,i,Some(step)));
            }
        }
        schedule.sort_by(|a,b|a.0.total_cmp(&b.0));
        while !schedule.is_empty() {
            let (tick,i,step)=schedule.remove(0);
            let tick=if jobs[i].1.id=="ABI_SCH_060_04" {
                (tick/BARRAGE_FRAME_TICKS).ceil()*BARRAGE_FRAME_TICKS
            }else{tick};
            loop {
                let next=states.iter().enumerate().filter_map(|(n,s)|s.pending.map(|(t,id)|(n,t,id)))
                    .filter(|(_,t,_)|*t<=tick).min_by(|a,b|a.1.total_cmp(&b.1));
                let Some((n,_,id))=next else {break};states[n].pending=None;self.timer_rng("break_slow_timer",Some(id));
            }
            let (_,child,targets,summon)=&jobs[i];let run=&mut states[i];
            let random_count=matches!(child.id.as_str(),"ABI_DAN_MAKA_039"|"ABI_WAR_010_03"|"ABI_WAR_010_04"|"ABI_SCH_060_04");
            if step.is_none() {
                if random_count {let before=self.state.rng;let extra=self.state.rng.index(if child.id=="ABI_SCH_060_04" {3}else{1});run.count=extra+match child.id.as_str(){"ABI_WAR_010_03"=>5,"ABI_WAR_010_04"|"ABI_SCH_060_04"=>6,_=>10};for n in (1..=run.count).rev() {self.state.rng.index(n);}self.trace_rng("bg_random_count_setup",Some(actor),before);}
                continue;
            }
            match *step.unwrap() {
                Step::CounterBranch{destination,..}|Step::Jump{destination,..}|Step::PatternBranch{ice:destination,..}=>{
                    let jump=match *step.unwrap(){
                        Step::CounterBranch{..}=>if run.counter>=run.count||!self.grace_enemies_alive(){Some(destination)}else{None},
                        Step::PatternBranch{ice,thunder,..}=>{let before=self.state.rng;let pattern=self.state.rng.index(3);self.trace_rng("barrage_element_pattern",Some(actor),before);match pattern{0=>None,1=>Some(ice),_=>Some(thunder)}},
                        _=>Some(destination),
                    };
                    if let Some(destination)=jump {
                        ensure(destination>step.unwrap().tick(),"Authored route sequence jump must advance")?;
                        schedule.retain(|(_,job,_)|*job!=i);
                        for next in &child.steps {if next.tick()>=destination {schedule.push((tick+next.tick()-destination,i,Some(next)));}}
                        schedule.sort_by(|a,b|a.0.total_cmp(&b.0));
                    }
                },
                Step::RandomTarget {..}=>{
                    run.counter+=1;
                    let live=targets.iter().copied().filter(|t|self.state.actors[t.0].alive()).collect::<Vec<_>>();
                    let before=self.state.rng;
                    run.random=if live.is_empty() {None} else {Some(live[self.state.rng.index(live.len() as u32) as usize])};
                    self.trace_rng("bg_random_hit_target",Some(actor),before);
                },
                Step::Voice {voice_type,..}=>{
                    if child.id=="ABI_SCH_060_04"&&self.scenario.actors[actor.0].id=="osvald" {run.voice_busy_until=tick+f64::from(crate::opening_crt::lord_sands_data().skill_voice_duration_ticks["elemental_barrage"]);}
                    if let Some(s)=summon {if voice_type==18&&s.ability_voice {let before=self.state.rng;self.state.rng.weighted_bool(1.0);self.trace_rng("summon_ability_voice",Some(actor),before);}}
                    else if child.id.starts_with("ABI_SCH_011_")||child.id.starts_with("ABI_SCH_070_") {self.scholar_cast_voice(actor,child);}
                    else {if child.id=="ABI_CLE_020_01"&&self.scenario.actors[actor.0].id=="temenos" {run.voice_busy_until=tick+f64::from(crate::opening_crt::lord_sands_data().skill_voice_duration_ticks["holy_light"]);}self.voice(actor,voice_type,VoiceCondition {ability:Some(match child.id.as_str(){"ABI_WAR_010_03"|"ABI_WAR_010_04"=>"ABI_WAR_010_01","ABI_SCH_060_04"=>"ABI_SCH_060_01",_=>&child.id}),..Default::default()});}
                },
                Step::Effects {index,last,on_hit,..}=>{
                    // CheckLastHit uses the parent summon flag for Hired Help,
                    // not the single-hit child's flag. Early lethal summons
                    // retain Break until the last summoned attack settles death.
                    let last=if summon.is_some_and(|s|s.id.starts_with("PC_ABI_MER_YOHEI")) {i+1==jobs.len()}else{last};
                    if random_count&&!self.grace_enemies_alive() {continue;}
                    let damage_event_start=events.len();
                    let before_states=self.state.actors.iter().map(|a|(a.alive(),a.broken())).collect::<Vec<_>>();
                    let mut reaction=None;
                    for (n,op) in child.operations.iter().enumerate() {
                        if index.is_some_and(|v|v!=n) {continue;}
                        let before=self.state.rng;
                        let mut selected=if random_count {run.random.into_iter().collect()} else {op.select_targets(actor,targets,&party,&mut self.state.rng)?};
                        if on_hit {selected.retain(|t|run.hits.contains(t));}
                        self.trace_rng("bg_ailment_target",Some(actor),before);
                        let result=match op.kind {
                            PhysicalDamage|MagicDamage|RandomTargetMagicDamage|RandomTargetPhysicalDamage=>{
                                let begin=events.len();let profile=summon.map(|s|s.combat());
                                let result=self.grace_damage(actor,&selected,child,op,last,profile.as_ref(),events)?;
                                for e in &events[begin..] {if let Event::Damage {target,..}=e {if !run.hits.contains(target) {run.hits.push(*target);}
                                    if summon.is_some_and(|s|s.id.starts_with("PC_ABI_MER_YOHEI")) {self.state.grace.get_or_insert_with(Default::default).guard_hits.push(*target);}
                                }}
                                result
                            },
                            AdditiveShieldDamage=>{ensure(selected.iter().all(|t|!self.state.actors[t.0].alive()||self.state.actors[t.0].broken()),"Beastlings additive shields currently require broken target")?;None},
                            ShieldDamage=>self.grace_shield_damage(actor,&selected,op.value,events),
                            OrderDelay=>{self.grace_delay(actor,&selected,op.value as usize);None},
                            _=>{self.grace_scalar(actor,&selected,op,recovery_settled,events)?;None},
                        };
                        if result.is_some_and(|v|reaction.is_none_or(|r|v>r)) {reaction=result;}
                    }
                    if summon.is_none()&&tick>=run.voice_busy_until&&self.state.actors[actor.0].alive() {if let Some(kind)=reaction {self.voice(actor,kind,VoiceCondition::default());}}
                    let damage_targets:Vec<_>=events[damage_event_start..].iter().filter_map(|e|if let Event::Damage {target,..}=e {Some(*target)}else{None}).collect();
                    let mut new_break=None;let mut broken_hit=None;
                    for (n,(was_alive,was_broken)) in before_states.into_iter().enumerate() {
                        let target=ActorId(n);if self.side(target)!=Some(Side::Enemy)||(!was_alive&&!was_broken) {continue;}
                        let state=&mut self.state.actors[n];if last&&!state.alive() {state.break_until_round=0;}
                        if state.broken() {if !was_broken {new_break=Some(target);}else if damage_targets.contains(&target) {broken_hit=Some(target);}}
                    }
                    if let Some(target)=new_break {
                        run.pending=Some((tick+f64::from(opening_crt::data().break_timer_delay_ticks),target));
                        if tick>break_camera_busy_until {break_camera_busy_until=tick+f64::from(opening_crt::data().break_busy_ticks);}
                    }
                    // The action is updated before its appended break-camera
                    // sequence. At the same endpoint tick, OnStop has not yet
                    // cleared Playing (retail Guards trace exercises equality).
                    else if tick>break_camera_busy_until && !child.id.starts_with("ABI_SCH_011_") {if let Some(target)=broken_hit {self.timer_rng("broken_hit_slow_timer",Some(target));}}
                }
            }
        }
        let mut remaining:Vec<_>=states.into_iter().filter_map(|s|s.pending).collect();remaining.sort_by(|a,b|a.0.total_cmp(&b.0));
        for (_,target) in remaining {self.timer_rng("break_slow_timer",Some(target));}
        Ok(())
    }
    pub(super) fn grace_player_death(&mut self,target:ActorId) {
        if self.side(target)!=Some(Side::Player) {return;}
        let state=&mut self.state.actors[target.0];
        let permanent=state.statuses.iter().copied().filter(|s|s.turns==255).collect();
        state.statuses.clear();state.resources.bp=0;state.resources.latent=0;
        state.guard=false;state.cover_by=None;state.cover_hits=0;
        self.state.grace.get_or_insert_with(Default::default).dead_permanent.insert(target,permanent);
        // Keep the executing row/cursor stable. All future rows are removed,
        // including extra actions, so revival cannot restore a current action.
        let mut index=0;
        self.state.queue.retain(|id| {let keep=index<=self.state.cursor||*id!=target;index+=1;keep});
        self.state.next_queue.retain(|id|*id!=target);
    }
    pub(super) fn grace_revive(&mut self,actor:ActorId,target:ActorId,ratio:u32,speaker:Option<ActorId>,events:&mut Vec<Event>) {
        let hp=(self.scenario.actors[target.0].stats.max_hp as f32*(ratio as f32*0.01_f32)) as u32;
        if self.state.actors[target.0].alive() {
            self.grace_recover(actor,target,Resource::Hp,hp,speaker,events);return;
        }
        self.state.actors[target.0].resources.hp=hp.min(self.scenario.actors[target.0].stats.max_hp);
        events.push(Event::ResourceChanged {actor:target,resource:Resource::Hp,before:0,after:self.state.actors[target.0].resources.hp});
        self.timer_rng("revive_recovery_notification_timer",Some(target));
        if let Some(permanent)=self.state.grace.as_mut().and_then(|g|g.dead_permanent.remove(&target)) {
            self.state.actors[target.0].statuses=permanent;
        }
        self.voice(target,14,VoiceCondition {recovery_enforcer:self.grace_enforcer(actor),..Default::default()});
        self.state.next_queue.retain(|id|*id!=target);
        // BattleActionOrderBP.InsertActionOrder(normal): choose one of all
        // insertion gaps after CheckOverTake/CheckWrapAround constraints.
        let mut gaps:Vec<_>=(0..=self.state.next_queue.len()).collect();
        for (i,&id) in self.state.next_queue.iter().enumerate() {
            let state=&self.state.actors[id.0];
            if state.broken() {
                if state.break_until_round<=self.state.round+1 {gaps.retain(|g|*g!=i);}
                else {gaps.retain(|g|*g!=i+1);}
            }
        }
        let before=self.state.rng;
        let index=gaps[self.state.rng.index(gaps.len() as u32) as usize];
        self.trace_rng("revive_next_order_insert",Some(target),before);
        self.state.next_queue.insert(index,target);
    }
    fn grace_enforcer(&self,actor:ActorId)->Option<&'static str> {
        match self.scenario.actors[actor.0].id.as_str() {
            "throne"=>Some("eTHIEF"),"hikari"=>Some("eFENCER"),"agnea"=>Some("eDANCER"),
            "partitio"=>Some("eMERCHANT"),"osvald"=>Some("ePROFESSOR"),"temenos"=>Some("ePRIEST"),_=>None,
        }
    }
    pub(super) fn grace_damage(&mut self,actor:ActorId,targets:&[ActorId],child:&crate::bg_effects::Child,op:&Operation,last:bool,summon:Option<&CombatProfile>,events:&mut Vec<Event>)->Result<Option<u8>> {
        let element=match op.source.as_str(){"RANDOM_TARGET_MAGIC_DAMAGE_FIRE"=>Some(combat::Element::Fire),"RANDOM_TARGET_MAGIC_DAMAGE_ICE"=>Some(combat::Element::Ice),"RANDOM_TARGET_MAGIC_DAMAGE_THUNDER"=>Some(combat::Element::Lightning),_=>match child.attribute.rsplit("::").next().unwrap() {
            "eNONE"=>None,"eFIRE"=>Some(combat::Element::Fire),"eICE"=>Some(combat::Element::Ice),
            "eTHUNDER"=>Some(combat::Element::Lightning),"eWIND"=>Some(combat::Element::Wind),
            "eLIGHT"=>Some(combat::Element::Light),"eDARK"=>Some(combat::Element::Dark),
            other=>return Err(SimError(format!("Unknown BG element {other}"))),
        }};
        let attack=combat::Attack {kind:if matches!(op.kind,EffectKind::PhysicalDamage|EffectKind::RandomTargetPhysicalDamage) {DamageKind::Physical} else {DamageKind::Magical},
            weapon:if child.id.starts_with("ABI_WAR_010_") {Some(combat::Weapon::Sword)}else{summon.and_then(|s|s.main_weapon)},element,invocation:op.value,ability_ratio:child.ability_ratio,
            hit_bonus:child.hit_ratio,critical_bonus:child.critical_ratio,critical_power:child.critical_power,normal_attack:false};
        let mut reaction=None;
        for &target in targets {
            // Commands retain their prepared targets. LibAilment's
            // damage loop checks object validity, not current HP; later Guards
            // and BG's three light hits still process an earlier-killed target.
            // The final hit can consequently request the aggregate kill voice.
            let broken=self.state.actors[target.0].broken();
            let known=self.state.actors[target.0].revealed_element_weaknesses;
            // RandomCountAttackCommand.ExecStepCounter caches CheckHit before
            // JudgeSpecialHitRate returns it, even with Critical Scope active.
            // Aggressive Slash has MinimumCount=0 (no forced-hit slots).
            let hit=if child.id.starts_with("ABI_WAR_010_") {Some(self.normal_accuracy(actor,target,&attack))}else{None};
            self.apply_attack_profile(actor,target,&attack,hit,summon,events);
            if self.side(target)==Some(Side::Enemy) {
                let after=&self.state.actors[target.0];
                let kind=if last&&!after.alive() {Some(12)} else if !broken&&after.broken() {Some(10)}
                    else if known!=after.revealed_element_weaknesses {Some(9)} else {None};
                if kind.is_some_and(|k|reaction.is_none_or(|old|k>old)) {reaction=kind;}
            }
        }
        Ok(reaction)
    }
    pub(super) fn grace_delay(&mut self,actor:ActorId,targets:&[ActorId],count:usize) {
        self.shift_action_orders(actor, targets, count.min(i32::MAX as usize) as i32);
    }
    pub(super) fn grace_shield_damage(&mut self,actor:ActorId,targets:&[ActorId],amount:u32,events:&mut Vec<Event>)->Option<u8> {
        for &target in targets {
            if !self.state.actors[target.0].alive()||self.side(target)!=Some(Side::Enemy) {continue;}
            let s=&mut self.state.actors[target.0];
            let before=s.shields;s.shields=s.shields.saturating_sub(amount.min(255) as u8);
            events.push(Event::ShieldChanged {actor:target,before,after:s.shields});
            if before>0&&s.shields==0 {
                s.break_until_round=self.state.round+2;
                events.push(Event::Broken {actor:target,recover_round:s.break_until_round});
                self.break_action_order(target);
                self.timer_rng("bg_shield_break_notification",Some(target));
                let cap=crate::field_escape::player_latent_max(&self.scenario.actors[actor.0].id).expect("BG actor");
                let r=&mut self.state.actors[actor.0].resources.latent;*r=r.saturating_add(25).min(cap);
            }
        }
        // Battle_ExecDamageSP has break UI/camera callbacks, but unlike HP
        // damage does not request an enforcer break voice. Its OnAttackDamageSP
        // parent is a non-native Blueprint event, not another voice callback.
        None
    }
    /// Execute scalar effects in ailment order (outside the per-target loop).
    /// `recovery_settled` belongs to the command, not an individual ailment.
    pub(super) fn grace_scalar(&mut self,actor:ActorId,targets:&[ActorId],op:&Operation,recovery_settled:&mut bool,events:&mut Vec<Event>)->Result<()> {
        use EffectKind::*;
        match op.kind {
            ReviveRatio=>{
                let speaker=self.grace_recovery_speaker(actor,targets,recovery_settled);
                for &target in targets {self.grace_revive(actor,target,op.value,speaker,events);}
            },
            DisableItems=>self.state.grace.get_or_insert_with(Default::default).items_disabled=true,
            ExpMultiplier=>self.state.grace.get_or_insert_with(Default::default).exp_requests.push(op.value),
            JpMultiplier=>self.state.grace.get_or_insert_with(Default::default).jp_requests.push(op.value),
            AddMoney=>{
                let m=self.state.grace.get_or_insert_with(Default::default);
                m.money_gained=m.money_gained.saturating_add(op.value);
            },
            HpFixedRecovery|SpFixedRecovery|SpRatioRecovery|BpRecovery|LatentRecovery=>{
                let speaker=self.grace_recovery_speaker(actor,targets,recovery_settled);
                for &target in targets {
                    let (resource,amount)=match op.kind {
                        HpFixedRecovery=>(Resource::Hp,op.value),
                        SpFixedRecovery=>(Resource::Sp,op.value),
                        SpRatioRecovery=>(Resource::Sp,(self.scenario.actors[target.0].stats.max_sp as f32*(op.value as f32*0.01_f32)) as u32),
                        BpRecovery=>(Resource::Bp,op.value),
                        LatentRecovery=>(Resource::Latent,op.value),_=>unreachable!(),
                    };
                    self.grace_recover(actor,target,resource,amount,speaker,events);
                }
            },
            BpDamage|LatentDamage|SpRatioDamage=>{
                for &target in targets {
                    let (resource,amount)=match op.kind {
                        BpDamage=>(Resource::Bp,op.value),LatentDamage=>(Resource::Latent,op.value),
                        SpRatioDamage=>(Resource::Sp,(self.scenario.actors[target.0].stats.max_sp as f32*(op.value as f32*0.01_f32)) as u32),
                        _=>unreachable!(),
                    };
                    self.grace_resource_loss(target,resource,amount,events)?;
                }
            },
            HpOne=>for &target in targets {self.grace_hp_one(actor,target,events);},
            InstantDeathExceptCaster=>for &target in targets {self.grace_instant_death(actor,target,op,events);},
            AddStatus=>for &target in targets {self.grace_status(actor,target,op,events)?;},
            RemovePartyDeconditions=>for &target in targets {self.grace_remove_deconditions(target);},
            ReverseBuffs=>for &target in targets {self.grace_reverse_buffs(target);},
            _=>return Err(SimError(format!("BG operation requires its specialized executor: {:?}",op.kind))),
        }
        Ok(())
    }
    pub(super) fn grace_recovery_speaker(&mut self,actor:ActorId,targets:&[ActorId],settled:&mut bool)->Option<ActorId> {
        if *settled {return None;}
        *settled=true;
        if targets.len()<=1 {return targets.first().copied().filter(|t|*t!=actor);}
        let before=self.state.rng;
        let mut work=targets.to_vec();
        // GenericArray_Shuffle draws for every slot, including the last one.
        for i in 0..work.len() {
            let j=i+self.state.rng.index((work.len()-i) as u32) as usize;
            work.swap(i,j);
        }
        self.trace_rng("bg_recovery_speaker_shuffle",Some(actor),before);
        work.into_iter().find(|t|*t!=actor)
    }
    pub(super) fn grace_recover(&mut self,actor:ActorId,target:ActorId,resource:Resource,amount:u32,speaker:Option<ActorId>,events:&mut Vec<Event>) {
        if !self.state.actors[target.0].alive() {return;}
        if resource==Resource::Latent {
            let cap=crate::field_escape::player_latent_max(&self.scenario.actors[target.0].id).expect("validated carried player");
            let value=&mut self.state.actors[target.0].resources.latent;
            let before=*value;
            *value=(u32::from(*value).saturating_add(amount)).min(u32::from(cap)) as u16;
            events.push(Event::ResourceChanged {actor:target,resource,before:before.into(),after:(*value).into()});
        } else {
            self.apply_effect(target,&Effect::Restore {resource,recovery:Recovery::Fixed {amount}},events);
        }
        // BattlePlayerBP recovery executes its display before OnRecovered.
        // Its timer consumes CRT even at zero variance; moving it after the
        // voice changes which conditional recipient voice succeeds.
        if resource!=Resource::Latent {self.timer_rng("recovery_notification_timer",Some(target));}
        if speaker==Some(target)&&target!=actor {
            let enforcer=self.grace_enforcer(actor);
            self.voice(target,13,VoiceCondition {recovery_enforcer:enforcer,..Default::default()});
        }
    }
    pub(super) fn grace_hp_one(&mut self,actor:ActorId,target:ActorId,events:&mut Vec<Event>) {
        if !self.state.actors[target.0].alive() {return;}
        // BG's child disables reflection/hide-out. HP1 has no accuracy,
        // critical or damage-variance roll (Battle_CheckDamageSuccessForHP1).
        let amount=self.state.actors[target.0].resources.hp-1;
        self.resolve_damage(actor,target,amount,false,false,events);
    }
    pub(super) fn grace_instant_death(&mut self,actor:ActorId,target:ActorId,op:&Operation,events:&mut Vec<Event>) {
        if actor==target||!self.state.actors[target.0].alive() {return;}
        let before=self.state.rng;
        let success=self.state.rng.weighted_bool(op.disease_rate as f32*0.01_f32);
        self.trace_rng("bg_instant_death",Some(actor),before);
        if success {
            let amount=self.state.actors[target.0].resources.hp;
            self.resolve_damage(actor,target,amount,false,false,events);
        }
    }
    pub(super) fn grace_remove_deconditions(&mut self,target:ActorId) {
        // The BG-reachable deconditions. Buff/debuff and field item-disable
        // are different categories and must survive this cleanse.
        self.state.actors[target.0].statuses.retain(|s|!matches!(s.kind,StatusKind::Silence|StatusKind::Poison));
    }
    pub(super) fn grace_reverse_buffs(&mut self,target:ActorId) {
        // REVERSE_BUFF selects buff category 1, then converts only diseases
        // with EnableResist. AddDisease is direct, without the 21-roll handler.
        let buffs:Vec<_>=self.state.actors[target.0].statuses.iter().copied().filter_map(|status| {
            let kind=match status.kind {
                StatusKind::AttackUp=>StatusKind::AttackDown,
                StatusKind::MagicUp=>StatusKind::MagicDown,
                StatusKind::SpeedUp=>StatusKind::SpeedDown,
                StatusKind::DefenseUp=>StatusKind::DefenseDown,
                StatusKind::MagicDefenseUp=>StatusKind::MagicDefenseDown,
                _=>return None,
            };
            Some((status,kind))
        }).collect();
        for (status,kind) in buffs {
            self.state.actors[target.0].statuses.retain(|s|s.kind!=status.kind);
            self.grace_add_disease(target,Status {kind,turns:status.turns});
            self.grace_disease_voice(target,kind,false);
        }
    }
    fn grace_disease_voice(&mut self,target:ActorId,kind:StatusKind,was_silent:bool) {
        if matches!(kind,StatusKind::AttackDown|StatusKind::MagicDown|StatusKind::SpeedDown|StatusKind::DefenseDown|StatusKind::MagicDefenseDown|StatusKind::Silence|StatusKind::Poison)
            && !was_silent && self.state.actors[target.0].has_status(kind) {
            // OnTakeDisease explicitly allows the first newly inflicted Silence
            // reaction; subsequent reactions remain blocked while silenced.
            self.voice_with_disease_check(target,4,VoiceCondition {decondition:true,..Default::default()},kind==StatusKind::Silence);
        }
    }
    fn grace_add_disease(&mut self,target:ActorId,status:Status) {
        let statuses=&mut self.state.actors[target.0].statuses;
        if let Some(old)=statuses.iter_mut().find(|s|s.kind==status.kind) {
            old.turns=if old.turns==255 {255} else {old.turns.saturating_add(status.turns).min(9)};
            return;
        }
        // AddDisease removes a nonpermanent opposing disease and returns;
        // it does NOT install the new disease on that same application.
        let opposite=match status.kind {
            StatusKind::AttackUp=>Some(StatusKind::AttackDown),StatusKind::AttackDown=>Some(StatusKind::AttackUp),
            StatusKind::MagicUp=>Some(StatusKind::MagicDown),StatusKind::MagicDown=>Some(StatusKind::MagicUp),
            StatusKind::DefenseUp=>Some(StatusKind::DefenseDown),StatusKind::DefenseDown=>Some(StatusKind::DefenseUp),
            StatusKind::MagicDefenseUp=>Some(StatusKind::MagicDefenseDown),StatusKind::MagicDefenseDown=>Some(StatusKind::MagicDefenseUp),
            StatusKind::SpeedUp=>Some(StatusKind::SpeedDown),StatusKind::SpeedDown=>Some(StatusKind::SpeedUp),
            _=>None,
        };
        if let Some(index)=opposite.and_then(|kind|statuses.iter().position(|s|s.kind==kind&&s.turns!=255)) {
            statuses.remove(index);return;
        }
        statuses.push(status);
    }
    /// Execute one already-targeted scalar/status operation. This intentionally
    /// does not select another child, spend action resources or advance a turn.
    pub(super) fn grace_status(&mut self, actor:ActorId,target:ActorId,op:&Operation,events:&mut Vec<Event>)->Result<()> {
        ensure(op.kind==EffectKind::AddStatus,"Not a BG status")?;
        let kind=match op.status.as_deref() {
            Some("SUPER_MAGIC")=>StatusKind::AdvancedMagic,
            Some("FORCE_CRITICAL")=>StatusKind::ForceCritical,
            Some("ATK_UP")=>StatusKind::AttackUp,
            Some("MATK_UP")=>StatusKind::MagicUp,
            Some("DEF_UP")=>StatusKind::DefenseUp,
            Some("MDEF_UP")=>StatusKind::MagicDefenseUp,
            Some("AGI_UP")=>StatusKind::SpeedUp,
            Some("DEF_DOWN")=>StatusKind::DefenseDown,
            Some("MDEF_DOWN")=>StatusKind::MagicDefenseDown,
            Some("SILENCE")=>StatusKind::Silence,
            Some("POISON")=>StatusKind::Poison,
            Some("MAGIC_REFLECTION")=>StatusKind::MagicReflection,
            Some("LAST_ACT_SPECIAL")=>StatusKind::LastAct,
            _=>return Err(SimError(format!("Unknown BG status {:?}",op.status))),
        };
        ensure(target.0<self.state.actors.len(),"Invalid BG status target")?;
        if !self.state.actors[target.0].alive() {return Ok(());}
        let before=self.state.rng;
        let rate=if op.disease_rate==0 {100} else {op.disease_rate};
        let success=self.state.rng.weighted_bool(rate as f32*0.01_f32);
        self.trace_rng("bg_status_application",Some(actor),before);
        if success {
            // Imported BG ailment rows have one actual disease in slot zero;
            // AddDisease/OnTakeDisease executes before the remaining None slots.
            let before=self.state.rng;
            let extra=op.status_random_min+self.state.rng.index(op.status_random_max-op.status_random_min+1);
            self.trace_rng("bg_status_duration",Some(target),before);
            let turns=(u32::from(op.turns)+extra).min(9) as u8;
            let status=Status {kind,turns};
            let was_silent=self.state.actors[target.0].has_status(StatusKind::Silence);
            self.grace_add_disease(target,status);
            // BattlePlayerBP.OnTakeDisease: deconditions/debuffs request type 4
            // with DECONDITION. The shared settled-damage gate prevents repeats.
            self.grace_disease_voice(target,kind,was_silent);
            events.push(Event::StatusApplied {actor:target,status});
            let before=self.state.rng;
            for _ in 1..op.status_slots {self.state.rng.index(op.status_random_max-op.status_random_min+1);}
            self.trace_rng("bg_status_remaining_slots",Some(target),before);
        }
        Ok(())
    }

    pub(super) fn grace_resource_loss(&mut self,target:ActorId,resource:Resource,amount:u32,events:&mut Vec<Event>)->Result<()> {
        ensure(target.0<self.state.actors.len(),"Invalid BG resource target")?;
        ensure(matches!(resource,Resource::Bp|Resource::Sp|Resource::Latent),"HP damage must use the damage pipeline")?;
        if !self.state.actors[target.0].alive() {return Ok(());}
        let r=&mut self.state.actors[target.0].resources;
        let (before,after)=match resource {
            Resource::Bp=>{let before=u32::from(r.bp);r.bp=r.bp.saturating_sub(amount.min(255) as u8);(before,u32::from(r.bp))},
            Resource::Sp=>{let before=r.sp;r.sp=r.sp.saturating_sub(amount);(before,r.sp)},
            Resource::Latent=>{let before=u32::from(r.latent);r.latent=r.latent.saturating_sub(amount.min(65535) as u16);(before,u32::from(r.latent))},
            Resource::Hp=>unreachable!(),
        };
        events.push(Event::ResourceChanged {actor:target,resource,before,after});
        // BP/SP handlers call PlayDamageUI even at zero. Latent damage instead
        // calls AddPotentialityActionPoint directly and has no damage UI draw.
        if resource!=Resource::Latent {self.timer_rng("bg_resource_damage_notification",Some(target));}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn battle()->Battle {
        let party:Vec<_>=[7,1,8,4].map(|id|crate::field_escape::route_player(id,5000).unwrap()).into();
        crate::field_escape::carried_step_ahead(CrtRng::new(0),crate::escape::Situation::Normal,&party,
            &["ENE_NML_FLD_WLD_250","ENE_NML_FLD_WLD_160","ENE_NML_OTR_150"],true,&crate::knowledge::EnemyHistory::new()).unwrap()
    }
    #[test]
    fn live_guards_retains_prepared_targets_through_throne_menu() {
        let mut b=battle();
        b.state.queue=vec![ActorId(3),ActorId(0),ActorId(2)];b.state.cursor=0;
        b.state.actors[3].resources.bp=3;
        b.state.rng=CrtRng::new(0);b.state.rng.advance(19_290_035);
        b.state.opening_crt.as_mut().unwrap().trace.as_mut().unwrap().clear();
        let report=b.step_player(Command::Boost {id:"hired_help_guards".into(),bp:3,target:None}).unwrap();
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        let live:serde_json::Value=serde_json::from_str(include_str!("../../data/source/live-guards-20261010.json")).unwrap();
        let live=live.as_array().unwrap();assert_eq!(live.len(),69);
        let mut checked=0;
        for span in trace {
            for position in span.before.draws..span.after.draws {
                let row=&live[(position-19_290_035) as usize];
                assert_eq!(row["position"].as_u64(),Some(position));
                let mut expected=span.before;expected.advance(position-span.before.draws);
                assert_eq!(row["before"].as_u64(),Some(u64::from(expected.state)));
                let node=row["node"].as_str().unwrap();
                let rva=row["return_rva"].as_str().unwrap();
                match span.reason {
                    "boost_voice"|"ability_voice"|"summon_ability_voice"|"order_voice"=>assert!(node.ends_with(".PlayBattleVoice")),
                    "guards_variant_preparation"=>assert_eq!(rva,"0x30D7604"),
                    "damage_notification_timer"|"break_slow_timer"|"broken_hit_slow_timer"=>assert_eq!(rva,"0x2C52E57","{position}: {}",span.reason),
                    "critical_hit_and_damage"=>assert!(node.ends_with(".CheckCritical")||node.ends_with(".CheckHit")||node.ends_with(".GetCalcValueRandomRatio")),
                    other=>panic!("unexpected live draw {position}: {other}"),
                }
                checked+=1;
            }
        }
        assert_eq!(checked,69);
        assert_eq!(b.current_actor(),Some(ActorId(0)));
        assert_eq!(b.state.rng.draws,19_290_104);
        assert_eq!(report.events.iter().filter(|e|matches!(e,Event::Damage {..}|Event::Miss {..})).count(),12);
    }
    #[test]
    fn osvald_guards_live_1e10aa9c() {
        let v:serde_json::Value=serde_json::from_str(include_str!("../../data/source/live-guards-1e10aa9c-checkpoints.json")).unwrap();
        let party:Vec<crate::PlayerCharacter>=serde_json::from_value(v["party"].clone()).unwrap();
        let enemies:Vec<_>=v["enemy_ids"].as_array().unwrap().iter().map(|v|v.as_str().unwrap()).collect();
        let entry:CrtRng=serde_json::from_value(v["battle_entry"].clone()).unwrap();
        let history=serde_json::from_value(v["enemy_knowledge"].clone()).unwrap();
        let mut b=crate::field_escape::carried_step_ahead(entry,crate::escape::Situation::Normal,&party,&enemies,true,&history).unwrap();
        let original=b.clone();
        b.step_player(Command::Defend).unwrap();
        b.step_player_with_boost_inputs(Command::Boost{id:"donate_bp".into(),target:Some(ActorId(1)),bp:1},&[1]).unwrap();
        assert_eq!(b.state.rng.draws+entry.draws,4772);
        let inputs:Vec<u8>=serde_json::from_value(v["guard_inputs"].clone()).unwrap();
        let r=b.step_player_with_boost_inputs(Command::Boost{id:"hired_help_guards".into(),target:None,bp:3},&inputs).unwrap();
        assert_eq!(b.current_actor(),Some(ActorId(0)));
        assert_eq!(r.events.iter().filter(|e|matches!(e,Event::Damage{target:ActorId(6),..})).count(),4);
        assert_eq!(b.state.actors[6].resources.hp,0);
        assert_eq!(b.state.rng.draws+entry.draws,4844);
        b.step_player(Command::Item{id:"energizing_pomegranate_m".into(),target:Some(ActorId(2))}).unwrap();
        assert_eq!(b.current_actor(),Some(ActorId(2)));
        assert_eq!(b.state.rng.draws+entry.draws,4889);
        let bg:Vec<u8>=serde_json::from_value(v["bg_inputs"].clone()).unwrap();
        let pre_bg=b.preview_boost_inputs(&bg).unwrap();
        assert_eq!(pre_bg.rng.draws+entry.draws,5002);
        assert_eq!(pre_bg.rng.state,0xA3596FFE);
        let r=b.step_player_with_boost_inputs(Command::Boost{id:"bewildering_grace".into(),target:None,bp:3},&bg).unwrap();
        let effects:Vec<_>=r.events.iter().filter_map(|e|if let Event::Action{name,..}=e {name.starts_with("ABI_DAN_MAKA").then_some(name.as_str())}else{None}).collect();
        assert_eq!(effects,["ABI_DAN_MAKA_027","ABI_DAN_MAKA_002","ABI_DAN_MAKA_018","ABI_DAN_MAKA_030"]);
        assert_eq!(b.current_actor(),Some(ActorId(1)));
        assert_eq!(b.state.rng.draws+entry.draws,5203);
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        let menu=trace.last().unwrap();
        assert_eq!(menu.reason,"order_voice");
        assert_eq!(menu.before.draws+entry.draws,5184);
        let reactions:Vec<_>=trace.iter().filter(|s|s.reason=="damage_voice"&&s.before.draws+entry.draws>=5002).collect();
        assert_eq!(reactions.len(),4);
        assert!(reactions.iter().all(|s|s.after.draws-s.before.draws==2));
        let fight=crate::gravell_routine::replay(original,&bg,&inputs).unwrap();
        let rewards=crate::gravell_routine::rewards(&fight,true).unwrap();
        let mut field=rewards.after;field.advance(2);
        assert_eq!(fight.evaluation,crate::gravell_routine::Evaluation::Accepted);
        assert_eq!(fight.after.draws+entry.draws,5207);
        assert_eq!(rewards.after.draws+entry.draws,5210);
        assert_eq!(field.draws+entry.draws,5212);
        assert_eq!(field.state,0xD2A671B8);
    }
    #[test]
    fn live_hikari_donate_bp_matches_partitio_menu() {
        let mut b=battle();
        b.state.queue=vec![ActorId(1),ActorId(3),ActorId(0),ActorId(2)];
        b.state.cursor=0;
        b.state.rng=CrtRng::new(0);b.state.rng.advance(19_290_014);
        b.state.opening_crt.as_mut().unwrap().trace.as_mut().unwrap().clear();
        b.step_player(Command::Boost {id:"donate_bp".into(),bp:1,target:Some(ActorId(3))}).unwrap();
        assert_eq!(b.current_actor(),Some(ActorId(3)));
        assert_eq!(b.state.actors[3].resources.bp,3);
        assert_eq!(b.state.rng.draws,19_290_035);
        assert_eq!(b.state.rng.state,3265071429);
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        let spans:Vec<_>=trace.iter().map(|s|(s.reason,s.before.draws,s.after.draws)).collect();
        assert_eq!(spans,vec![
            ("boost_voice",19_290_014,19_290_015),
            ("ability_voice",19_290_015,19_290_016),
            ("recovery_notification_timer",19_290_016,19_290_017),
            ("voice",19_290_017,19_290_018),
            ("order_voice",19_290_018,19_290_035),
        ]);
    }
    #[test]
    fn all_thirty_nine_effects_execute_against_real_combatants() {
        for number in 1..=39 {
            let mut b=battle();let mut events=vec![];
            let child=crate::bg_effects::child(&format!("ABI_DAN_MAKA_{number:03}")).unwrap();
            b.grace_child(ActorId(2),child,&mut events).unwrap_or_else(|e|panic!("{number}: {e}"));
            assert!(b.state.actors.iter().enumerate().all(|(i,a)|a.resources.hp<=b.scenario.actors[i].stats.max_hp));
            if number==3 {assert_eq!(b.scenario.actors[6].id,"ENE_NML_OTR_150","Chubby is transform-immune");assert!(b.state.rng.draws>b.initial.rng.draws);}
            if number==6||number==17 {assert!(b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().any(|s|s.reason=="bg_summon_initial_frame"));}
            if number==17 {assert!(b.state.actors[4..].iter().all(|a|a.has_status(StatusKind::AttackUp)&&a.has_status(StatusKind::SpeedUp)));}
            if number==39 {assert!(b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().any(|s|s.reason=="bg_random_count_setup"&&s.after.draws-s.before.draws==11));}
        }
    }
    #[test]
    fn guards_interleave_child_voices_and_damage_and_ignore_owner_attack() {
        let mut b=battle();b.state.queue=vec![ActorId(1),ActorId(0),ActorId(2),ActorId(3)];b.state.cursor=0;b.state.actors[1].resources.bp=3;
        let start=b.state.rng.draws;
        let report=b.step_player(Command::Boost {id:"hired_help_guards".into(),bp:2,target:None}).unwrap();
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        let voices:Vec<_>=trace.iter().filter(|s|s.reason=="summon_ability_voice"&&s.before.draws>=start).collect();
        assert_eq!(voices.len(),3);
        let first_damage=trace.iter().find(|s|s.reason=="critical_hit_and_damage"&&s.before.draws>=start).unwrap();
        assert!(voices[1].after.draws<=first_damage.before.draws,"second Guard voice precedes first Guard damage");
        let cat_damage:Vec<_>=report.events.iter().filter_map(|e|if let Event::Damage {target:ActorId(6),amount,..}=e {Some(*amount)}else{None}).collect();
        assert!(cat_damage.iter().all(|n|*n==31||*n==32||(*n>=39&&*n<=41)));
    }
    #[test]
    fn poison_can_kill_and_last_act_is_a_real_additional_menu() {
        let mut b=battle();b.state.actors[2].resources.hp=1;b.state.actors[2].statuses.push(Status {kind:StatusKind::Poison,turns:9});
        let before=b.state.rng.draws;b.grace_post_action(ActorId(2),&mut vec![]);
        assert!(!b.state.actors[2].alive());assert_eq!(b.state.rng.draws,before+1);
        let mut b=battle();b.state.actors[2].statuses.push(Status {kind:StatusKind::LastAct,turns:1});
        let before=b.state.queue.len();b.grace_last_act();assert_eq!(b.state.queue.len(),before+1);assert_eq!(b.state.queue.last(),Some(&ActorId(2)));
        b.grace_last_act();assert_eq!(b.state.queue.len(),before+1);
    }
    #[test]
    fn every_common_child_executes_its_authored_timeline() {
        let mut count=0;
        for child in crate::bg_effects::children().values().filter(|c|c.command.contains("/_Common/CommonCommand.")&&c.id!="ABI_MER_YOHEI_05_01") {
            let mut b=battle();let actor=ActorId(2);let mut events=vec![];
            let actors=(0..b.state.actors.len()).map(|i|crate::bg_effects::TargetActor {id:ActorId(i),side:b.side(ActorId(i)).unwrap(),alive:true}).collect::<Vec<_>>();
            let targets=child.select_targets(actor,&actors,&mut b.state.rng).unwrap();
            b.grace_common_child(actor,child,&targets,&mut events).unwrap_or_else(|e|panic!("{}: {e:?}",child.id));
            count+=1;
            if child.id=="ABI_DAN_MAKA_002" {assert_eq!(b.state.grace.as_ref().unwrap().exp_percent(),10000);}
            if child.id=="ABI_DAN_MAKA_016" {assert!(b.state.actors[..4].iter().all(|a|a.resources.bp==0));}
            if child.id=="ABI_DAN_MAKA_007" {assert!(b.state.actors[..4].iter().all(|a|a.resources.hp==1));}
            if child.id=="ABI_DAN_MAKA_029" {assert!(b.state.actors[4..].iter().any(|a|a.broken()));}
        }
        assert!(count>=35,"Every ordinary child must be exercised, got {count}");
    }
    #[test]
    fn death_then_revive_removes_current_action_and_rolls_next_insertion() {
        let mut b=battle();let caster=ActorId(2);let target=ActorId(0);
        b.state.queue=vec![caster,target,ActorId(1),ActorId(3)];b.state.cursor=0;
        b.state.next_queue=vec![target,ActorId(1),caster,ActorId(3)];
        b.state.actors[0].resources.bp=5;b.state.actors[0].resources.latent=80;
        b.state.actors[0].statuses=vec![Status {kind:StatusKind::AttackUp,turns:2},Status {kind:StatusKind::ActFirst,turns:255}];
        let hp=b.state.actors[0].resources.hp;
        b.resolve_damage(caster,target,hp,false,false,&mut vec![]);
        assert!(!b.state.queue.contains(&target));assert!(!b.state.next_queue.contains(&target));
        assert_eq!(b.state.actors[0].resources.bp,0);assert_eq!(b.state.actors[0].resources.latent,0);
        assert!(b.state.actors[0].statuses.is_empty());
        let before=b.state.rng;let mut reference=before;
        reference.next_u15(); // recovery display
        // Revive voice is conditional, but its rate selection is shared with
        // the production voice resolver. Insertion must follow that voice.
        let mut voice_copy=b.clone();voice_copy.timer_rng("test",Some(target));
        voice_copy.voice(target,14,VoiceCondition {recovery_enforcer:Some("eDANCER"),..Default::default()});
        reference=voice_copy.state.rng;let index=reference.index(4) as usize;
        b.grace_revive(caster,target,100,None,&mut vec![]);
        assert_eq!(b.state.rng,reference);assert_eq!(b.state.next_queue[index],target);
        assert!(!b.state.queue.contains(&target));assert_eq!(b.state.actors[0].resources.hp,hp);
        assert_eq!(b.state.actors[0].statuses,vec![Status {kind:StatusKind::ActFirst,turns:255}]);
    }
    #[test]
    fn wind_delay_moves_remaining_party_now_and_caster_next_turn() {
        let mut b=battle();let caster=ActorId(2);
        b.state.queue=vec![caster,ActorId(0),ActorId(4),ActorId(1),ActorId(5),ActorId(3),ActorId(6)];
        b.state.cursor=0;b.state.next_queue=b.state.queue.clone();let before=b.state.rng;
        b.grace_delay(caster,&[ActorId(0),ActorId(1),caster,ActorId(3)],10);
        assert_eq!(b.state.queue,vec![caster,ActorId(4),ActorId(5),ActorId(6),ActorId(0),ActorId(1),ActorId(3)]);
        assert_eq!(b.state.next_queue,vec![ActorId(0),ActorId(4),ActorId(1),ActorId(5),ActorId(3),ActorId(6),caster]);
        assert_eq!(b.state.rng,before);
    }
    #[test]
    fn opposing_disease_cancels_instead_of_replacing_and_permanent_buff_survives() {
        let mut b=battle();b.state.actors[0].statuses=vec![Status {kind:StatusKind::MagicDefenseUp,turns:3}];
        b.grace_add_disease(ActorId(0),Status {kind:StatusKind::MagicDefenseDown,turns:2});
        assert!(b.state.actors[0].statuses.is_empty());
        b.state.actors[0].statuses=vec![Status {kind:StatusKind::MagicDefenseUp,turns:255}];
        b.grace_add_disease(ActorId(0),Status {kind:StatusKind::MagicDefenseDown,turns:2});
        assert_eq!(b.state.actors[0].statuses.len(),2);
    }
    #[test]
    fn recovery_shuffles_once_per_command_and_retains_character_latent_cap() {
        let mut b=battle();let before=b.state.rng;let mut settled=false;
        let targets=[ActorId(0),ActorId(1),ActorId(2),ActorId(3)];
        let speaker=b.grace_recovery_speaker(ActorId(2),&targets,&mut settled);
        assert_ne!(speaker,Some(ActorId(2)));assert!(speaker.is_some());
        assert_eq!(b.state.rng.draws,before.draws+4);
        let after=b.state.rng;
        assert_eq!(b.grace_recovery_speaker(ActorId(2),&targets,&mut settled),None);
        assert_eq!(b.state.rng,after);
        b.grace_recover(ActorId(2),ActorId(1),Resource::Latent,300,None,&mut vec![]);
        assert_eq!(b.state.actors[1].resources.latent,125);assert_eq!(b.state.rng,after);
    }
    #[test]
    fn reported_shipwreck_bg_effects_share_parent_recovery_gate() {
        let v:serde_json::Value=serde_json::from_str(include_str!("../../data/source/bg-shipwreck-1e10aa9c.json")).unwrap();
        let party=serde_json::from_value::<Vec<crate::PlayerCharacter>>(v["party"].clone()).unwrap();
        let entry:CrtRng=serde_json::from_value(v["battle_entry"].clone()).unwrap();
        let history=serde_json::from_value(v["enemy_knowledge"].clone()).unwrap();
        let enemies=v["enemy_ids"].as_array().unwrap().iter().map(|e|e.as_str().unwrap()).collect::<Vec<_>>();
        let b=crate::field_escape::shipwreck_jp_battle(entry,crate::escape::Situation::Normal,&party,&enemies,&history).unwrap();
        let search_entry=b.clone();
        let (mut b,actions)=crate::nameless_routine::prefix(b).unwrap().unwrap();
        for (action,row) in actions.iter().zip(v["actions"].as_array().unwrap()) {
            assert_eq!(serde_json::to_value(action.action).unwrap(),row["action"]);
        }
        assert_eq!(b.state.rng.draws+entry.draws,10221);
        let inputs:Vec<u8>=serde_json::from_value(v["actions"][4]["inputs"].clone()).unwrap();
        let pre=b.preview_boost_inputs(&inputs).unwrap().rng;
        assert_eq!((pre.draws+entry.draws,pre.state),(10241,0x4FBF56EF));
        let report=b.step_player_with_boost_inputs(crate::gravell_routine::Action::BewilderingGrace.command(),&inputs).unwrap();
        let effects=report.events.iter().filter_map(|e|if let Event::Action{name,..}=e {name.starts_with("ABI_DAN_MAKA").then_some(name.as_str())}else{None}).collect::<Vec<_>>();
        assert_eq!(effects,["ABI_DAN_MAKA_012","ABI_DAN_MAKA_032","ABI_DAN_MAKA_033","ABI_DAN_MAKA_011"]);
        assert_eq!(b.state.grace.as_ref().unwrap().jp_percent(),100);
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        let draws=trace.iter().filter(|s|s.reason=="bewildering_grace_lottery").map(|s|s.before.draws+entry.draws).collect::<Vec<_>>();
        assert_eq!(draws,[10241,10260,10261,10262]);
        assert_eq!(trace.iter().filter(|s|s.reason=="bg_recovery_speaker_shuffle").count(),1);
        let fixed=crate::nameless_routine::search_shipwreck(search_entry,96).unwrap().expect("corrected JP plan");
        assert_eq!(fixed.state.outcome,Outcome::Victory);
        assert!(fixed.state.actors[..4].iter().all(|a|a.alive()));
        assert_eq!(fixed.state.grace.as_ref().unwrap().jp_percent(),10000);
        let dance=fixed.actions.iter().find(|a|a.action==crate::gravell_routine::Action::BewilderingGrace).unwrap();
        let lottery=fixed.trace.iter().find(|s|s.reason=="bewildering_grace_lottery").unwrap();
        eprintln!("Corrected same-entry BG: menu {}, inputs {:?}, pre-cast {} / 0x{:08X}",dance.before.draws+entry.draws,dance.inputs,lottery.before.draws+entry.draws,lottery.before.state);
    }
    #[test]
    fn every_pair_of_bg_recovery_effects_shares_one_owner() {
        // Source: BP_ActionCommandBase.Get/SetSettledPlayOnce_OnRecovered
        // delegates to the summon owner. Even self-healing consumes that gate.
        for first in [5,12,20,21,27,32,33] {for second in [5,12,20,21,27,32,33] {
            let mut b=battle();let mut settled=false;let mut events=vec![];
            let a=crate::bg_effects::child(&format!("ABI_DAN_MAKA_{first:03}")).unwrap();
            let c=crate::bg_effects::child(&format!("ABI_DAN_MAKA_{second:03}")).unwrap();
            b.grace_child_with_recovery(ActorId(2),a,&mut events,&mut settled).unwrap();
            assert!(settled);
            b.state.opening_crt.as_mut().unwrap().trace.as_mut().unwrap().clear();
            b.grace_child_with_recovery(ActorId(2),c,&mut events,&mut settled).unwrap();
            let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
            assert!(!trace.iter().any(|s|matches!(s.reason,"bg_recovery_speaker_shuffle"|"voice")),"{first} then {second}");
            // The next independent command gets a fresh gate.
            b.grace_child(ActorId(2),crate::bg_effects::child("ABI_DAN_MAKA_012").unwrap(),&mut events).unwrap();
            assert!(b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap().iter().any(|s|s.reason=="bg_recovery_speaker_shuffle"));
        }}
    }
    #[test]
    fn bg_light_retains_targets_until_final_hit_and_kill_voice() {
        let mut b=battle();
        for target in &mut b.state.actors[4..] {target.resources.hp=1;}
        b.state.opening_crt.as_mut().unwrap().trace.as_mut().unwrap().clear();
        let mut events=vec![];
        b.grace_child(ActorId(2),crate::bg_effects::child("ABI_DAN_MAKA_011").unwrap(),&mut events).unwrap();
        assert_eq!(events.iter().filter(|e|matches!(e,Event::Damage{..})).count(),9);
        assert!(b.state.actors[4..].iter().all(|a|!a.alive()));
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        assert_eq!(trace.iter().filter(|s|s.reason=="kill_voice").count(),1);
    }
    #[test]
    fn bg_direct_shield_damage_does_not_invent_a_break_voice() {
        let mut b=battle();let mut events=vec![];
        b.state.opening_crt.as_mut().unwrap().trace.as_mut().unwrap().clear();
        b.grace_child(ActorId(2),crate::bg_effects::child("ABI_DAN_MAKA_029").unwrap(),&mut events).unwrap();
        assert!(events.iter().any(|e|matches!(e,Event::Broken{..})));
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        assert!(trace.iter().any(|s|s.reason=="bg_shield_break_notification"));
        assert!(trace.iter().any(|s|s.reason=="break_slow_timer"));
        assert!(!trace.iter().any(|s|s.reason.ends_with("voice")));
    }
    #[test]
    fn exp100_survives_later_lesser_rolls_and_scattering_rejects_items_atomically() {
        let mut b=battle();let mut settled=false;let mut events=vec![];let before=b.state.rng;
        for name in ["ABI_DAN_MAKA_002","ABI_DAN_MAKA_010","ABI_DAN_MAKA_019","ABI_DAN_MAKA_015"] {
            let op=&crate::bg_effects::child(name).unwrap().operations[0];
            b.grace_scalar(ActorId(2),&[],op,&mut settled,&mut events).unwrap();
        }
        assert_eq!(b.state.rng,before);
        assert_eq!(b.state.grace.as_ref().unwrap().exp_percent(),10_000);
        let snapshot=b.state.clone();
        assert!(b.step_player(Command::Item {id:"thunder_soulstone_l".into(),target:None}).unwrap_err().0.contains("scattered"));
        assert_eq!(b.state,snapshot);
    }
    #[test]
    fn silence_mutates_actual_battle_state_and_preserves_all_status_rolls() {
        let mut b=battle();let before=b.state.rng;let mut reference=before;
        let op=&crate::bg_effects::child("ABI_DAN_MAKA_024").unwrap().operations[0];
        let slots=op.roll_status_slots(&mut reference).unwrap().unwrap();
        reference.advance(2); // Counter candidate, then the new-Silence debuff voice.
        let mut events=vec![];
        b.grace_status(ActorId(2),ActorId(0),op,&mut events).unwrap();
        assert_eq!(b.state.rng,reference);
        assert_eq!(b.state.actors[0].statuses.iter().find(|s|s.kind==StatusKind::Silence).unwrap().turns,2+slots[0] as u8);
        assert_eq!(b.state.round,1);
        assert_eq!(b.state.rng.draws-before.draws,23);
    }
    #[test]
    fn bp_depletion_is_clamped_and_does_not_change_hp_or_sp() {
        let mut b=battle();let before=b.state.actors[0].resources.clone();let rng=b.state.rng;
        b.grace_resource_loss(ActorId(0),Resource::Bp,5,&mut vec![]).unwrap();
        assert_eq!(b.state.actors[0].resources.bp,0);
        assert_eq!(b.state.actors[0].resources.hp,before.hp);
        assert_eq!(b.state.actors[0].resources.sp,before.sp);
        assert_eq!(b.state.rng.draws,rng.draws+1);
    }
    #[test]
    fn latent_depletion_does_not_borrow_bp_damage_ui_rng() {
        let mut b=battle();b.state.actors[0].resources.latent=80;let before=b.state.rng;
        b.grace_resource_loss(ActorId(0),Resource::Latent,300,&mut vec![]).unwrap();
        assert_eq!(b.state.actors[0].resources.latent,0);assert_eq!(b.state.rng,before);
    }
    #[test]
    fn hp_one_does_not_kill_or_consume_a_damage_roll() {
        let mut b=battle();let mut events=vec![];
        b.grace_hp_one(ActorId(2),ActorId(0),&mut events);
        assert_eq!(b.state.actors[0].resources.hp,1);
        assert!(!events.iter().any(|e|matches!(e,Event::KnockedOut {..})));
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        assert!(!trace.iter().any(|s|s.reason=="critical_hit_and_damage"));
    }
    #[test]
    fn silence_suppresses_reactions_but_not_its_first_infliction_or_menu_voice() {
        let mut b=battle();let target=ActorId(0);let mut events=vec![];
        let op=&crate::bg_effects::child("ABI_DAN_MAKA_024").unwrap().operations[0];
        b.grace_status(ActorId(2),target,op,&mut events).unwrap();
        b.state.opening_crt.as_mut().unwrap().settled_damage.fill(false);
        let before=b.state.rng;
        for kind in [4,11,13] {b.voice(target,kind,VoiceCondition::default());}
        assert_eq!(b.state.rng,before);
        b.state.queue=vec![target];b.state.cursor=0;
        b.order_voice(None);
        assert!(b.state.rng.draws>before.draws);
        let mut clean=b.clone();clean.state.actors[0].statuses.clear();
        clean.state.rng=before;b.state.rng=before;
        b.order_voice(None);clean.order_voice(None);
        assert_ne!(b.state.rng,clean.state.rng,"status-specific command voice must differ from healthy voice traversal");
    }
    #[test]
    fn temenos_identity_is_available_to_recovery_reactions() {
        let mut b=battle();
        Arc::make_mut(&mut b.scenario).actors[2].id="temenos".into();
        assert_eq!(b.grace_enforcer(ActorId(2)),Some("ePRIEST"));
    }
    #[test]
    fn reverse_reacts_once_and_cleanse_preserves_other_status_categories() {
        let mut b=battle();let before=b.state.rng;
        b.state.actors[0].statuses=vec![
            Status {kind:StatusKind::AttackUp,turns:5},Status {kind:StatusKind::Poison,turns:9},
            Status {kind:StatusKind::MagicReflection,turns:1},Status {kind:StatusKind::DefenseDown,turns:2}];
        b.grace_reverse_buffs(ActorId(0));
        assert_eq!(b.state.rng.draws-before.draws,2);
        let after_reverse=b.state.rng;
        b.grace_remove_deconditions(ActorId(0));
        assert_eq!(b.state.rng,after_reverse);
        assert!(b.state.actors[0].has_status(StatusKind::AttackDown));
        assert!(b.state.actors[0].has_status(StatusKind::MagicReflection));
        assert!(b.state.actors[0].has_status(StatusKind::DefenseDown));
        assert!(!b.state.actors[0].has_status(StatusKind::Poison));
    }
    #[test]
    fn instant_death_has_a_real_chance_and_caster_is_excluded() {
        let op=&crate::bg_effects::child("ABI_DAN_MAKA_008").unwrap().operations[0];
        let mut killed=false;let mut survived=false;
        for seed in [0,10_000,50_000,u32::MAX] {
            let mut b=battle();b.state.rng=CrtRng::new(seed);let before=b.state.rng;
            b.grace_instant_death(ActorId(2),ActorId(2),op,&mut vec![]);assert_eq!(b.state.rng,before);
            b.grace_instant_death(ActorId(2),ActorId(0),op,&mut vec![]);
            killed|=!b.state.actors[0].alive();survived|=b.state.actors[0].alive();
        }
        assert!(killed&&survived);
    }
    #[test]
    fn buffs_on_broken_enemies_do_not_create_broken_hit_timers() {
        let mut b=battle();b.state.actors[4].break_until_round=2;
        b.state.opening_crt.as_mut().unwrap().trace.as_mut().unwrap().clear();
        b.grace_child(ActorId(2),crate::bg_effects::child("ABI_DAN_MAKA_017").unwrap(),&mut vec![]).unwrap();
        let trace=b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap();
        assert!(!trace.iter().any(|s|s.reason=="broken_hit_slow_timer"));
    }

}




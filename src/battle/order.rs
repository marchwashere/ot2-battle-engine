//! Battle queue controls, including the route bosses' two action slots. Source: BattleActionOrderBP
//! MoveActionOrder; BattleCharacterBaseBP CheckOverTakeEnable and
//! OrderCountControl. Other special boss / ExVide paths are not enabled.
use super::*;

impl Battle {
    /// BreakActionOrder removes each outstanding row and inserts a hidden
    /// BREAK (7) row at the end. It does not shuffle or consume CRT RNG.
    pub(super) fn break_action_order(&mut self, target: ActorId) {
        for (rows,start) in [(&mut self.state.queue,self.state.cursor + 1),(&mut self.state.next_queue,0)] {
            let mut removed = false;
            for index in (start..rows.len()).rev() {
                if rows[index] == target {rows.remove(index); removed = true;}
            }
            if removed {
                rows.push(target);
            }
        }
    }
    /// GUARD and FIRST have the same boundary in the supported ordinary rows:
    /// after enemies recovering from break next turn. Neither existing GUARD
    /// nor BE_THE_FIRST is protected by CheckOverTakeEnable. The order enum's
    /// value 6 is BREAK_STYLE, not GUARD (3).
    pub(super) fn promote_next_order(&mut self, target: ActorId) {
        let Some(index) = self.state.next_queue.iter().position(|id| *id == target) else {return};
        let boundary = self.state.next_queue.iter().enumerate().filter_map(|(i,id)| {
            let s = &self.state.actors[id.0];
            (s.broken() && s.break_until_round <= self.state.round + 1).then_some(i + 1)
        }).last().unwrap_or(0);
        let id = self.state.next_queue.remove(index);
        self.state.next_queue.insert(boundary.min(self.state.next_queue.len()), id);
    }

    /// Mark either current OR next, then move marked rows together. Processing
    /// direction and shrinking boundaries preserve the order of AoE targets.
    pub(super) fn shift_action_orders(&mut self, actor: ActorId, targets: &[ActorId], positions: i32) {
        if positions == 0 {return;}
        let mut current = Vec::new();
        let mut next = Vec::new();
        for &target in targets {
            if !self.state.actors[target.0].alive() || self.state.actors[target.0].broken() {continue;}
            if target != actor && self.state.queue[self.state.cursor + 1..].contains(&target) {
                current.push(target);
            } else {
                next.push(target);
            }
        }
        for (rows,start,selected,is_next) in [
            (&mut self.state.queue,self.state.cursor + 1,current,false),
            (&mut self.state.next_queue,0,next,true),
        ] {
            if start >= rows.len() || selected.is_empty() {continue;}
            let mut top = start;
            let mut end = rows.len() - 1;
            for (i,id) in rows.iter().enumerate().skip(start) {
                let s = &self.state.actors[id.0];
                if s.broken() {
                    if is_next && s.break_until_round <= self.state.round + 1 {top = i + 1;}
                    else {end = i.saturating_sub(1);}
                }
            }
            if top > end {continue;}
            let indices: Vec<_> = if positions < 0 {(start..rows.len()).collect()}
                else {(start..rows.len()).rev().collect()};
            for i in indices {
                if !selected.contains(&rows[i]) {continue;}
                let to = (i as i64 + i64::from(positions)).clamp(top as i64,end as i64) as usize;
                let id = rows.remove(i);
                rows.insert(to,id);
                if positions < 0 && to == top {top = (top + 1).min(end);}
                else if positions > 0 && to == end {end = end.saturating_sub(1).max(top);}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn battle() -> Battle {
        let party: Vec<_> = [7,1,8,4].map(|id| crate::field_escape::route_player(id,5000).unwrap()).into();
        crate::field_escape::carried_step_ahead(CrtRng::new(0), crate::escape::Situation::Normal,
            &party, &["ENE_NML_FLD_WLD_250","ENE_NML_FLD_WLD_160","ENE_NML_OTR_150"],
            false, &Default::default()).unwrap()
    }
    fn boots(b: &mut Battle, target: usize, turns: u8) {
        b.apply_action_effect(ActorId(0),ActorId(target), &Effect::Status {
            status: Status {kind:StatusKind::ActFirst,turns}
        }, &mut vec![]);
    }
    #[test]
    fn boots_and_defend_follow_application_order_without_reordering_current_turn() {
        let mut b = battle();
        b.state.queue = (0..4).map(ActorId).collect(); b.state.cursor = 0;
        b.state.next_queue = (0..7).rev().map(ActorId).collect();
        let current = b.state.queue.clone();
        boots(&mut b,2,5);
        assert_eq!(b.state.queue,current);
        assert_eq!(b.state.next_queue[0],ActorId(2));
        b.step_player(Command::Defend).unwrap();
        assert_eq!(&b.state.next_queue[..2], &[ActorId(0),ActorId(2)]);
        b.step_player(Command::Defend).unwrap();
        assert_eq!(&b.state.next_queue[..3], &[ActorId(1),ActorId(0),ActorId(2)]);
        boots(&mut b,2,5);
        assert_eq!(&b.state.next_queue[..3], &[ActorId(2),ActorId(1),ActorId(0)]);
        assert_eq!(b.state.queue,current);
    }
    #[test]
    fn all_four_defends_reverse_the_party_next_turn_and_do_not_persist() {
        for seed in 0..64 {
            let mut b = battle(); b.state.rng = CrtRng::new(seed);
            let opening = b.state.turn_order().to_vec();
            for _ in 0..4 {b.step_player(Command::Defend).unwrap();}
            assert_eq!(b.state.round,2);
            let reversed: Vec<_> = opening.into_iter().rev().collect();
            assert_eq!(&b.state.turn_order()[..4],reversed.as_slice());
            // A surviving guard status is damage protection, not a persistent
            // ordering modifier in the newly generated T3 preview.
            let mut unguarded = b.clone();
            for s in &mut unguarded.state.actors {s.guard = false;}
            assert_eq!(b.source_order(3),unguarded.source_order(3));
            assert_eq!(b.state.rng,unguarded.state.rng);
        }
    }
    #[test]
    fn recovering_break_stays_ahead_of_boots_and_defend() {
        let mut b = battle();
        b.state.round = 2;
        b.state.next_queue = [4,3,2,1,0,5,6].map(ActorId).into();
        b.state.actors[4].break_until_round = 3;
        boots(&mut b,2,5);
        assert_eq!(&b.state.next_queue[..2], &[ActorId(4),ActorId(2)]);
        b.promote_next_order(ActorId(1));
        assert_eq!(&b.state.next_queue[..3], &[ActorId(4),ActorId(1),ActorId(2)]);
        assert_eq!(b.source_order(3)[0],ActorId(4));
    }
    #[test]
    fn boots_are_sorted_before_shuffle_and_expire_without_resorting_current_queue() {
        let mut b = battle();
        for s in &mut b.state.actors[3..] {s.resources.hp = 0;}
        // Three Boots recipients cannot enter any shuffle window; their exact
        // order is descending speed, not shuffled order followed by partition.
        for id in 0..3 {boots(&mut b,id,1);}
        let before = b.state.rng;
        let order = b.source_order(2);
        let mut expected: Vec<_> = (0..3).map(ActorId).collect();
        expected.sort_by_key(|id| std::cmp::Reverse(b.scenario.actors[id.0].stats.speed));
        assert_eq!(order,expected);
        assert_eq!(b.state.rng,before,"Boots must suppress the actual shuffle RNG calls");
        b.state.next_queue = order.clone();
        b.start_ot2_round(&mut vec![]);
        assert_eq!(b.state.queue,order,"Already generated T2 order survives expiration");
        assert!(b.state.actors[..3].iter().all(|s| !s.has_status(StatusKind::ActFirst)));
        let span = b.state.opening_crt.as_ref().unwrap().trace.as_ref().unwrap().last().unwrap();
        assert_eq!(span.reason,"next_order_shuffle");
        assert!(span.after.draws > span.before.draws,"T3 shuffle resumes after expiration");
    }
    #[test]
    fn ordinary_shift_changes_only_the_targets_outstanding_turn() {
        let mut b = battle();
        b.state.queue = [0,1,4,2,5,3,6].map(ActorId).into(); b.state.cursor = 0;
        b.state.next_queue = [6,5,4,3,2,1,0].map(ActorId).into();
        let next = b.state.next_queue.clone(); let rng = b.state.rng;
        b.apply_action_effect(ActorId(0),ActorId(2),&Effect::OrderShift {positions:-1},&mut vec![]);
        assert_eq!(b.state.queue,[0,1,2,4,5,3,6].map(ActorId));
        assert_eq!(b.state.next_queue,next);
        let current = b.state.queue.clone();
        b.apply_action_effect(ActorId(0),ActorId(0),&Effect::OrderShift {positions:-1},&mut vec![]);
        assert_eq!(b.state.queue,current);
        assert_eq!(b.state.next_queue,[6,5,4,3,2,0,1].map(ActorId));
        assert_eq!(b.state.rng,rng);
    }
    #[test]
    fn aoe_delay_keeps_relative_order_and_respects_broken_reserved_rows() {
        let mut b = battle();
        b.state.queue = [0,1,2,3,4,5,6].map(ActorId).into(); b.state.cursor = 0;
        b.state.actors[6].break_until_round = b.state.round + 2;
        b.shift_action_orders(ActorId(0),&[ActorId(1),ActorId(2)],100);
        assert_eq!(b.state.queue,[0,3,4,5,1,2,6].map(ActorId));
    }
    #[test]
    fn breaking_moves_hidden_rows_to_the_end_before_later_delay_effects() {
        let mut b = battle();
        b.state.queue = [0,4,1,5,2,6,3].map(ActorId).into(); b.state.cursor = 0;
        b.state.next_queue = b.state.queue.clone();
        b.state.actors[4].break_until_round = b.state.round + 2;
        let rng = b.state.rng;
        b.break_action_order(ActorId(4));
        assert_eq!(b.state.queue,[0,1,5,2,6,3,4].map(ActorId));
        assert_eq!(b.state.next_queue,b.state.queue);
        b.shift_action_orders(ActorId(0),&[ActorId(5),ActorId(6)],1);
        assert_eq!(b.state.queue,[0,1,2,5,3,6,4].map(ActorId));
        assert_eq!(b.state.rng,rng);
    }
    #[test]
    fn break_collapses_multiple_outstanding_rows_to_one_hidden_entry() {
        let mut b = battle();
        b.state.queue = [0,4,1,4,2].map(ActorId).into(); b.state.cursor = 0;
        b.state.next_queue = [4,1,4,2].map(ActorId).into();
        b.break_action_order(ActorId(4));
        assert_eq!(b.state.queue,[0,1,2,4].map(ActorId));
        assert_eq!(b.state.next_queue,[1,2,4].map(ActorId));
    }
}

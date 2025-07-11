use crate::{
    consts::MAX_PATH_DIST,
    data::*,
    util::{entities_have_line_of_sight, handle_attack_aggression},
};
use common::{
    combat::{self, AttackSource},
    comp::{
        Ability, AbilityInput, Agent, CharacterAbility, CharacterState, ControlAction,
        ControlEvent, Controller, Fluid, InputKind,
        ability::{ActiveAbilities, AuxiliaryAbility, BASE_ABILITY_LIMIT, Stance, SwordStance},
        buff::BuffKind,
        fluid_dynamics::LiquidKind,
        item::tool::AbilityContext,
        skills::{AxeSkill, BowSkill, HammerSkill, SceptreSkill, Skill, StaffSkill, SwordSkill},
    },
    consts::GRAVITY,
    path::TraversalConfig,
    states::{
        self_buff,
        sprite_summon::{self, SpriteSummonAnchor},
        utils::StageSection,
    },
    terrain::Block,
    util::Dir,
    vol::ReadVol,
};
use rand::{Rng, prelude::SliceRandom};
use std::{f32::consts::PI, time::Duration};
use vek::*;
use world::util::CARDINALS;

// ground-level max range from projectile speed and launch height
fn projectile_flat_range(speed: f32, height: f32) -> f32 {
    let w = speed.powi(2);
    let u = 0.5 * 2_f32.sqrt() * speed;
    (0.5 * w + u * (0.5 * w + 2.0 * GRAVITY * height).sqrt()) / GRAVITY
}

// multi-projectile spread (in degrees) based on maximum of linear increase
fn projectile_multi_angle(projectile_spread: f32, num_projectiles: u32) -> f32 {
    (180.0 / PI) * projectile_spread * (num_projectiles - 1) as f32
}

fn rng_from_span(rng: &mut impl Rng, span: [f32; 2]) -> f32 { rng.gen_range(span[0]..=span[1]) }

impl AgentData<'_> {
    // Intended for any agent that has one attack, that attack is a melee attack,
    // and the agent is able to freely walk around
    pub fn handle_simple_melee(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        if attack_data.in_min_range() && attack_data.angle < 30.0 {
            controller.push_basic_input(InputKind::Primary);
            controller.inputs.move_dir = Vec2::zero();
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && rng.gen::<f32>() < 0.02
            {
                controller.push_basic_input(InputKind::Roll);
            }
        }
    }

    // Intended for any agent that has one attack, that attack is a melee attack,
    // and the agent is able to freely fly around
    pub fn handle_simple_flying_melee(
        &self,
        _agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        // Fly to target
        let dir_to_target = ((tgt_data.pos.0 + Vec3::unit_z() * 1.5) - self.pos.0)
            .try_normalized()
            .unwrap_or_else(Vec3::zero);
        let speed = 1.0;
        controller.inputs.move_dir = dir_to_target.xy() * speed;

        // Always fly! If the floor can't touch you, it can't hurt you...
        controller.push_basic_input(InputKind::Fly);
        // Flee from the ground! The internet told me it was lava!
        // If on the ground, jump with every last ounce of energy, holding onto
        // all that is dear in life and straining for the wide open skies.
        if self.physics_state.on_ground.is_some() {
            controller.push_basic_input(InputKind::Jump);
        } else {
            // Use a proportional controller with a coefficient of 1.0 to
            // maintain altidude at the the provided set point
            let mut maintain_altitude = |set_point| {
                let alt = read_data
                    .terrain
                    .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 7.0))
                    .until(Block::is_solid)
                    .cast()
                    .0;
                let error = set_point - alt;
                controller.inputs.move_z = error;
            };
            if (tgt_data.pos.0 - self.pos.0).xy().magnitude_squared() > (5.0_f32).powi(2) {
                maintain_altitude(5.0);
            } else {
                maintain_altitude(2.0);

                // Attack if in range
                if attack_data.dist_sqrd < 3.5_f32.powi(2) && attack_data.angle < 150.0 {
                    controller.push_basic_input(InputKind::Primary);
                }
            }
        }
    }

    pub fn handle_bloodmoon_bat_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            AttackTimer,
        }

        let home = agent.patrol_origin.unwrap_or(self.pos.0.round());

        agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] > 8.0 {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
        }

        let dir_to_target = ((tgt_data.pos.0 + Vec3::unit_z() * 1.5) - self.pos.0)
            .try_normalized()
            .unwrap_or_else(Vec3::zero);
        let speed = 1.0;
        controller.inputs.move_dir = dir_to_target.xy() * speed;

        // Always fly
        controller.push_basic_input(InputKind::Fly);
        if self.physics_state.on_ground.is_some() {
            controller.push_basic_input(InputKind::Jump);
        } else {
            // Use a proportional controller with a coefficient of 1.0 to
            // maintain altidude at the the provided set point
            let mut maintain_altitude = |set_point| {
                let alt = read_data
                    .terrain
                    .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 7.0))
                    .until(Block::is_solid)
                    .cast()
                    .0;
                let error = set_point - alt;
                controller.inputs.move_z = error;
            };
            if !(-20.6..20.6).contains(&(tgt_data.pos.0.y - home.y))
                || !(-26.6..26.6).contains(&(tgt_data.pos.0.x - home.x))
            {
                if (home - self.pos.0).xy().magnitude_squared() > (5.0_f32).powi(2) {
                    controller.push_action(ControlAction::StartInput {
                        input: InputKind::Ability(0),
                        target_entity: None,
                        select_pos: Some(home),
                    });
                } else {
                    controller.push_basic_input(InputKind::Ability(1));
                }
            } else if (tgt_data.pos.0 - self.pos.0).xy().magnitude_squared() > (5.0_f32).powi(2) {
                maintain_altitude(5.0);
            } else {
                maintain_altitude(2.0);
                if tgt_data.pos.0.z < home.z + 5.0 && self.pos.0.z < home.z + 25.0 {
                    if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 3.0 {
                        controller.push_basic_input(InputKind::Secondary);
                    } else {
                        controller.push_basic_input(InputKind::Ability(1));
                    }
                } else if attack_data.dist_sqrd < 6.0_f32.powi(2) {
                    // use shockwave or singlestrike when close
                    if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 2.0 {
                        controller.push_basic_input(InputKind::Ability(2));
                    } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize]
                        < 4.0
                    {
                        controller.push_basic_input(InputKind::Ability(3));
                    } else {
                        controller.push_basic_input(InputKind::Primary);
                    }
                } else if tgt_data.pos.0.z < home.z + 30.0
                    && agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 3.0
                {
                    controller.push_action(ControlAction::StartInput {
                        input: InputKind::Ability(0),
                        target_entity: agent
                            .target
                            .as_ref()
                            .and_then(|t| read_data.uids.get(t.target))
                            .copied(),
                        select_pos: None,
                    });
                }
            }
        }
    }

    pub fn handle_vampire_bat_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        _attack_data: &AttackData,
        _tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            AttackTimer,
        }

        agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] > 9.0 {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
        }

        // stay centered
        let home = agent.patrol_origin.unwrap_or(self.pos.0.round());
        self.path_toward_target(agent, controller, home, read_data, Path::Full, None);
        // teleport home if straying too far
        if (home - self.pos.0).xy().magnitude_squared() > (10.0_f32).powi(2) {
            controller.push_action(ControlAction::StartInput {
                input: InputKind::Ability(1),
                target_entity: None,
                select_pos: Some(home),
            });
        }
        // Always fly! If the floor can't touch you, it can't hurt you...
        controller.push_basic_input(InputKind::Fly);
        if self.pos.0.z < home.z + 4.0
            && agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] > 6.0
        {
            controller.push_basic_input(InputKind::Secondary);
        } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 3.0
            && (self.pos.0.z - home.z) < 110.0
        {
            controller.push_basic_input(InputKind::Primary);
        } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 6.0 {
            controller.push_basic_input(InputKind::Ability(0));
        }
    }

    pub fn handle_bloodmoon_heiress_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const DASH_TIMER: usize = 0;
        const SUMMON_THRESHOLD: f32 = 0.20;
        enum ActionStateFCounters {
            FCounterHealthThreshold = 0,
        }
        enum ActionStateConditions {
            ConditionCounterInit = 0,
        }
        agent.combat_state.timers[DASH_TIMER] += read_data.dt.0;
        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already initialized
        if !agent.combat_state.conditions[ActionStateConditions::ConditionCounterInit as usize] {
            agent.combat_state.counters[ActionStateFCounters::FCounterHealthThreshold as usize] =
                1.0 - SUMMON_THRESHOLD;
            agent.combat_state.conditions[ActionStateConditions::ConditionCounterInit as usize] =
                true;
        }

        if agent.combat_state.counters[ActionStateFCounters::FCounterHealthThreshold as usize]
            > health_fraction
        {
            // Summon minions at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(2));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterHealthThreshold as usize] -= SUMMON_THRESHOLD;
            }
        }
        // teleport to target when it can't be pathed to
        else if self
            .path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            )
            .is_none()
            || !(-3.0..3.0).contains(&(tgt_data.pos.0.z - self.pos.0.z))
        {
            controller.push_action(ControlAction::StartInput {
                input: InputKind::Ability(0),
                target_entity: agent
                    .target
                    .as_ref()
                    .and_then(|t| read_data.uids.get(t.target))
                    .copied(),
                select_pos: None,
            });
        } else if matches!(self.char_state, CharacterState::DashMelee(s) if !matches!(s.stage_section, StageSection::Recover))
        {
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.in_min_range() && attack_data.angle < 45.0 {
            if agent.combat_state.timers[DASH_TIMER] > 2.0 {
                agent.combat_state.timers[DASH_TIMER] = 0.0;
            }
            match rng.gen_range(0..2) {
                0 => controller.push_basic_input(InputKind::Primary),
                _ => controller.push_basic_input(InputKind::Ability(3)),
            };
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2)
            && self
                .path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    None,
                )
                .is_some()
            && line_of_sight_with_target()
            && agent.combat_state.timers[DASH_TIMER] > 4.0
            && attack_data.angle < 45.0
        {
            match rng.gen_range(0..2) {
                0 => controller.push_basic_input(InputKind::Secondary),
                _ => controller.push_basic_input(InputKind::Ability(1)),
            };
            agent.combat_state.timers[DASH_TIMER] = 0.0;
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    // Intended for any agent that has one attack, that attack is a melee attack,
    // the agent is able to freely walk around, and the agent is trying to attack
    // from behind its target
    pub fn handle_simple_backstab(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        // Behaviour parameters
        const STRAFE_DIST: f32 = 4.5;
        const STRAFE_SPEED_MULT: f32 = 0.75;
        const STRAFE_SPIRAL_MULT: f32 = 0.8; // how quickly they close gap while strafing
        const BACKSTAB_SPEED_MULT: f32 = 0.3;

        // Handle movement of agent
        let target_ori = agent
            .target
            .and_then(|t| read_data.orientations.get(t.target))
            .map(|ori| ori.look_vec())
            .unwrap_or_default();
        let dist = attack_data.dist_sqrd.sqrt();
        let in_front_of_target = target_ori.dot(self.pos.0 - tgt_data.pos.0) > 0.0;

        // Handle attacking of agent
        if attack_data.in_min_range() && attack_data.angle < 30.0 {
            controller.push_basic_input(InputKind::Primary);
            controller.inputs.move_dir = Vec2::zero();
        }

        if attack_data.dist_sqrd < STRAFE_DIST.powi(2) {
            // If in front of the target, circle to try and get behind, else just make a
            // beeline for the back of the agent
            let vec_to_target = (tgt_data.pos.0 - self.pos.0).xy();
            if in_front_of_target {
                let theta = (PI / 2. - dist * 0.1).max(0.0);
                // Checks both CW and CCW rotation
                let potential_move_dirs = [
                    vec_to_target
                        .rotated_z(theta)
                        .try_normalized()
                        .unwrap_or_default(),
                    vec_to_target
                        .rotated_z(-theta)
                        .try_normalized()
                        .unwrap_or_default(),
                ];
                // Finds shortest path to get behind
                if let Some(move_dir) = potential_move_dirs
                    .iter()
                    .find(|move_dir| target_ori.xy().dot(**move_dir) < 0.0)
                {
                    controller.inputs.move_dir =
                        STRAFE_SPEED_MULT * (*move_dir - STRAFE_SPIRAL_MULT * target_ori.xy());
                }
            } else {
                // Aim for a point a given distance behind the target to prevent sideways
                // movement
                let move_target = tgt_data.pos.0.xy() - dist / 2. * target_ori.xy();
                controller.inputs.move_dir = ((move_target - self.pos.0) * BACKSTAB_SPEED_MULT)
                    .try_normalized()
                    .unwrap_or_default();
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
    }

    pub fn handle_elevated_ranged(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        // Behaviour parameters
        const PREF_DIST: f32 = 30.0;
        const RETREAT_DIST: f32 = 8.0;

        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        let elevation = self.pos.0.z - tgt_data.pos.0.z;

        if attack_data.angle_xy < 30.0
            && (elevation > 10.0 || attack_data.dist_sqrd > PREF_DIST.powi(2))
            && line_of_sight_with_target()
        {
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd < RETREAT_DIST.powi(2) {
            // Attempt to move quickly away from target if too close
            if let Some((bearing, _)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                let flee_dir = -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero);
                let pos = self.pos.0.xy().with_z(self.pos.0.z + 1.5);
                if read_data
                    .terrain
                    .ray(pos, pos + flee_dir * 2.0)
                    .until(|b| b.is_solid() || b.get_sprite().is_none())
                    .cast()
                    .0
                    > 1.0
                {
                    // If able to flee, flee
                    controller.inputs.move_dir = flee_dir;
                    if !self.char_state.is_attack() {
                        controller.inputs.look_dir = -controller.inputs.look_dir;
                    }
                } else {
                    // Otherwise, fight to the death
                    controller.push_basic_input(InputKind::Primary);
                }
            }
        } else if attack_data.dist_sqrd < PREF_DIST.powi(2) {
            // Attempt to move away from target if too close, while still attacking
            if let Some((bearing, _)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if line_of_sight_with_target() {
                    controller.push_basic_input(InputKind::Primary);
                }
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero);
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
    }

    pub fn handle_hammer_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        if !agent.combat_state.initialized {
            agent.combat_state.initialized = true;
            let available_tactics = {
                let mut tactics = Vec::new();
                let try_tactic = |skill, tactic, tactics: &mut Vec<HammerTactics>| {
                    if self.skill_set.has_skill(Skill::Hammer(skill)) {
                        tactics.push(tactic);
                    }
                };
                try_tactic(
                    HammerSkill::Thunderclap,
                    HammerTactics::AttackExpert,
                    &mut tactics,
                );
                try_tactic(
                    HammerSkill::Judgement,
                    HammerTactics::SupportExpert,
                    &mut tactics,
                );
                if tactics.is_empty() {
                    try_tactic(
                        HammerSkill::IronTempest,
                        HammerTactics::AttackAdvanced,
                        &mut tactics,
                    );
                    try_tactic(
                        HammerSkill::Rampart,
                        HammerTactics::SupportAdvanced,
                        &mut tactics,
                    );
                }
                if tactics.is_empty() {
                    try_tactic(
                        HammerSkill::Retaliate,
                        HammerTactics::AttackIntermediate,
                        &mut tactics,
                    );
                    try_tactic(
                        HammerSkill::PileDriver,
                        HammerTactics::SupportIntermediate,
                        &mut tactics,
                    );
                }
                if tactics.is_empty() {
                    try_tactic(
                        HammerSkill::Tremor,
                        HammerTactics::AttackSimple,
                        &mut tactics,
                    );
                    try_tactic(
                        HammerSkill::HeavyWhorl,
                        HammerTactics::SupportSimple,
                        &mut tactics,
                    );
                }
                if tactics.is_empty() {
                    try_tactic(
                        HammerSkill::ScornfulSwipe,
                        HammerTactics::Simple,
                        &mut tactics,
                    );
                }
                if tactics.is_empty() {
                    tactics.push(HammerTactics::Unskilled);
                }
                tactics
            };

            let tactic = available_tactics
                .choose(rng)
                .copied()
                .unwrap_or(HammerTactics::Unskilled);

            agent.combat_state.int_counters[IntCounters::Tactic as usize] = tactic as u8;

            let auxiliary_key = ActiveAbilities::active_auxiliary_key(Some(self.inventory));
            let set_ability = |controller: &mut Controller, slot, skill| {
                controller.push_event(ControlEvent::ChangeAbility {
                    slot,
                    auxiliary_key,
                    new_ability: AuxiliaryAbility::MainWeapon(skill),
                });
            };
            let mut set_random = |controller: &mut Controller, slot, options: &mut Vec<usize>| {
                if options.is_empty() {
                    return;
                }
                let i = rng.gen_range(0..options.len());
                set_ability(controller, slot, options.swap_remove(i));
            };

            match tactic {
                HammerTactics::Unskilled => {},
                HammerTactics::Simple => {
                    // Scornful swipe
                    set_ability(controller, 0, 0);
                },
                HammerTactics::AttackSimple => {
                    // Scornful swipe
                    set_ability(controller, 0, 0);
                    // Tremor or vigorous bash
                    set_ability(controller, 1, rng.gen_range(1..3));
                },
                HammerTactics::AttackIntermediate => {
                    // Scornful swipe
                    set_ability(controller, 0, 0);
                    // Tremor or vigorous bash
                    set_ability(controller, 1, rng.gen_range(1..3));
                    // Retaliate, spine cracker, or breach
                    set_ability(controller, 2, rng.gen_range(3..6));
                },
                HammerTactics::AttackAdvanced => {
                    // Scornful swipe, tremor, vigorous bash, retaliate, spine cracker, or breach
                    let mut options = vec![0, 1, 2, 3, 4, 5];
                    set_random(controller, 0, &mut options);
                    set_random(controller, 1, &mut options);
                    set_random(controller, 2, &mut options);
                    set_ability(controller, 3, rng.gen_range(6..8));
                },
                HammerTactics::AttackExpert => {
                    // Scornful swipe, tremor, vigorous bash, retaliate, spine cracker, breach, iron
                    // tempest, or upheaval
                    let mut options = vec![0, 1, 2, 3, 4, 5, 6, 7];
                    set_random(controller, 0, &mut options);
                    set_random(controller, 1, &mut options);
                    set_random(controller, 2, &mut options);
                    set_random(controller, 3, &mut options);
                    set_ability(controller, 4, rng.gen_range(8..10));
                },
                HammerTactics::SupportSimple => {
                    // Scornful swipe
                    set_ability(controller, 0, 0);
                    // Heavy whorl or intercept
                    set_ability(controller, 1, rng.gen_range(10..12));
                },
                HammerTactics::SupportIntermediate => {
                    // Scornful swipe
                    set_ability(controller, 0, 0);
                    // Heavy whorl or intercept
                    set_ability(controller, 1, rng.gen_range(10..12));
                    // Retaliate, spine cracker, or breach
                    set_ability(controller, 2, rng.gen_range(12..15));
                },
                HammerTactics::SupportAdvanced => {
                    // Scornful swipe, heavy whorl, intercept, pile driver, lung pummel, or helm
                    // crusher
                    let mut options = vec![0, 10, 11, 12, 13, 14];
                    set_random(controller, 0, &mut options);
                    set_random(controller, 1, &mut options);
                    set_random(controller, 2, &mut options);
                    set_ability(controller, 3, rng.gen_range(15..17));
                },
                HammerTactics::SupportExpert => {
                    // Scornful swipe, heavy whorl, intercept, pile driver, lung pummel, helm
                    // crusher, rampart, or tenacity
                    let mut options = vec![0, 10, 11, 12, 13, 14, 15, 16];
                    set_random(controller, 0, &mut options);
                    set_random(controller, 1, &mut options);
                    set_random(controller, 2, &mut options);
                    set_random(controller, 3, &mut options);
                    set_ability(controller, 4, rng.gen_range(17..19));
                },
            }

            agent.combat_state.int_counters[IntCounters::ActionMode as usize] =
                ActionMode::Reckless as u8;
        }

        enum IntCounters {
            Tactic = 0,
            ActionMode = 1,
        }

        enum Timers {
            GuardedCycle = 0,
            PosTimeOut = 1,
        }

        enum Conditions {
            GuardedDefend = 0,
            RollingBreakThrough = 1,
        }

        enum FloatCounters {
            GuardedTimer = 0,
        }

        enum Positions {
            GuardedCover = 0,
            Flee = 1,
        }

        let attempt_attack = handle_attack_aggression(
            self,
            agent,
            controller,
            attack_data,
            tgt_data,
            read_data,
            rng,
            Timers::PosTimeOut as usize,
            Timers::GuardedCycle as usize,
            FloatCounters::GuardedTimer as usize,
            IntCounters::ActionMode as usize,
            Conditions::GuardedDefend as usize,
            Conditions::RollingBreakThrough as usize,
            Positions::GuardedCover as usize,
            Positions::Flee as usize,
        );

        let attack_failed = if attempt_attack {
            let primary = self.extract_ability(AbilityInput::Primary);
            let secondary = self.extract_ability(AbilityInput::Secondary);
            let abilities = [
                self.extract_ability(AbilityInput::Auxiliary(0)),
                self.extract_ability(AbilityInput::Auxiliary(1)),
                self.extract_ability(AbilityInput::Auxiliary(2)),
                self.extract_ability(AbilityInput::Auxiliary(3)),
                self.extract_ability(AbilityInput::Auxiliary(4)),
            ];
            let could_use_input = |input, desired_energy| match input {
                InputKind::Primary => primary.as_ref().is_some_and(|p| {
                    p.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                    s.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                    let ability = self.active_abilities.get_ability(
                        AbilityInput::Auxiliary(x),
                        Some(self.inventory),
                        Some(self.skill_set),
                        self.stats,
                    );
                    let additional_conditions = match ability {
                        Ability::MainWeaponAux(0) => self
                            .buffs
                            .is_some_and(|buffs| !buffs.contains(BuffKind::ScornfulTaunt)),
                        Ability::MainWeaponAux(2) => {
                            tgt_data.char_state.is_some_and(|cs| cs.is_stunned())
                        },
                        Ability::MainWeaponAux(4) => tgt_data.ori.is_some_and(|ori| {
                            ori.look_vec().angle_between(tgt_data.pos.0 - self.pos.0)
                                < combat::BEHIND_TARGET_ANGLE
                        }),
                        Ability::MainWeaponAux(5) => tgt_data.char_state.is_some_and(|cs| {
                            cs.is_block(AttackSource::Melee) || cs.is_parry(AttackSource::Melee)
                        }),
                        Ability::MainWeaponAux(7) => tgt_data
                            .buffs
                            .is_some_and(|buffs| !buffs.contains(BuffKind::OffBalance)),
                        Ability::MainWeaponAux(12) => tgt_data
                            .buffs
                            .is_some_and(|buffs| !buffs.contains(BuffKind::Rooted)),
                        Ability::MainWeaponAux(13) => tgt_data
                            .buffs
                            .is_some_and(|buffs| !buffs.contains(BuffKind::Winded)),
                        Ability::MainWeaponAux(14) => tgt_data
                            .buffs
                            .is_some_and(|buffs| !buffs.contains(BuffKind::Amnesia)),
                        Ability::MainWeaponAux(15) => self
                            .buffs
                            .is_some_and(|buffs| !buffs.contains(BuffKind::ProtectingWard)),
                        _ => true,
                    };
                    a.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                        && additional_conditions
                }),
                _ => false,
            };
            let continue_current_input = |current_input, next_input: &mut Option<InputKind>| {
                if matches!(current_input, InputKind::Secondary) {
                    let charging =
                        matches!(self.char_state.stage_section(), Some(StageSection::Charge));
                    let charged = self
                        .char_state
                        .durations()
                        .and_then(|durs| durs.charge)
                        .zip(self.char_state.timer())
                        .is_some_and(|(dur, timer)| timer > dur);
                    if !(charging && charged) {
                        *next_input = Some(InputKind::Secondary);
                    }
                } else {
                    *next_input = Some(current_input);
                }
            };
            let current_input = self.char_state.ability_info().map(|ai| ai.input);
            let ability_preferences = AbilityPreferences {
                desired_energy: 40.0,
                combo_scaling_buildup: 0,
            };
            let mut next_input = None;
            if let Some(input) = current_input {
                continue_current_input(input, &mut next_input);
            } else {
                match HammerTactics::from_u8(
                    agent.combat_state.int_counters[IntCounters::Tactic as usize],
                ) {
                    HammerTactics::Unskilled => {
                        if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    HammerTactics::Simple => {
                        if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    HammerTactics::AttackSimple | HammerTactics::SupportSimple => {
                        if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    HammerTactics::AttackIntermediate | HammerTactics::SupportIntermediate => {
                        let random_ability = InputKind::Ability(rng.gen_range(0..3));
                        if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    HammerTactics::AttackAdvanced | HammerTactics::SupportAdvanced => {
                        let random_ability = InputKind::Ability(rng.gen_range(0..5));
                        if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    HammerTactics::AttackExpert | HammerTactics::SupportExpert => {
                        let random_ability = InputKind::Ability(rng.gen_range(0..5));
                        if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                }
            }
            if let Some(input) = next_input {
                if could_use_input(input, ability_preferences) {
                    controller.push_basic_input(input);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            false
        };

        if attack_failed && attack_data.dist_sqrd > 1.5_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_sword_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        if !agent.combat_state.initialized {
            agent.combat_state.initialized = true;
            let available_tactics = {
                let mut tactics = Vec::new();
                let try_tactic = |skill, tactic, tactics: &mut Vec<SwordTactics>| {
                    if self.skill_set.has_skill(Skill::Sword(skill)) {
                        tactics.push(tactic);
                    }
                };
                try_tactic(
                    SwordSkill::HeavyFortitude,
                    SwordTactics::HeavyAdvanced,
                    &mut tactics,
                );
                try_tactic(
                    SwordSkill::AgileDancingEdge,
                    SwordTactics::AgileAdvanced,
                    &mut tactics,
                );
                try_tactic(
                    SwordSkill::DefensiveStalwartSword,
                    SwordTactics::DefensiveAdvanced,
                    &mut tactics,
                );
                try_tactic(
                    SwordSkill::CripplingEviscerate,
                    SwordTactics::CripplingAdvanced,
                    &mut tactics,
                );
                try_tactic(
                    SwordSkill::CleavingBladeFever,
                    SwordTactics::CleavingAdvanced,
                    &mut tactics,
                );
                if tactics.is_empty() {
                    try_tactic(
                        SwordSkill::HeavySweep,
                        SwordTactics::HeavySimple,
                        &mut tactics,
                    );
                    try_tactic(
                        SwordSkill::AgileQuickDraw,
                        SwordTactics::AgileSimple,
                        &mut tactics,
                    );
                    try_tactic(
                        SwordSkill::DefensiveDisengage,
                        SwordTactics::DefensiveSimple,
                        &mut tactics,
                    );
                    try_tactic(
                        SwordSkill::CripplingGouge,
                        SwordTactics::CripplingSimple,
                        &mut tactics,
                    );
                    try_tactic(
                        SwordSkill::CleavingWhirlwindSlice,
                        SwordTactics::CleavingSimple,
                        &mut tactics,
                    );
                }
                if tactics.is_empty() {
                    try_tactic(SwordSkill::CrescentSlash, SwordTactics::Basic, &mut tactics);
                }
                if tactics.is_empty() {
                    tactics.push(SwordTactics::Unskilled);
                }
                tactics
            };

            let tactic = available_tactics
                .choose(rng)
                .copied()
                .unwrap_or(SwordTactics::Unskilled);

            agent.combat_state.int_counters[IntCounters::Tactics as usize] = tactic as u8;

            let auxiliary_key = ActiveAbilities::active_auxiliary_key(Some(self.inventory));
            let set_sword_ability = |controller: &mut Controller, slot, skill| {
                controller.push_event(ControlEvent::ChangeAbility {
                    slot,
                    auxiliary_key,
                    new_ability: AuxiliaryAbility::MainWeapon(skill),
                });
            };

            match tactic {
                SwordTactics::Unskilled => {},
                SwordTactics::Basic => {
                    // Crescent slash
                    set_sword_ability(controller, 0, 0);
                    // Fell strike
                    set_sword_ability(controller, 1, 1);
                    // Skewer
                    set_sword_ability(controller, 2, 2);
                    // Cascade
                    set_sword_ability(controller, 3, 3);
                    // Cross cut
                    set_sword_ability(controller, 4, 4);
                },
                SwordTactics::HeavySimple => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Crescent slash
                    set_sword_ability(controller, 1, 0);
                    // Cascade
                    set_sword_ability(controller, 2, 3);
                    // Windmill slash
                    set_sword_ability(controller, 3, 6);
                    // Pommel strike
                    set_sword_ability(controller, 4, 7);
                },
                SwordTactics::AgileSimple => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Skewer
                    set_sword_ability(controller, 1, 2);
                    // Cross cut
                    set_sword_ability(controller, 2, 4);
                    // Quick draw
                    set_sword_ability(controller, 3, 8);
                    // Feint
                    set_sword_ability(controller, 4, 9);
                },
                SwordTactics::DefensiveSimple => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Crescent slash
                    set_sword_ability(controller, 1, 0);
                    // Fell strike
                    set_sword_ability(controller, 2, 1);
                    // Riposte
                    set_sword_ability(controller, 3, 10);
                    // Disengage
                    set_sword_ability(controller, 4, 11);
                },
                SwordTactics::CripplingSimple => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Fell strike
                    set_sword_ability(controller, 1, 1);
                    // Skewer
                    set_sword_ability(controller, 2, 2);
                    // Gouge
                    set_sword_ability(controller, 3, 12);
                    // Hamstring
                    set_sword_ability(controller, 4, 13);
                },
                SwordTactics::CleavingSimple => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Cascade
                    set_sword_ability(controller, 1, 3);
                    // Cross cut
                    set_sword_ability(controller, 2, 4);
                    // Whirlwind slice
                    set_sword_ability(controller, 3, 14);
                    // Earth splitter
                    set_sword_ability(controller, 4, 15);
                },
                SwordTactics::HeavyAdvanced => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Windmill slash
                    set_sword_ability(controller, 1, 6);
                    // Pommel strike
                    set_sword_ability(controller, 2, 7);
                    // Fortitude
                    set_sword_ability(controller, 3, 16);
                    // Pillar Thrust
                    set_sword_ability(controller, 4, 17);
                },
                SwordTactics::AgileAdvanced => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Quick draw
                    set_sword_ability(controller, 1, 8);
                    // Feint
                    set_sword_ability(controller, 2, 9);
                    // Dancing edge
                    set_sword_ability(controller, 3, 18);
                    // Flurry
                    set_sword_ability(controller, 4, 19);
                },
                SwordTactics::DefensiveAdvanced => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Riposte
                    set_sword_ability(controller, 1, 10);
                    // Disengage
                    set_sword_ability(controller, 2, 11);
                    // Stalwart sword
                    set_sword_ability(controller, 3, 20);
                    // Deflect
                    set_sword_ability(controller, 4, 21);
                },
                SwordTactics::CripplingAdvanced => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Gouge
                    set_sword_ability(controller, 1, 12);
                    // Hamstring
                    set_sword_ability(controller, 2, 13);
                    // Eviscerate
                    set_sword_ability(controller, 3, 22);
                    // Bloody gash
                    set_sword_ability(controller, 4, 23);
                },
                SwordTactics::CleavingAdvanced => {
                    // Finisher
                    set_sword_ability(controller, 0, 5);
                    // Whirlwind slice
                    set_sword_ability(controller, 1, 14);
                    // Earth splitter
                    set_sword_ability(controller, 2, 15);
                    // Blade fever
                    set_sword_ability(controller, 3, 24);
                    // Sky splitter
                    set_sword_ability(controller, 4, 25);
                },
            }

            agent.combat_state.int_counters[IntCounters::ActionMode as usize] =
                ActionMode::Reckless as u8;
        }

        enum IntCounters {
            Tactics = 0,
            ActionMode = 1,
        }

        enum Timers {
            GuardedCycle = 0,
            PosTimeOut = 1,
        }

        enum Conditions {
            GuardedDefend = 0,
            RollingBreakThrough = 1,
        }

        enum FloatCounters {
            GuardedTimer = 0,
        }

        enum Positions {
            GuardedCover = 0,
            Flee = 1,
        }

        let attempt_attack = handle_attack_aggression(
            self,
            agent,
            controller,
            attack_data,
            tgt_data,
            read_data,
            rng,
            Timers::PosTimeOut as usize,
            Timers::GuardedCycle as usize,
            FloatCounters::GuardedTimer as usize,
            IntCounters::ActionMode as usize,
            Conditions::GuardedDefend as usize,
            Conditions::RollingBreakThrough as usize,
            Positions::GuardedCover as usize,
            Positions::Flee as usize,
        );

        let attack_failed = if attempt_attack {
            let primary = self.extract_ability(AbilityInput::Primary);
            let secondary = self.extract_ability(AbilityInput::Secondary);
            let abilities = [
                self.extract_ability(AbilityInput::Auxiliary(0)),
                self.extract_ability(AbilityInput::Auxiliary(1)),
                self.extract_ability(AbilityInput::Auxiliary(2)),
                self.extract_ability(AbilityInput::Auxiliary(3)),
                self.extract_ability(AbilityInput::Auxiliary(4)),
            ];
            let could_use_input = |input, desired_energy| match input {
                InputKind::Primary => primary.as_ref().is_some_and(|p| {
                    p.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                    s.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                    a.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                _ => false,
            };
            let continue_current_input = |current_input, next_input: &mut Option<InputKind>| {
                if matches!(current_input, InputKind::Secondary) {
                    let charging =
                        matches!(self.char_state.stage_section(), Some(StageSection::Charge));
                    let charged = self
                        .char_state
                        .durations()
                        .and_then(|durs| durs.charge)
                        .zip(self.char_state.timer())
                        .is_some_and(|(dur, timer)| timer > dur);
                    if !(charging && charged) {
                        *next_input = Some(InputKind::Secondary);
                    }
                } else {
                    *next_input = Some(current_input);
                }
            };
            match SwordTactics::from_u8(
                agent.combat_state.int_counters[IntCounters::Tactics as usize],
            ) {
                SwordTactics::Unskilled => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 15.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else if rng.gen_bool(0.5) {
                        next_input = Some(InputKind::Primary);
                    } else {
                        next_input = Some(InputKind::Secondary);
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::Basic => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 25.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let attempt_ability = InputKind::Ability(rng.gen_range(0..5));
                        if could_use_input(attempt_ability, ability_preferences) {
                            next_input = Some(attempt_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::HeavySimple => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 35.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(3..5));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Heavy))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::AgileSimple => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 35.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(3..5));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Agile))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::DefensiveSimple => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 35.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(3..5));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Defensive))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(InputKind::Ability(3), ability_preferences) {
                            next_input = Some(InputKind::Ability(3));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::CripplingSimple => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 35.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(3..5));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Crippling))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::CleavingSimple => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 35.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(3..5));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Cleaving))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::HeavyAdvanced => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 50.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(1..3));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Heavy))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::AgileAdvanced => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 50.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(1..3));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Agile))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::DefensiveAdvanced => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 50.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(1..3));
                        let random_ability = InputKind::Ability(rng.gen_range(1..4));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Defensive))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if could_use_input(InputKind::Ability(4), ability_preferences)
                            && rng.gen_bool(2.0 * read_data.dt.0 as f64)
                        {
                            next_input = Some(InputKind::Ability(4));
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::CripplingAdvanced => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 50.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(1..3));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Crippling))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
                SwordTactics::CleavingAdvanced => {
                    let ability_preferences = AbilityPreferences {
                        desired_energy: 50.0,
                        combo_scaling_buildup: 0,
                    };
                    let current_input = self.char_state.ability_info().map(|ai| ai.input);
                    let mut next_input = None;
                    if let Some(input) = current_input {
                        continue_current_input(input, &mut next_input);
                    } else {
                        let stance_ability = InputKind::Ability(rng.gen_range(1..3));
                        let random_ability = InputKind::Ability(rng.gen_range(1..5));
                        if !matches!(self.stance, Some(Stance::Sword(SwordStance::Cleaving))) {
                            if could_use_input(stance_ability, ability_preferences) {
                                next_input = Some(stance_ability);
                            } else if rng.gen_bool(0.5) {
                                next_input = Some(InputKind::Primary);
                            } else {
                                next_input = Some(InputKind::Secondary);
                            }
                        } else if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    };
                    if let Some(input) = next_input {
                        if could_use_input(input, ability_preferences) {
                            controller.push_basic_input(input);
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                },
            }
        } else {
            false
        };

        if attack_failed && attack_data.dist_sqrd > 1.5_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_axe_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        if !agent.combat_state.initialized {
            agent.combat_state.initialized = true;
            let available_tactics = {
                let mut tactics = Vec::new();
                let try_tactic = |skill, tactic, tactics: &mut Vec<AxeTactics>| {
                    if self.skill_set.has_skill(Skill::Axe(skill)) {
                        tactics.push(tactic);
                    }
                };
                try_tactic(AxeSkill::Execute, AxeTactics::SavageAdvanced, &mut tactics);
                try_tactic(
                    AxeSkill::Lacerate,
                    AxeTactics::MercilessAdvanced,
                    &mut tactics,
                );
                try_tactic(AxeSkill::Bulkhead, AxeTactics::RivingAdvanced, &mut tactics);
                if tactics.is_empty() {
                    try_tactic(
                        AxeSkill::RisingTide,
                        AxeTactics::SavageIntermediate,
                        &mut tactics,
                    );
                    try_tactic(
                        AxeSkill::FierceRaze,
                        AxeTactics::MercilessIntermediate,
                        &mut tactics,
                    );
                    try_tactic(
                        AxeSkill::Plunder,
                        AxeTactics::RivingIntermediate,
                        &mut tactics,
                    );
                }
                if tactics.is_empty() {
                    try_tactic(
                        AxeSkill::BrutalSwing,
                        AxeTactics::SavageSimple,
                        &mut tactics,
                    );
                    try_tactic(AxeSkill::Rake, AxeTactics::MercilessSimple, &mut tactics);
                    try_tactic(AxeSkill::SkullBash, AxeTactics::RivingSimple, &mut tactics);
                }
                if tactics.is_empty() {
                    tactics.push(AxeTactics::Unskilled);
                }
                tactics
            };

            let tactic = available_tactics
                .choose(rng)
                .copied()
                .unwrap_or(AxeTactics::Unskilled);

            agent.combat_state.int_counters[IntCounters::Tactic as usize] = tactic as u8;

            let auxiliary_key = ActiveAbilities::active_auxiliary_key(Some(self.inventory));
            let set_axe_ability = |controller: &mut Controller, slot, skill| {
                controller.push_event(ControlEvent::ChangeAbility {
                    slot,
                    auxiliary_key,
                    new_ability: AuxiliaryAbility::MainWeapon(skill),
                });
            };

            match tactic {
                AxeTactics::Unskilled => {},
                AxeTactics::SavageSimple => {
                    // Brutal swing
                    set_axe_ability(controller, 0, 0);
                },
                AxeTactics::MercilessSimple => {
                    // Rake
                    set_axe_ability(controller, 0, 6);
                },
                AxeTactics::RivingSimple => {
                    // Skull bash
                    set_axe_ability(controller, 0, 12);
                },
                AxeTactics::SavageIntermediate => {
                    // Brutal swing
                    set_axe_ability(controller, 0, 0);
                    // Berserk
                    set_axe_ability(controller, 1, 1);
                    // Rising tide
                    set_axe_ability(controller, 2, 2);
                },
                AxeTactics::MercilessIntermediate => {
                    // Rake
                    set_axe_ability(controller, 0, 6);
                    // Bloodfeast
                    set_axe_ability(controller, 1, 7);
                    // Fierce raze
                    set_axe_ability(controller, 2, 8);
                },
                AxeTactics::RivingIntermediate => {
                    // Skull bash
                    set_axe_ability(controller, 0, 12);
                    // Sunder
                    set_axe_ability(controller, 1, 13);
                    // Plunder
                    set_axe_ability(controller, 2, 14);
                },
                AxeTactics::SavageAdvanced => {
                    // Berserk
                    set_axe_ability(controller, 0, 1);
                    // Rising tide
                    set_axe_ability(controller, 1, 2);
                    // Savage sense
                    set_axe_ability(controller, 2, 3);
                    // Adrenaline rush
                    set_axe_ability(controller, 3, 4);
                    // Execute/maelstrom
                    set_axe_ability(controller, 4, 5);
                },
                AxeTactics::MercilessAdvanced => {
                    // Bloodfeast
                    set_axe_ability(controller, 0, 7);
                    // Fierce raze
                    set_axe_ability(controller, 1, 8);
                    // Furor
                    set_axe_ability(controller, 2, 9);
                    // Fracture
                    set_axe_ability(controller, 3, 10);
                    // Lacerate/riptide
                    set_axe_ability(controller, 4, 11);
                },
                AxeTactics::RivingAdvanced => {
                    // Sunder
                    set_axe_ability(controller, 0, 13);
                    // Plunder
                    set_axe_ability(controller, 1, 14);
                    // Defiance
                    set_axe_ability(controller, 2, 15);
                    // Keelhaul
                    set_axe_ability(controller, 3, 16);
                    // Bulkhead/capsize
                    set_axe_ability(controller, 4, 17);
                },
            }

            agent.combat_state.int_counters[IntCounters::ActionMode as usize] =
                ActionMode::Reckless as u8;
        }

        enum IntCounters {
            Tactic = 0,
            ActionMode = 1,
        }

        enum Timers {
            GuardedCycle = 0,
            PosTimeOut = 1,
        }

        enum Conditions {
            GuardedDefend = 0,
            RollingBreakThrough = 1,
        }

        enum FloatCounters {
            GuardedTimer = 0,
        }

        enum Positions {
            GuardedCover = 0,
            Flee = 1,
        }

        let attempt_attack = handle_attack_aggression(
            self,
            agent,
            controller,
            attack_data,
            tgt_data,
            read_data,
            rng,
            Timers::PosTimeOut as usize,
            Timers::GuardedCycle as usize,
            FloatCounters::GuardedTimer as usize,
            IntCounters::ActionMode as usize,
            Conditions::GuardedDefend as usize,
            Conditions::RollingBreakThrough as usize,
            Positions::GuardedCover as usize,
            Positions::Flee as usize,
        );

        let attack_failed = if attempt_attack {
            let primary = self.extract_ability(AbilityInput::Primary);
            let secondary = self.extract_ability(AbilityInput::Secondary);
            let abilities = [
                self.extract_ability(AbilityInput::Auxiliary(0)),
                self.extract_ability(AbilityInput::Auxiliary(1)),
                self.extract_ability(AbilityInput::Auxiliary(2)),
                self.extract_ability(AbilityInput::Auxiliary(3)),
                self.extract_ability(AbilityInput::Auxiliary(4)),
            ];
            let could_use_input = |input, desired_energy| match input {
                InputKind::Primary => primary.as_ref().is_some_and(|p| {
                    p.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                    s.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                    a.could_use(attack_data, self, tgt_data, read_data, desired_energy)
                }),
                _ => false,
            };
            let continue_current_input = |current_input, next_input: &mut Option<InputKind>| {
                if matches!(current_input, InputKind::Secondary) {
                    let charging =
                        matches!(self.char_state.stage_section(), Some(StageSection::Charge));
                    let charged = self
                        .char_state
                        .durations()
                        .and_then(|durs| durs.charge)
                        .zip(self.char_state.timer())
                        .is_some_and(|(dur, timer)| timer > dur);
                    if !(charging && charged) {
                        *next_input = Some(InputKind::Secondary);
                    }
                } else {
                    *next_input = Some(current_input);
                }
            };
            let current_input = self.char_state.ability_info().map(|ai| ai.input);
            let ability_preferences = AbilityPreferences {
                desired_energy: 40.0,
                combo_scaling_buildup: 15,
            };
            let mut next_input = None;
            if let Some(input) = current_input {
                continue_current_input(input, &mut next_input);
            } else {
                match AxeTactics::from_u8(
                    agent.combat_state.int_counters[IntCounters::Tactic as usize],
                ) {
                    AxeTactics::Unskilled => {
                        if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    AxeTactics::SavageSimple
                    | AxeTactics::MercilessSimple
                    | AxeTactics::RivingSimple => {
                        if could_use_input(InputKind::Ability(0), ability_preferences) {
                            next_input = Some(InputKind::Ability(0));
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    AxeTactics::SavageIntermediate
                    | AxeTactics::MercilessIntermediate
                    | AxeTactics::RivingIntermediate => {
                        let random_ability = InputKind::Ability(rng.gen_range(0..3));
                        if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                    AxeTactics::SavageAdvanced
                    | AxeTactics::MercilessAdvanced
                    | AxeTactics::RivingAdvanced => {
                        let random_ability = InputKind::Ability(rng.gen_range(0..5));
                        if could_use_input(random_ability, ability_preferences) {
                            next_input = Some(random_ability);
                        } else if rng.gen_bool(0.5) {
                            next_input = Some(InputKind::Primary);
                        } else {
                            next_input = Some(InputKind::Secondary);
                        }
                    },
                }
            }
            if let Some(input) = next_input {
                if could_use_input(input, ability_preferences) {
                    controller.push_basic_input(input);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            false
        };

        if attack_failed && attack_data.dist_sqrd > 1.5_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_bow_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const MIN_CHARGE_FRAC: f32 = 0.5;
        const OPTIMAL_TARGET_VELOCITY: f32 = 5.0;
        const DESIRED_ENERGY_LEVEL: f32 = 50.0;

        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        // Logic to use abilities
        if let CharacterState::ChargedRanged(c) = self.char_state {
            if !matches!(c.stage_section, StageSection::Recover) {
                // Don't even bother with this logic if in recover
                let target_speed_sqd = agent
                    .target
                    .as_ref()
                    .map(|t| t.target)
                    .and_then(|e| read_data.velocities.get(e))
                    .map_or(0.0, |v| v.0.magnitude_squared());
                if c.charge_frac() < MIN_CHARGE_FRAC
                    || (target_speed_sqd > OPTIMAL_TARGET_VELOCITY.powi(2) && c.charge_frac() < 1.0)
                {
                    // If haven't charged to desired level, or target is moving too fast and haven't
                    // fully charged, keep charging
                    controller.push_basic_input(InputKind::Primary);
                }
                // Else don't send primary input to release the shot
            }
        } else if matches!(self.char_state, CharacterState::RepeaterRanged(c) if self.energy.current() > 5.0 && !matches!(c.stage_section, StageSection::Recover))
        {
            // If in repeater ranged, have enough energy, and aren't in recovery, try to
            // keep firing
            if attack_data.dist_sqrd > attack_data.min_attack_dist.powi(2)
                && line_of_sight_with_target()
            {
                // Only keep firing if not in melee range or if can see target
                controller.push_basic_input(InputKind::Secondary);
            }
        } else if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            if self
                .skill_set
                .has_skill(Skill::Bow(BowSkill::UnlockShotgun))
                && self.energy.current() > 45.0
                && rng.gen_bool(0.5)
            {
                // Use shotgun if target close and have sufficient energy
                controller.push_basic_input(InputKind::Ability(0));
            } else if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && self.energy.current()
                    > CharacterAbility::default_roll(Some(self.char_state)).energy_cost()
                && !matches!(self.char_state, CharacterState::BasicRanged(c) if !matches!(c.stage_section, StageSection::Recover))
            {
                // Else roll away if can roll and have enough energy, and not using shotgun
                // (other 2 attacks have interrupt handled above) unless in recover
                controller.push_basic_input(InputKind::Roll);
            } else {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    None,
                );
                if attack_data.angle < 15.0 {
                    controller.push_basic_input(InputKind::Primary);
                }
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) && line_of_sight_with_target() {
            // If not really far, and can see target, attempt to shoot bow
            if self.energy.current() < DESIRED_ENERGY_LEVEL {
                // If low on energy, use primary to attempt to regen energy
                controller.push_basic_input(InputKind::Primary);
            } else {
                // Else we have enough energy, use repeater
                controller.push_basic_input(InputKind::Secondary);
            }
        }
        // Logic to move. Intentionally kept separate from ability logic so duplicated
        // work is less necessary.
        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if line_of_sight_with_target() && attack_data.angle < 45.0 {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(rng.gen_range(0.5..1.57))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            }
            // Sometimes try to roll
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && rng.gen::<f32>() < 0.01
            {
                controller.push_basic_input(InputKind::Roll);
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_staff_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        enum ActionStateConditions {
            ConditionStaffCanShockwave = 0,
        }
        let context = AbilityContext::from(self.stance, Some(self.inventory), self.combo);
        let extract_ability = |input: AbilityInput| {
            self.active_abilities
                .activate_ability(
                    input,
                    Some(self.inventory),
                    self.skill_set,
                    self.body,
                    Some(self.char_state),
                    &context,
                    self.stats,
                )
                .map_or(Default::default(), |a| a.0)
        };
        let (flamethrower, shockwave) = (
            extract_ability(AbilityInput::Secondary),
            extract_ability(AbilityInput::Auxiliary(0)),
        );
        let flamethrower_range = match flamethrower {
            CharacterAbility::BasicBeam { range, .. } => range,
            _ => 20.0_f32,
        };
        let shockwave_cost = shockwave.energy_cost();
        if self.body.is_some_and(|b| b.is_humanoid())
            && attack_data.in_min_range()
            && self.energy.current()
                > CharacterAbility::default_roll(Some(self.char_state)).energy_cost()
            && !matches!(self.char_state, CharacterState::Shockwave(_))
        {
            // if a humanoid, have enough stamina, not in shockwave, and in melee range,
            // emergency roll
            controller.push_basic_input(InputKind::Roll);
        } else if matches!(self.char_state, CharacterState::Shockwave(_)) {
            agent.combat_state.conditions
                [ActionStateConditions::ConditionStaffCanShockwave as usize] = false;
        } else if agent.combat_state.conditions
            [ActionStateConditions::ConditionStaffCanShockwave as usize]
            && matches!(self.char_state, CharacterState::Wielding(_))
        {
            controller.push_basic_input(InputKind::Ability(0));
        } else if !matches!(self.char_state, CharacterState::Shockwave(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // only try to use another ability unless in shockwave or recover
            let target_approaching_speed = -agent
                .target
                .as_ref()
                .map(|t| t.target)
                .and_then(|e| read_data.velocities.get(e))
                .map_or(0.0, |v| v.0.dot(self.ori.look_vec()));
            if self
                .skill_set
                .has_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                && target_approaching_speed > 12.0
                && self.energy.current() > shockwave_cost
            {
                // if enemy is closing distance quickly, use shockwave to knock back
                if matches!(self.char_state, CharacterState::Wielding(_)) {
                    controller.push_basic_input(InputKind::Ability(0));
                } else {
                    agent.combat_state.conditions
                        [ActionStateConditions::ConditionStaffCanShockwave as usize] = true;
                }
            } else if self.energy.current()
                > shockwave_cost
                    + CharacterAbility::default_roll(Some(self.char_state)).energy_cost()
                && attack_data.dist_sqrd < flamethrower_range.powi(2)
            {
                controller.push_basic_input(InputKind::Secondary);
            } else {
                controller.push_basic_input(InputKind::Primary);
            }
        }
        // Logic to move. Intentionally kept separate from ability logic so duplicated
        // work is less necessary.
        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                ) && attack_data.angle < 45.0
                {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(rng.gen_range(-1.57..-0.5))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            }
            // Sometimes try to roll
            if self.body.is_some_and(|b| b.is_humanoid())
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && !matches!(self.char_state, CharacterState::Shockwave(_))
                && rng.gen::<f32>() < 0.02
            {
                controller.push_basic_input(InputKind::Roll);
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_sceptre_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const DESIRED_ENERGY_LEVEL: f32 = 50.0;
        const DESIRED_COMBO_LEVEL: u32 = 8;

        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        // Logic to use abilities
        if attack_data.dist_sqrd > attack_data.min_attack_dist.powi(2)
            && line_of_sight_with_target()
        {
            // If far enough away, and can see target, check which skill is appropriate to
            // use
            if self.energy.current() > DESIRED_ENERGY_LEVEL
                && read_data
                    .combos
                    .get(*self.entity)
                    .is_some_and(|c| c.counter() >= DESIRED_COMBO_LEVEL)
                && !read_data.buffs.get(*self.entity).iter().any(|buff| {
                    buff.iter_kind(BuffKind::Regeneration)
                        .peekable()
                        .peek()
                        .is_some()
                })
            {
                // If have enough energy and combo to use healing aura, do so
                controller.push_basic_input(InputKind::Secondary);
            } else if self
                .skill_set
                .has_skill(Skill::Sceptre(SceptreSkill::UnlockAura))
                && self.energy.current() > DESIRED_ENERGY_LEVEL
                && !read_data.buffs.get(*self.entity).iter().any(|buff| {
                    buff.iter_kind(BuffKind::ProtectingWard)
                        .peekable()
                        .peek()
                        .is_some()
                })
            {
                // Use ward if target is far enough away, self is not buffed, and have
                // sufficient energy
                controller.push_basic_input(InputKind::Ability(0));
            } else {
                // If low on energy, use primary to attempt to regen energy
                // Or if at desired energy level but not able/willing to ward, just attack
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            if self.body.is_some_and(|b| b.is_humanoid())
                && self.energy.current()
                    > CharacterAbility::default_roll(Some(self.char_state)).energy_cost()
                && !matches!(self.char_state, CharacterState::BasicAura(c) if !matches!(c.stage_section, StageSection::Recover))
            {
                // Else roll away if can roll and have enough energy, and not using aura or in
                // recover
                controller.push_basic_input(InputKind::Roll);
            } else if attack_data.angle < 15.0 {
                controller.push_basic_input(InputKind::Primary);
            }
        }
        // Logic to move. Intentionally kept separate from ability logic where possible
        // so duplicated work is less necessary.
        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if line_of_sight_with_target() && attack_data.angle < 45.0 {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(rng.gen_range(0.5..1.57))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            }
            // Sometimes try to roll
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && !matches!(self.char_state, CharacterState::BasicAura(_))
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && rng.gen::<f32>() < 0.01
            {
                controller.push_basic_input(InputKind::Roll);
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_stone_golem_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerHandleStoneGolemAttack = 0, //Timer 0
        }

        if attack_data.in_min_range() && attack_data.angle < 90.0 {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
            //controller.inputs.primary.set_state(true);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if self.vel.0.is_approx_zero() {
                controller.push_basic_input(InputKind::Ability(0));
            }
            if self
                .path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    None,
                )
                .is_some()
                && entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
                && attack_data.angle < 90.0
            {
                if agent.combat_state.timers
                    [ActionStateTimers::TimerHandleStoneGolemAttack as usize]
                    > 5.0
                {
                    controller.push_basic_input(InputKind::Secondary);
                    agent.combat_state.timers
                        [ActionStateTimers::TimerHandleStoneGolemAttack as usize] = 0.0;
                } else {
                    agent.combat_state.timers
                        [ActionStateTimers::TimerHandleStoneGolemAttack as usize] += read_data.dt.0;
                }
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_iron_golem_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            AttackTimer = 0,
        }

        let home = agent.patrol_origin.unwrap_or(self.pos.0);

        let attack_select =
            if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 3.0 {
                0
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 4.5 {
                1
            } else {
                2
            };
        // stay centered
        if (home - self.pos.0).xy().magnitude_squared() > (3.0_f32).powi(2) {
            self.path_toward_target(agent, controller, home, read_data, Path::Full, None);
        // shoot at targets above
        } else if tgt_data.pos.0.z > home.z + 5.0 {
            controller.push_basic_input(InputKind::Ability(0))
        } else if attack_data.in_min_range() {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
        } else {
            match attack_select {
                0 => {
                    // firebolt
                    controller.push_basic_input(InputKind::Ability(0))
                },
                1 => {
                    // spin
                    controller.push_basic_input(InputKind::Ability(1))
                },
                _ => {
                    // shockwave
                    controller.push_basic_input(InputKind::Secondary)
                },
            };
        };
        agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] > 7.5 {
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
        };
    }

    pub fn handle_circle_charge_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        radius: u32,
        circle_time: u32,
        rng: &mut impl Rng,
    ) {
        enum ActionStateCountersF {
            CounterFHandleCircleChargeAttack = 0,
        }

        enum ActionStateCountersI {
            CounterIHandleCircleChargeAttack = 0,
        }

        if agent.combat_state.counters
            [ActionStateCountersF::CounterFHandleCircleChargeAttack as usize]
            >= circle_time as f32
        {
            // if circle charge is in progress and time hasn't expired, continue charging
            controller.push_basic_input(InputKind::Secondary);
        }
        if attack_data.in_min_range() {
            if agent.combat_state.counters
                [ActionStateCountersF::CounterFHandleCircleChargeAttack as usize]
                > 0.0
            {
                // set timer and rotation counter to zero if in minimum range
                agent.combat_state.counters
                    [ActionStateCountersF::CounterFHandleCircleChargeAttack as usize] = 0.0;
                agent.combat_state.int_counters
                    [ActionStateCountersI::CounterIHandleCircleChargeAttack as usize] = 0;
            } else {
                // melee attack
                controller.push_basic_input(InputKind::Primary);
                controller.inputs.move_dir = Vec2::zero();
            }
        } else if attack_data.dist_sqrd < (radius as f32 + attack_data.min_attack_dist).powi(2) {
            // if in range to charge, circle, then charge
            if agent.combat_state.int_counters
                [ActionStateCountersI::CounterIHandleCircleChargeAttack as usize]
                == 0
            {
                // if you haven't chosen a direction to go in, choose now
                agent.combat_state.int_counters
                    [ActionStateCountersI::CounterIHandleCircleChargeAttack as usize] =
                    1 + rng.gen_bool(0.5) as u8;
            }
            if agent.combat_state.counters
                [ActionStateCountersF::CounterFHandleCircleChargeAttack as usize]
                < circle_time as f32
            {
                // circle if circle timer not ready
                let move_dir = match agent.combat_state.int_counters
                    [ActionStateCountersI::CounterIHandleCircleChargeAttack as usize]
                {
                    1 =>
                    // circle left if counter is 1
                    {
                        (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y)
                    },
                    2 =>
                    // circle right if counter is 2
                    {
                        (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(-0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y)
                    },
                    _ =>
                    // if some illegal value slipped in, get zero vector
                    {
                        Vec2::zero()
                    },
                };
                let obstacle = read_data
                    .terrain
                    .ray(
                        self.pos.0 + Vec3::unit_z(),
                        self.pos.0 + move_dir.with_z(0.0) * 2.0 + Vec3::unit_z(),
                    )
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_some());
                if obstacle {
                    // if obstacle detected, stop circling
                    agent.combat_state.counters
                        [ActionStateCountersF::CounterFHandleCircleChargeAttack as usize] =
                        circle_time as f32;
                }
                controller.inputs.move_dir = move_dir;
                // use counter as timer since timer may be modified in other parts of the code
                agent.combat_state.counters
                    [ActionStateCountersF::CounterFHandleCircleChargeAttack as usize] +=
                    read_data.dt.0;
            }
            // activating charge once circle timer expires is handled above
        } else {
            let path = if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
                // if too far away from target, move towards them
                Path::Separate
            } else {
                Path::Partial
            };
            self.path_toward_target(agent, controller, tgt_data.pos.0, read_data, path, None);
        }
    }

    pub fn handle_quadlow_ranged_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerHandleQuadLowRanged = 0,
        }

        if attack_data.dist_sqrd < (3.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 90.0
        {
            controller.inputs.move_dir = if !attack_data.in_min_range() {
                (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y)
            } else {
                Vec2::zero()
            };

            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if attack_data.angle < 15.0
                    && entities_have_line_of_sight(
                        self.pos,
                        self.body,
                        self.scale,
                        tgt_data.pos,
                        tgt_data.body,
                        tgt_data.scale,
                        read_data,
                    )
                {
                    if agent.combat_state.timers
                        [ActionStateTimers::TimerHandleQuadLowRanged as usize]
                        > 5.0
                    {
                        agent.combat_state.timers
                            [ActionStateTimers::TimerHandleQuadLowRanged as usize] = 0.0;
                    } else if agent.combat_state.timers
                        [ActionStateTimers::TimerHandleQuadLowRanged as usize]
                        > 2.5
                    {
                        controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(1.75 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * speed;
                        agent.combat_state.timers
                            [ActionStateTimers::TimerHandleQuadLowRanged as usize] +=
                            read_data.dt.0;
                    } else {
                        controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.25 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * speed;
                        agent.combat_state.timers
                            [ActionStateTimers::TimerHandleQuadLowRanged as usize] +=
                            read_data.dt.0;
                    }
                    controller.push_basic_input(InputKind::Secondary);
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                } else {
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            } else {
                agent.target = None;
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_tail_slap_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerTailSlap = 0,
        }

        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            if agent.combat_state.timers[ActionStateTimers::TimerTailSlap as usize] > 4.0 {
                controller.push_cancel_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerTailSlap as usize] = 0.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerTailSlap as usize] > 1.0 {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerTailSlap as usize] +=
                    read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Secondary);
                agent.combat_state.timers[ActionStateTimers::TimerTailSlap as usize] +=
                    read_data.dt.0;
            }
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y)
                * 0.1;
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_quadlow_quick_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.dist_sqrd < (3.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.dist_sqrd > (2.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 90.0
        {
            controller.push_basic_input(InputKind::Primary);
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .rotated_z(-0.47 * PI)
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_quadlow_basic_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerQuadLowBasic = 0,
        }

        if attack_data.angle < 70.0
            && attack_data.dist_sqrd < (1.3 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            if agent.combat_state.timers[ActionStateTimers::TimerQuadLowBasic as usize] > 5.0 {
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBasic as usize] = 0.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerQuadLowBasic as usize] > 2.0
            {
                controller.push_basic_input(InputKind::Secondary);
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBasic as usize] +=
                    read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBasic as usize] +=
                    read_data.dt.0;
            }
        } else {
            let path = if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
                Path::Separate
            } else {
                Path::Partial
            };
            self.path_toward_target(agent, controller, tgt_data.pos.0, read_data, path, None);
        }
    }

    pub fn handle_quadmed_jump_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.angle < 15.0
            && attack_data.dist_sqrd < (5.0 * attack_data.min_attack_dist).powi(2)
        {
            controller.push_basic_input(InputKind::Ability(0));
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if self
                .path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    None,
                )
                .is_some()
                && attack_data.angle < 15.0
                && entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
            {
                controller.push_basic_input(InputKind::Primary);
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_quadmed_basic_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerQuadMedBasic = 0,
        }

        if attack_data.angle < 90.0 && attack_data.in_min_range() {
            controller.inputs.move_dir = Vec2::zero();
            if agent.combat_state.timers[ActionStateTimers::TimerQuadMedBasic as usize] < 2.0 {
                controller.push_basic_input(InputKind::Secondary);
                agent.combat_state.timers[ActionStateTimers::TimerQuadMedBasic as usize] +=
                    read_data.dt.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerQuadMedBasic as usize] < 3.0
            {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerQuadMedBasic as usize] +=
                    read_data.dt.0;
            } else {
                agent.combat_state.timers[ActionStateTimers::TimerQuadMedBasic as usize] = 0.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_quadmed_hoof_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const HOOF_ATTACK_RANGE: f32 = 1.0;
        const HOOF_ATTACK_ANGLE: f32 = 50.0;

        if attack_data.angle < HOOF_ATTACK_ANGLE
            && attack_data.dist_sqrd
                < (HOOF_ATTACK_RANGE + self.body.map_or(0.0, |b| b.front_radius())).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
    }

    pub fn handle_quadlow_beam_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerQuadLowBeam = 0,
        }
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (2.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.dist_sqrd < (7.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 15.0
        {
            if agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] < 2.0 {
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] +=
                    read_data.dt.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] < 4.0
                && attack_data.angle < 15.0
            {
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(-0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] +=
                    read_data.dt.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] < 6.0
                && attack_data.angle < 15.0
            {
                controller.push_basic_input(InputKind::Ability(0));
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] +=
                    read_data.dt.0;
            } else {
                agent.combat_state.timers[ActionStateTimers::TimerQuadLowBeam as usize] = 0.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_organ_aura_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        _tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerOrganAura = 0,
        }

        const ORGAN_AURA_DURATION: f32 = 34.75;
        if attack_data.dist_sqrd < (7.0 * attack_data.min_attack_dist).powi(2) {
            if agent.combat_state.timers[ActionStateTimers::TimerOrganAura as usize]
                > ORGAN_AURA_DURATION
            {
                agent.combat_state.timers[ActionStateTimers::TimerOrganAura as usize] = 0.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerOrganAura as usize] < 1.0 {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerOrganAura as usize] +=
                    read_data.dt.0;
            } else {
                agent.combat_state.timers[ActionStateTimers::TimerOrganAura as usize] +=
                    read_data.dt.0;
            }
        } else {
            agent.target = None;
        }
    }

    pub fn handle_theropod_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0 && attack_data.in_min_range() {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_turret_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if entities_have_line_of_sight(
            self.pos,
            self.body,
            self.scale,
            tgt_data.pos,
            tgt_data.body,
            tgt_data.scale,
            read_data,
        ) && attack_data.angle < 15.0
        {
            controller.push_basic_input(InputKind::Primary);
        } else {
            agent.target = None;
        }
    }

    pub fn handle_fixed_turret_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        controller.inputs.look_dir = self.ori.look_dir();
        if entities_have_line_of_sight(
            self.pos,
            self.body,
            self.scale,
            tgt_data.pos,
            tgt_data.body,
            tgt_data.scale,
            read_data,
        ) && attack_data.angle < 15.0
        {
            controller.push_basic_input(InputKind::Primary);
        } else {
            agent.target = None;
        }
    }

    pub fn handle_rotating_turret_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        controller.inputs.look_dir = Dir::new(
            Quaternion::from_xyzw(self.ori.look_dir().x, self.ori.look_dir().y, 0.0, 0.0)
                .rotated_z(6.0 * read_data.dt.0)
                .into_vec3()
                .try_normalized()
                .unwrap_or_default(),
        );
        if entities_have_line_of_sight(
            self.pos,
            self.body,
            self.scale,
            tgt_data.pos,
            tgt_data.body,
            tgt_data.scale,
            read_data,
        ) {
            controller.push_basic_input(InputKind::Primary);
        } else {
            agent.target = None;
        }
    }

    pub fn handle_radial_turret_attack(&self, controller: &mut Controller) {
        controller.push_basic_input(InputKind::Primary);
    }

    pub fn handle_fiery_tornado_attack(&self, agent: &mut Agent, controller: &mut Controller) {
        enum Conditions {
            AuraEmited = 0,
        }
        if matches!(self.char_state, CharacterState::BasicAura(c) if matches!(c.stage_section, StageSection::Recover))
        {
            agent.combat_state.conditions[Conditions::AuraEmited as usize] = true;
        }
        // 1 time use of aura
        if !agent.combat_state.conditions[Conditions::AuraEmited as usize] {
            controller.push_basic_input(InputKind::Secondary);
        } else {
            // Spin
            controller.push_basic_input(InputKind::Primary);
        }
    }

    pub fn handle_mindflayer_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        _attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        enum FCounters {
            SummonThreshold = 0,
        }
        enum Timers {
            PositionTimer,
            AttackTimer1,
            AttackTimer2,
        }
        enum Conditions {
            AttackToggle1,
        }
        const SUMMON_THRESHOLD: f32 = 0.20;
        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        agent.combat_state.timers[Timers::PositionTimer as usize] += read_data.dt.0;
        agent.combat_state.timers[Timers::AttackTimer1 as usize] += read_data.dt.0;
        agent.combat_state.timers[Timers::AttackTimer2 as usize] += read_data.dt.0;
        if agent.combat_state.timers[Timers::AttackTimer1 as usize] > 10.0 {
            agent.combat_state.timers[Timers::AttackTimer1 as usize] = 0.0
        }
        agent.combat_state.conditions[Conditions::AttackToggle1 as usize] =
            agent.combat_state.timers[Timers::AttackTimer1 as usize] < 5.0;
        if matches!(self.char_state, CharacterState::Blink(c) if matches!(c.stage_section, StageSection::Recover))
        {
            agent.combat_state.timers[Timers::AttackTimer2 as usize] = 0.0
        }

        let position_timer = agent.combat_state.timers[Timers::PositionTimer as usize];
        if position_timer > 60.0 {
            agent.combat_state.timers[Timers::PositionTimer as usize] = 0.0;
        }
        let home = agent.patrol_origin.unwrap_or(self.pos.0);
        let p = match position_timer as i32 {
            0_i32..=6_i32 => 0,
            7_i32..=13_i32 => 2,
            14_i32..=20_i32 => 3,
            21_i32..=27_i32 => 1,
            28_i32..=34_i32 => 4,
            35_i32..=47_i32 => 5,
            _ => 6,
        };
        let pos = if p > 5 {
            tgt_data.pos.0
        } else if p > 3 {
            home
        } else {
            Vec3::new(
                home.x + (CARDINALS[p].x * 15) as f32,
                home.y + (CARDINALS[p].y * 15) as f32,
                home.z,
            )
        };
        if !agent.combat_state.initialized {
            // Sets counter at start of combat, using `condition` to keep track of whether
            // it was already initialized
            agent.combat_state.counters[FCounters::SummonThreshold as usize] =
                1.0 - SUMMON_THRESHOLD;
            agent.combat_state.initialized = true;
        }

        if position_timer > 55.0
            && health_fraction < agent.combat_state.counters[FCounters::SummonThreshold as usize]
        {
            // Summon Husks at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(2));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters[FCounters::SummonThreshold as usize] -=
                    SUMMON_THRESHOLD;
            }
        } else if p > 5 {
            if pos.distance_squared(self.pos.0) > 20.0_f32.powi(2) {
                // teleport chase attack mode
                controller.push_action(ControlAction::StartInput {
                    input: InputKind::Ability(0),
                    target_entity: None,
                    select_pos: Some(pos),
                });
            } else {
                controller.push_basic_input(InputKind::Ability(4))
            }
        } else if p > 4 {
            // chase attack mode
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );

            if agent.combat_state.conditions[Conditions::AttackToggle1 as usize] {
                controller.push_basic_input(InputKind::Primary);
            } else {
                controller.push_basic_input(InputKind::Ability(1))
            }
        } else {
            // positioned attack mode
            if pos.distance_squared(self.pos.0) > 5.0_f32.powi(2) {
                controller.push_action(ControlAction::StartInput {
                    input: InputKind::Ability(0),
                    target_entity: None,
                    select_pos: Some(pos),
                });
            } else if agent.combat_state.timers[Timers::AttackTimer2 as usize] < 4.0 {
                controller.push_basic_input(InputKind::Secondary);
            } else {
                controller.push_basic_input(InputKind::Ability(3))
            }
        }
    }

    pub fn handle_forgemaster_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MELEE_RANGE: f32 = 6.0;
        const MID_RANGE: f32 = 25.0;
        const SUMMON_THRESHOLD: f32 = 0.2;

        enum FCounters {
            SummonThreshold = 0,
        }
        enum Timers {
            AttackRand = 0,
        }
        if agent.combat_state.timers[Timers::AttackRand as usize] > 10.0 {
            agent.combat_state.timers[Timers::AttackRand as usize] = 0.0;
        }

        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        let home = agent.patrol_origin.unwrap_or(self.pos.0);
        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Teleport back to home position if we're too far from our home position but in
        // range of the blink ability
        if (5f32.powi(2)..100f32.powi(2)).contains(&home.distance_squared(self.pos.0)) {
            controller.push_action(ControlAction::StartInput {
                input: InputKind::Ability(5),
                target_entity: None,
                select_pos: Some(home),
            });
        } else if !agent.combat_state.initialized {
            // Sets counter at start of combat, using `condition` to keep track of whether
            // it was already initialized
            agent.combat_state.counters[FCounters::SummonThreshold as usize] =
                1.0 - SUMMON_THRESHOLD;
            agent.combat_state.initialized = true;
        } else if health_fraction < agent.combat_state.counters[FCounters::SummonThreshold as usize]
        {
            // Summon IronDwarfs at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(0));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters[FCounters::SummonThreshold as usize] -=
                    SUMMON_THRESHOLD;
            }
        } else {
            // If target is in melee range use flamecrush and lawawave
            if attack_data.dist_sqrd < MELEE_RANGE.powi(2) {
                if agent.combat_state.timers[Timers::AttackRand as usize] < 3.5 {
                    // flamecrush
                    controller.push_basic_input(InputKind::Secondary);
                } else {
                    // lavawave
                    controller.push_basic_input(InputKind::Ability(3));
                }
                // If target is in mid range use lavawave, flamethrower and
                // groundislava1
            } else if attack_data.dist_sqrd < MID_RANGE.powi(2) && line_of_sight_with_target() {
                if agent.combat_state.timers[Timers::AttackRand as usize] > 6.5 {
                    controller.push_basic_input(InputKind::Ability(1));
                } else if agent.combat_state.timers[Timers::AttackRand as usize] > 3.5 {
                    // lavawave
                    controller.push_basic_input(InputKind::Ability(3));
                } else if agent.combat_state.timers[Timers::AttackRand as usize] > 2.5 {
                    // lavamortar
                    controller.push_basic_input(InputKind::Primary);
                } else {
                    // flamethrower
                    controller.push_basic_input(InputKind::Ability(2));
                }
                // If target is beyond mid range use lavamortar and
                // groundislava2
            } else if attack_data.dist_sqrd > MID_RANGE.powi(2) {
                if agent.combat_state.timers[Timers::AttackRand as usize] > 6.5 {
                    controller.push_basic_input(InputKind::Ability(4));
                } else {
                    // lavamortar
                    controller.push_basic_input(InputKind::Primary);
                }
            }
            agent.combat_state.timers[Timers::AttackRand as usize] += read_data.dt.0;
        }
        self.path_toward_target(agent, controller, home, read_data, Path::Full, None);
    }

    pub fn handle_flamekeeper_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MELEE_RANGE: f32 = 6.0;
        const MID_RANGE: f32 = 25.0;
        const SUMMON_THRESHOLD: f32 = 0.2;

        enum FCounters {
            SummonThreshold = 0,
        }
        enum Timers {
            AttackRand = 0,
        }
        if agent.combat_state.timers[Timers::AttackRand as usize] > 5.0 {
            agent.combat_state.timers[Timers::AttackRand as usize] = 0.0;
        }

        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already initialized
        if !agent.combat_state.initialized {
            agent.combat_state.counters[FCounters::SummonThreshold as usize] =
                1.0 - SUMMON_THRESHOLD;
            agent.combat_state.initialized = true;
        } else if health_fraction < agent.combat_state.counters[FCounters::SummonThreshold as usize]
        {
            // Summon Flamethrowers at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(0));
            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters[FCounters::SummonThreshold as usize] -=
                    SUMMON_THRESHOLD;
            }
        } else {
            // If target is in melee range use flamecrush
            if attack_data.dist_sqrd < MELEE_RANGE.powi(2) {
                if agent.combat_state.timers[Timers::AttackRand as usize] < 3.5 {
                    // flamecrush
                    controller.push_basic_input(InputKind::Secondary);
                } else {
                    // lavawave
                    controller.push_basic_input(InputKind::Ability(2));
                }
                // If target is in mid range use mines, lavawave, flamethrower
            } else if attack_data.dist_sqrd < MID_RANGE.powi(2) && line_of_sight_with_target() {
                if agent.combat_state.timers[Timers::AttackRand as usize] > 3.5 {
                    // lavawave
                    controller.push_basic_input(InputKind::Ability(2));
                } else if agent.combat_state.timers[Timers::AttackRand as usize] > 2.5 {
                    // mines
                    controller.push_basic_input(InputKind::Ability(3));
                } else {
                    // flamethrower
                    controller.push_basic_input(InputKind::Ability(1));
                }
                // If target is beyond mid range use lavamortar
            } else if attack_data.dist_sqrd > MID_RANGE.powi(2) {
                // lavamortar
                controller.push_basic_input(InputKind::Primary);
            }
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
            agent.combat_state.timers[Timers::AttackRand as usize] += read_data.dt.0;
        }
    }

    pub fn handle_birdlarge_fire_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        const PHOENIX_HEAL_THRESHOLD: f32 = 0.20;

        enum Conditions {
            Healed = 0,
        }
        enum ActionStateTimers {
            AttackTimer1,
            AttackTimer2,
            WaterTimer,
        }

        let attack_timer_1 =
            if agent.combat_state.timers[ActionStateTimers::AttackTimer1 as usize] < 2.0 {
                0
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer1 as usize] < 4.0 {
                1
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer1 as usize] < 6.0 {
                2
            } else {
                3
            };
        agent.combat_state.timers[ActionStateTimers::AttackTimer1 as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer1 as usize] > 8.0 {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer1 as usize] = 0.0;
        }
        let (attack_timer_2, speed) =
            if agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] < 3.0 {
                // fly high
                (0, 2.0)
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] < 6.0 {
                // attack_mid_1
                (1, 2.0)
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] < 9.0 {
                // fly high
                (0, 3.0)
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] < 16.0 {
                // attack_mid_2
                (2, 1.0)
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] < 20.0 {
                // fly low
                (5, 20.0)
            } else {
                // attack_close
                (3, 1.0)
            };
        agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] > 28.0 {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer2 as usize] = 0.0;
        }
        // Fly to target
        let dir_to_target = ((tgt_data.pos.0 + Vec3::unit_z() * 1.5) - self.pos.0)
            .try_normalized()
            .unwrap_or_else(Vec3::zero);
        controller.inputs.move_dir = dir_to_target.xy() * speed;

        // Always fly! If the floor can't touch you, it can't hurt you...
        controller.push_basic_input(InputKind::Fly);
        // Flee from the ground! The internet told me it was lava!
        // If on the ground, jump with every last ounce of energy, holding onto
        // all that is dear in life and straining for the wide open skies.

        // Don't stay in water
        if matches!(self.physics_state.in_fluid, Some(Fluid::Liquid { .. })) {
            agent.combat_state.timers[ActionStateTimers::WaterTimer as usize] = 2.0;
        };
        if agent.combat_state.timers[ActionStateTimers::WaterTimer as usize] > 0.0 {
            agent.combat_state.timers[ActionStateTimers::WaterTimer as usize] -= read_data.dt.0;
            if agent.combat_state.timers[ActionStateTimers::WaterTimer as usize] > 1.0 {
                controller.inputs.move_z = 1.0
            } else {
                // heat laser
                controller.push_basic_input(InputKind::Ability(3))
            }
        } else if self.physics_state.on_ground.is_some() {
            controller.push_basic_input(InputKind::Jump);
        } else {
            // Use a proportional controller with a coefficient of 1.0 to
            // maintain altidude at the the provided set point
            let mut maintain_altitude = |set_point| {
                let alt = read_data
                    .terrain
                    .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 7.0))
                    .until(Block::is_solid)
                    .cast()
                    .0;
                let error = set_point - alt;
                controller.inputs.move_z = error;
            };
            // heal once - from_the_ashes
            let health_fraction = self.health.map_or(0.5, |h| h.fraction());
            if matches!(self.char_state, CharacterState::SelfBuff(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.conditions[Conditions::Healed as usize] = true;
            }
            if !agent.combat_state.conditions[Conditions::Healed as usize]
                && PHOENIX_HEAL_THRESHOLD > health_fraction
            {
                controller.push_basic_input(InputKind::Ability(4));
            } else if (tgt_data.pos.0 - self.pos.0).xy().magnitude_squared() > (35.0_f32).powi(2) {
                // heat laser
                maintain_altitude(2.0);
                controller.push_basic_input(InputKind::Ability(3))
            } else {
                match attack_timer_2 {
                    0 => maintain_altitude(3.0),
                    1 => {
                        //summontornados
                        controller.push_basic_input(InputKind::Ability(1));
                    },
                    2 => {
                        // firerain
                        controller.push_basic_input(InputKind::Ability(2));
                    },
                    3 => {
                        if attack_data.dist_sqrd < 4.0_f32.powi(2) && attack_data.angle < 150.0 {
                            // close range attack
                            match attack_timer_1 {
                                1 => {
                                    // short strike
                                    controller.push_basic_input(InputKind::Primary);
                                },
                                3 => {
                                    // long strike
                                    controller.push_basic_input(InputKind::Secondary)
                                },
                                _ => {
                                    // leg strike
                                    controller.push_basic_input(InputKind::Ability(0))
                                },
                            }
                        } else {
                            match attack_timer_1 {
                                0 | 2 => {
                                    maintain_altitude(2.0);
                                },
                                _ => {
                                    // heat laser
                                    controller.push_basic_input(InputKind::Ability(3))
                                },
                            }
                        }
                    },
                    _ => {
                        maintain_altitude(2.0);
                    },
                }
            }
        }
    }

    pub fn handle_wyvern_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            AttackTimer = 0,
        }
        // Set fly to false
        controller.push_cancel_input(InputKind::Fly);
        if attack_data.dist_sqrd > 30.0_f32.powi(2) {
            if entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            ) && attack_data.angle < 15.0
            {
                controller.push_basic_input(InputKind::Primary);
            }
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                if (self.pos.0.z - tgt_data.pos.0.z) < 35.0 {
                    controller.push_basic_input(InputKind::Fly);
                    controller.inputs.move_z = 0.2;
                }
            }
        } else if !read_data
            .terrain
            .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 2.0))
            .until(Block::is_solid)
            .cast()
            .1
            .map_or(true, |b| b.is_some())
        {
            // Do not increment the timer during this movement
            // The next stage shouldn't trigger until the entity
            // is on the ground
            controller.push_basic_input(InputKind::Fly);
            let move_dir = tgt_data.pos.0 - self.pos.0;
            controller.inputs.move_dir =
                move_dir.xy().try_normalized().unwrap_or_else(Vec2::zero) * 2.0;
            controller.inputs.move_z = move_dir.z - 0.5;
            if attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
                && attack_data.angle < 15.0
            {
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd > (3.0 * attack_data.min_attack_dist).powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else if attack_data.angle < 15.0 {
            if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 5.0 {
                // beam
                controller.push_basic_input(InputKind::Ability(1));
            } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 9.0 {
                // shockwave
                controller.push_basic_input(InputKind::Ability(0));
            } else {
                agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
            }
            // Move towards the target slowly
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                Some(0.5),
            );
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 9.0
            && attack_data.angle < 90.0
            && attack_data.in_min_range()
        {
            // Triple strike
            controller.push_basic_input(InputKind::Secondary);
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        } else {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
            // Target is behind us or the timer needs to be reset. Chase target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_birdlarge_breathe_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            TimerBirdLargeBreathe = 0,
        }

        // Set fly to false
        controller.push_cancel_input(InputKind::Fly);
        if attack_data.dist_sqrd > 30.0_f32.powi(2) {
            if rng.gen_bool(0.05)
                && entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
                && attack_data.angle < 15.0
            {
                controller.push_basic_input(InputKind::Primary);
            }
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                if (self.pos.0.z - tgt_data.pos.0.z) < 20.0 {
                    controller.push_basic_input(InputKind::Fly);
                    controller.inputs.move_z = 1.0;
                }
            }
        } else if !read_data
            .terrain
            .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 2.0))
            .until(Block::is_solid)
            .cast()
            .1
            .map_or(true, |b| b.is_some())
        {
            // Do not increment the timer during this movement
            // The next stage shouldn't trigger until the entity
            // is on the ground
            controller.push_basic_input(InputKind::Fly);
            let move_dir = tgt_data.pos.0 - self.pos.0;
            controller.inputs.move_dir =
                move_dir.xy().try_normalized().unwrap_or_else(Vec2::zero) * 2.0;
            controller.inputs.move_z = move_dir.z - 0.5;
            if rng.gen_bool(0.05)
                && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
                && attack_data.angle < 15.0
            {
                controller.push_basic_input(InputKind::Primary);
            }
        } else if rng.gen_bool(0.05)
            && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 15.0
        {
            controller.push_basic_input(InputKind::Primary);
        } else if rng.gen_bool(0.5)
            && (self.pos.0.z - tgt_data.pos.0.z) < 15.0
            && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
        {
            controller.push_basic_input(InputKind::Fly);
            controller.inputs.move_z = 1.0;
        } else if attack_data.dist_sqrd > (3.0 * attack_data.min_attack_dist).powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else if self.energy.current() > 60.0
            && agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBreathe as usize] < 3.0
            && attack_data.angle < 15.0
        {
            // Fire breath attack
            controller.push_basic_input(InputKind::Ability(0));
            // Move towards the target slowly
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                Some(0.5),
            );
            agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBreathe as usize] +=
                read_data.dt.0;
        } else if agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBreathe as usize] < 6.0
            && attack_data.angle < 90.0
            && attack_data.in_min_range()
        {
            // Triple strike
            controller.push_basic_input(InputKind::Secondary);
            agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBreathe as usize] +=
                read_data.dt.0;
        } else {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBreathe as usize] = 0.0;
            // Target is behind us or the timer needs to be reset. Chase target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_birdlarge_basic_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerBirdLargeBasic = 0,
        }

        enum ActionStateConditions {
            ConditionBirdLargeBasic = 0, /* FIXME: Not sure what this represents. This name
                                          * should be reflective of the condition... */
        }

        const BIRD_ATTACK_RANGE: f32 = 4.0;
        const BIRD_CHARGE_DISTANCE: f32 = 15.0;
        let bird_attack_distance = self.body.map_or(0.0, |b| b.max_radius()) + BIRD_ATTACK_RANGE;
        // Increase action timer
        agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBasic as usize] +=
            read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBasic as usize] > 8.0 {
            // If action timer higher than 8, make bird summon tornadoes
            controller.push_basic_input(InputKind::Secondary);
            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                // Reset timer
                agent.combat_state.timers[ActionStateTimers::TimerBirdLargeBasic as usize] = 0.0;
            }
        } else if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already in dash, keep dashing if not in recover
            controller.push_basic_input(InputKind::Ability(0));
        } else if matches!(self.char_state, CharacterState::ComboMelee2(c) if matches!(c.stage_section, StageSection::Recover))
        {
            // If already in combo keep comboing if not in recover
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd > BIRD_CHARGE_DISTANCE.powi(2) {
            // Charges at target if they are far enough away
            if attack_data.angle < 60.0 {
                controller.push_basic_input(InputKind::Ability(0));
            }
        } else if attack_data.dist_sqrd < bird_attack_distance.powi(2) {
            // Combo melee target
            controller.push_basic_input(InputKind::Primary);
            agent.combat_state.conditions
                [ActionStateConditions::ConditionBirdLargeBasic as usize] = true;
        }
        // Make bird move towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Separate,
            None,
        );
    }

    pub fn handle_arthropod_ranged_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerArthropodRanged = 0,
        }

        agent.combat_state.timers[ActionStateTimers::TimerArthropodRanged as usize] +=
            read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::TimerArthropodRanged as usize] > 6.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Secondary);
            // Reset timer
            if matches!(self.char_state,
            CharacterState::SpriteSummon(sprite_summon::Data { stage_section, .. })
            | CharacterState::SelfBuff(self_buff::Data { stage_section, .. })
            if matches!(stage_section, StageSection::Recover))
            {
                agent.combat_state.timers[ActionStateTimers::TimerArthropodRanged as usize] = 0.0;
            }
        } else if attack_data.dist_sqrd < (2.5 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 90.0
        {
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y)
                // Slow down if very close to the target
                * if attack_data.in_min_range() { 0.3 } else { 1.0 };
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if attack_data.angle < 15.0
                    && entities_have_line_of_sight(
                        self.pos,
                        self.body,
                        self.scale,
                        tgt_data.pos,
                        tgt_data.body,
                        tgt_data.scale,
                        read_data,
                    )
                {
                    if agent.combat_state.timers[ActionStateTimers::TimerArthropodRanged as usize]
                        > 5.0
                    {
                        agent.combat_state.timers
                            [ActionStateTimers::TimerArthropodRanged as usize] = 0.0;
                    } else if agent.combat_state.timers
                        [ActionStateTimers::TimerArthropodRanged as usize]
                        > 2.5
                    {
                        controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(1.75 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * speed;
                        agent.combat_state.timers
                            [ActionStateTimers::TimerArthropodRanged as usize] += read_data.dt.0;
                    } else {
                        controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.25 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * speed;
                        agent.combat_state.timers
                            [ActionStateTimers::TimerArthropodRanged as usize] += read_data.dt.0;
                    }
                    controller.push_basic_input(InputKind::Ability(0));
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                } else {
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            } else {
                agent.target = None;
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_arthropod_ambush_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            TimersArthropodAmbush = 0,
        }

        agent.combat_state.timers[ActionStateTimers::TimersArthropodAmbush as usize] +=
            read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::TimersArthropodAmbush as usize] > 12.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Secondary);
            // Reset timer
            if matches!(self.char_state,
            CharacterState::SpriteSummon(sprite_summon::Data { stage_section, .. })
            | CharacterState::SelfBuff(self_buff::Data { stage_section, .. })
            if matches!(stage_section, StageSection::Recover))
            {
                agent.combat_state.timers[ActionStateTimers::TimersArthropodAmbush as usize] = 0.0;
            }
        } else if attack_data.angle < 90.0
            && attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
        } else if rng.gen_bool(0.01)
            && attack_data.angle < 60.0
            && attack_data.dist_sqrd > (2.0 * attack_data.min_attack_dist).powi(2)
        {
            controller.push_basic_input(InputKind::Ability(0));
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_arthropod_melee_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimersArthropodMelee = 0,
        }
        agent.combat_state.timers[ActionStateTimers::TimersArthropodMelee as usize] +=
            read_data.dt.0;
        if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already charging, keep charging if not in recover
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.dist_sqrd > (2.5 * attack_data.min_attack_dist).powi(2) {
            // Charges at target if they are far enough away
            if attack_data.angle < 60.0 {
                controller.push_basic_input(InputKind::Secondary);
            }
        } else if attack_data.angle < 90.0
            && attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_minotaur_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MINOTAUR_FRENZY_THRESHOLD: f32 = 0.5;
        const MINOTAUR_ATTACK_RANGE: f32 = 5.0;
        const MINOTAUR_CHARGE_DISTANCE: f32 = 15.0;

        enum ActionStateFCounters {
            FCounterMinotaurAttack = 0,
        }

        enum ActionStateConditions {
            ConditionJustCrippledOrCleaved = 0,
        }

        enum Conditions {
            AttackToggle,
        }

        enum Timers {
            CheeseTimer,
            CanSeeTarget,
            Reposition,
        }

        let minotaur_attack_distance =
            self.body.map_or(0.0, |b| b.max_radius()) + MINOTAUR_ATTACK_RANGE;
        let health_fraction = self.health.map_or(1.0, |h| h.fraction());
        let home = agent.patrol_origin.unwrap_or(self.pos.0);
        let center = Vec2::new(home.x + 50.0, home.y + 75.0);
        let cheesed_from_above = tgt_data.pos.0.z > self.pos.0.z + 4.0;
        let center_cheesed = (center - self.pos.0.xy()).magnitude_squared() < 16.0_f32.powi(2);
        let pillar_cheesed = (center - tgt_data.pos.0.xy()).magnitude_squared() < 16.0_f32.powi(2);
        let cheesed = (pillar_cheesed || center_cheesed)
            && agent.combat_state.timers[Timers::CheeseTimer as usize] > 4.0;
        agent.combat_state.timers[Timers::CheeseTimer as usize] += read_data.dt.0;
        agent.combat_state.timers[Timers::CanSeeTarget as usize] += read_data.dt.0;
        agent.combat_state.timers[Timers::Reposition as usize] += read_data.dt.0;
        if agent.combat_state.timers[Timers::Reposition as usize] > 20.0 {
            agent.combat_state.timers[Timers::Reposition as usize] = 0.0;
        }
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        if !line_of_sight_with_target() {
            agent.combat_state.timers[Timers::CanSeeTarget as usize] = 0.0;
        };
        let remote_spikes_action = || ControlAction::StartInput {
            input: InputKind::Ability(3),
            target_entity: None,
            select_pos: Some(tgt_data.pos.0),
        };
        // Sets action counter at start of combat
        if agent.combat_state.counters[ActionStateFCounters::FCounterMinotaurAttack as usize]
            < MINOTAUR_FRENZY_THRESHOLD
            && health_fraction > MINOTAUR_FRENZY_THRESHOLD
        {
            agent.combat_state.counters[ActionStateFCounters::FCounterMinotaurAttack as usize] =
                MINOTAUR_FRENZY_THRESHOLD;
        }
        if matches!(self.char_state, CharacterState::SpriteSummon(c) if matches!(c.stage_section, StageSection::Recover))
        {
            agent.combat_state.conditions[Conditions::AttackToggle as usize] = true;
        }
        if matches!(self.char_state, CharacterState::BasicRanged(c) if matches!(c.stage_section, StageSection::Recover))
        {
            agent.combat_state.conditions[Conditions::AttackToggle as usize] = false;
            if agent.combat_state.timers[Timers::CheeseTimer as usize] > 10.0 {
                agent.combat_state.timers[Timers::CheeseTimer as usize] = 0.0;
            }
        }
        // when cheesed, throw axes and summon sprites.
        if cheesed_from_above || cheesed {
            if agent.combat_state.conditions[Conditions::AttackToggle as usize] {
                controller.push_basic_input(InputKind::Ability(2));
            } else {
                controller.push_action(remote_spikes_action());
            }
            //
            if center_cheesed {
                // when cheesed around the center pillar, try to reposition
                let dir_index = match agent.combat_state.timers[Timers::Reposition as usize] as i32
                {
                    0_i32..5_i32 => 0,
                    5_i32..10_i32 => 1,
                    10_i32..15_i32 => 2,
                    _ => 3,
                };
                let goto = Vec3::new(
                    center.x + (CARDINALS[dir_index].x * 25) as f32,
                    center.y + (CARDINALS[dir_index].y * 25) as f32,
                    tgt_data.pos.0.z,
                );
                self.path_toward_target(
                    agent,
                    controller,
                    goto,
                    read_data,
                    Path::Partial,
                    (attack_data.dist_sqrd
                        < (attack_data.min_attack_dist + MINOTAUR_ATTACK_RANGE / 3.0).powi(2))
                    .then_some(0.1),
                );
            }
        } else if health_fraction
            < agent.combat_state.counters[ActionStateFCounters::FCounterMinotaurAttack as usize]
        {
            // Makes minotaur buff itself with frenzy
            controller.push_basic_input(InputKind::Ability(1));
            if matches!(self.char_state, CharacterState::SelfBuff(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterMinotaurAttack as usize] = 0.0;
            }
        } else if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already charging, keep charging if not in recover
            controller.push_basic_input(InputKind::Ability(0));
        } else if matches!(self.char_state, CharacterState::ChargedMelee(c) if matches!(c.stage_section, StageSection::Charge) && c.timer < c.static_data.charge_duration)
        {
            // If already charging a melee attack, keep charging it if charging
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd > MINOTAUR_CHARGE_DISTANCE.powi(2) {
            // Charges at target if they are far enough away
            if attack_data.angle < 60.0 {
                controller.push_basic_input(InputKind::Ability(0));
            }
        } else if attack_data.dist_sqrd < minotaur_attack_distance.powi(2) {
            if agent.combat_state.conditions
                [ActionStateConditions::ConditionJustCrippledOrCleaved as usize]
                && !self.char_state.is_attack()
            {
                // Cripple target if not just used cripple
                controller.push_basic_input(InputKind::Secondary);
                agent.combat_state.conditions
                    [ActionStateConditions::ConditionJustCrippledOrCleaved as usize] = false;
            } else if !self.char_state.is_attack() {
                // Cleave target if not just used cleave
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.conditions
                    [ActionStateConditions::ConditionJustCrippledOrCleaved as usize] = true;
            }
        }
        // Chase target, when target is above, retreat to chamber
        if cheesed_from_above {
            self.path_toward_target(agent, controller, home, read_data, Path::Full, None);
        // delay chasing to counter wall cheese
        } else if agent.combat_state.timers[Timers::CanSeeTarget as usize] > 2.0
        // always chase when in hallway to boss chamber
        || (3.0..18.0).contains(&(self.pos.0.y - home.y))
        {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                (attack_data.dist_sqrd
                    < (attack_data.min_attack_dist + MINOTAUR_ATTACK_RANGE / 3.0).powi(2))
                .then_some(0.1),
            );
        }
    }

    pub fn handle_cyclops_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        // Primary
        const CYCLOPS_MELEE_RANGE: f32 = 9.0;
        // Secondary
        const CYCLOPS_FIRE_RANGE: f32 = 30.0;
        // Ability(1)
        const CYCLOPS_CHARGE_RANGE: f32 = 18.0;
        // Ability(0) - Ablity (2)
        const SHOCKWAVE_THRESHOLD: f32 = 0.6;

        enum FCounters {
            ShockwaveThreshold = 0,
        }
        enum Timers {
            AttackChange = 0,
        }

        if agent.combat_state.timers[Timers::AttackChange as usize] > 2.5 {
            agent.combat_state.timers[Timers::AttackChange as usize] = 0.0;
        }

        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already initialized
        if !agent.combat_state.initialized {
            agent.combat_state.counters[FCounters::ShockwaveThreshold as usize] =
                1.0 - SHOCKWAVE_THRESHOLD;
            agent.combat_state.initialized = true;
        } else if health_fraction
            < agent.combat_state.counters[FCounters::ShockwaveThreshold as usize]
        {
            // Scream when threshold is reached
            controller.push_basic_input(InputKind::Ability(2));

            if matches!(self.char_state, CharacterState::SelfBuff(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters[FCounters::ShockwaveThreshold as usize] -=
                    SHOCKWAVE_THRESHOLD;
            }
        } else if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already AOEing, keep AOEing if not in recover
            controller.push_basic_input(InputKind::Ability(0));
        } else if attack_data.dist_sqrd > CYCLOPS_FIRE_RANGE.powi(2) {
            // Chase
            controller.push_basic_input(InputKind::Ability(1));
        } else if attack_data.dist_sqrd > CYCLOPS_CHARGE_RANGE.powi(2) {
            // Shoot after target if they attempt to "flee"
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.dist_sqrd < CYCLOPS_MELEE_RANGE.powi(2) {
            if attack_data.angle < 60.0 {
                // Melee target if close enough and within angle
                controller.push_basic_input(InputKind::Primary);
            } else if attack_data.angle > 60.0 {
                // Scream if target exceeds angle but is close enough
                controller.push_basic_input(InputKind::Ability(0));
            }
        }

        // Always path towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Partial,
            (attack_data.dist_sqrd
                < (attack_data.min_attack_dist + CYCLOPS_MELEE_RANGE / 2.0).powi(2))
            .then_some(0.1),
        );
    }

    pub fn handle_dullahan_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        // Primary (12 Default / Melee)
        const MELEE_RANGE: f32 = 9.0;
        // Secondary (30 Default / Range)
        const LONG_RANGE: f32 = 30.0;
        // Ability(0) (0.1 aka 10% Default / AOE)
        const HP_THRESHOLD: f32 = 0.1;
        // Ability(1) (18 Default / Dash)
        const MID_RANGE: f32 = 18.0;

        enum FCounters {
            HealthThreshold = 0,
        }
        enum Timers {
            AttackChange = 0,
        }
        if agent.combat_state.timers[Timers::AttackChange as usize] > 2.5 {
            agent.combat_state.timers[Timers::AttackChange as usize] = 0.0;
        }

        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already initialized
        if !agent.combat_state.initialized {
            agent.combat_state.counters[FCounters::HealthThreshold as usize] = 1.0 - HP_THRESHOLD;
            agent.combat_state.initialized = true;
        } else if health_fraction < agent.combat_state.counters[FCounters::HealthThreshold as usize]
        {
            // InputKind when threshold is reached (Default is Ability(0))
            controller.push_basic_input(InputKind::Ability(0));

            if matches!(
                self.char_state.ability_info().map(|ai| ai.input),
                Some(InputKind::Ability(0))
            ) && matches!(self.char_state.stage_section(), Some(StageSection::Recover))
            {
                agent.combat_state.counters[FCounters::HealthThreshold as usize] -= HP_THRESHOLD;
            }
        } else if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already InputKind, keep InputKind if not in recover (Default is Shockwave)
            controller.push_basic_input(InputKind::Ability(0));
        } else if attack_data.dist_sqrd > LONG_RANGE.powi(2) {
            // InputKind after target if they attempt to "flee" (>LONG)
            controller.push_basic_input(InputKind::Ability(1));
        } else if attack_data.dist_sqrd > MID_RANGE.powi(2) {
            // InputKind after target if they attempt to "flee" (MID-LONG)
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.dist_sqrd < MELEE_RANGE.powi(2) {
            if attack_data.angle < 60.0 {
                // InputKind target if close enough and within angle (<MELEE)
                controller.push_basic_input(InputKind::Primary);
            } else if attack_data.angle > 60.0 {
                // InputKind if target exceeds angle but is close enough (FLANK/STRAFE)
                controller.push_basic_input(InputKind::Ability(0));
            }
        }

        // Path to target if too far away
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Full,
            (attack_data.dist_sqrd < (attack_data.min_attack_dist + MELEE_RANGE / 2.0).powi(2))
                .then_some(0.1),
        );
    }

    pub fn handle_grave_warden_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const GOLEM_MELEE_RANGE: f32 = 4.0;
        const GOLEM_LASER_RANGE: f32 = 30.0;
        const GOLEM_LONG_RANGE: f32 = 50.0;
        const GOLEM_TARGET_SPEED: f32 = 8.0;

        enum ActionStateFCounters {
            FCounterGlayGolemAttack = 0,
        }

        let golem_melee_range = self.body.map_or(0.0, |b| b.max_radius()) + GOLEM_MELEE_RANGE;
        // Fraction of health, used for activation of shockwave
        // If golem don't have health for some reason, assume it's full
        let health_fraction = self.health.map_or(1.0, |h| h.fraction());
        // Magnitude squared of cross product of target velocity with golem orientation
        let target_speed_cross_sqd = agent
            .target
            .as_ref()
            .map(|t| t.target)
            .and_then(|e| read_data.velocities.get(e))
            .map_or(0.0, |v| v.0.cross(self.ori.look_vec()).magnitude_squared());
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        if attack_data.dist_sqrd < golem_melee_range.powi(2) {
            if agent.combat_state.counters[ActionStateFCounters::FCounterGlayGolemAttack as usize]
                < 7.5
            {
                // If target is close, whack them
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterGlayGolemAttack as usize] += read_data.dt.0;
            } else {
                // If whacked for too long, nuke them
                controller.push_basic_input(InputKind::Ability(1));
                if matches!(self.char_state, CharacterState::BasicRanged(c) if matches!(c.stage_section, StageSection::Recover))
                {
                    agent.combat_state.counters
                        [ActionStateFCounters::FCounterGlayGolemAttack as usize] = 0.0;
                }
            }
        } else if attack_data.dist_sqrd < GOLEM_LASER_RANGE.powi(2) {
            if matches!(self.char_state, CharacterState::BasicBeam(c) if c.timer < Duration::from_secs(5))
                || target_speed_cross_sqd < GOLEM_TARGET_SPEED.powi(2)
                    && line_of_sight_with_target()
                    && attack_data.angle < 45.0
            {
                // If target in range threshold and haven't been lasering for more than 5
                // seconds already or if target is moving slow-ish, laser them
                controller.push_basic_input(InputKind::Secondary);
            } else if health_fraction < 0.7 {
                // Else target moving too fast for laser, shockwave time.
                // But only if damaged enough
                controller.push_basic_input(InputKind::Ability(0));
            }
        } else if attack_data.dist_sqrd < GOLEM_LONG_RANGE.powi(2) {
            if target_speed_cross_sqd < GOLEM_TARGET_SPEED.powi(2) && line_of_sight_with_target() {
                // If target is far-ish and moving slow-ish, rocket them
                controller.push_basic_input(InputKind::Ability(1));
            } else if health_fraction < 0.7 {
                // Else target moving too fast for laser, shockwave time.
                // But only if damaged enough
                controller.push_basic_input(InputKind::Ability(0));
            }
        }

        // Make grave warden move towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Separate,
            (attack_data.dist_sqrd
                < (attack_data.min_attack_dist + GOLEM_MELEE_RANGE / 1.5).powi(2))
            .then_some(0.1),
        );
    }

    pub fn handle_tidal_warrior_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const SCUTTLE_RANGE: f32 = 40.0;
        const BUBBLE_RANGE: f32 = 20.0;
        const MINION_SUMMON_THRESHOLD: f32 = 0.20;

        enum ActionStateConditions {
            ConditionCounterInitialized = 0,
        }

        enum ActionStateFCounters {
            FCounterMinionSummonThreshold = 0,
        }

        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        let home = agent.patrol_origin.unwrap_or(self.pos.0.round());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already initialized
        if !agent.combat_state.conditions
            [ActionStateConditions::ConditionCounterInitialized as usize]
        {
            agent.combat_state.counters
                [ActionStateFCounters::FCounterMinionSummonThreshold as usize] =
                1.0 - MINION_SUMMON_THRESHOLD;
            agent.combat_state.conditions
                [ActionStateConditions::ConditionCounterInitialized as usize] = true;
        }

        if agent.combat_state.counters[ActionStateFCounters::FCounterMinionSummonThreshold as usize]
            > health_fraction
        {
            // Summon minions at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(1));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterMinionSummonThreshold as usize] -=
                    MINION_SUMMON_THRESHOLD;
            }
        } else if attack_data.dist_sqrd < SCUTTLE_RANGE.powi(2) {
            if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
            {
                // Keep scuttling if already in dash melee and not in recover
                controller.push_basic_input(InputKind::Secondary);
            } else if attack_data.dist_sqrd < BUBBLE_RANGE.powi(2) {
                if matches!(self.char_state, CharacterState::BasicBeam(c) if !matches!(c.stage_section, StageSection::Recover) && c.timer < Duration::from_secs(10))
                {
                    // Keep shooting bubbles at them if already in basic beam and not in recover and
                    // have not been bubbling too long
                    controller.push_basic_input(InputKind::Ability(0));
                } else if attack_data.in_min_range() && attack_data.angle < 60.0 {
                    // Pincer them if they're in range and angle
                    controller.push_basic_input(InputKind::Primary);
                } else if attack_data.angle < 30.0 && line_of_sight_with_target() {
                    // Start bubbling them if not close enough to do something else and in angle and
                    // can see target
                    controller.push_basic_input(InputKind::Ability(0));
                }
            } else if attack_data.angle < 90.0 && line_of_sight_with_target() {
                // Start scuttling if not close enough to do something else and in angle and can
                // see target
                controller.push_basic_input(InputKind::Secondary);
            }
        }
        let path = if tgt_data.pos.0.z < self.pos.0.z {
            home
        } else {
            tgt_data.pos.0
        };
        // attempt to path towards target, move away from exiit  if target is cheesing
        // from below
        self.path_toward_target(agent, controller, path, read_data, Path::Partial, None);
    }

    pub fn handle_yeti_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const ICE_SPIKES_RANGE: f32 = 15.0;
        const ICE_BREATH_RANGE: f32 = 10.0;
        const ICE_BREATH_TIMER: f32 = 10.0;
        const SNOWBALL_MAX_RANGE: f32 = 50.0;

        enum ActionStateFCounters {
            FCounterYetiAttack = 0,
        }

        agent.combat_state.counters[ActionStateFCounters::FCounterYetiAttack as usize] +=
            read_data.dt.0;

        if attack_data.dist_sqrd < ICE_BREATH_RANGE.powi(2) {
            if matches!(self.char_state, CharacterState::BasicBeam(c) if c.timer < Duration::from_secs(2))
            {
                // Keep using ice breath for 2 second
                controller.push_basic_input(InputKind::Ability(0));
            } else if agent.combat_state.counters[ActionStateFCounters::FCounterYetiAttack as usize]
                > ICE_BREATH_TIMER
            {
                // Use ice breath if timer has gone for long enough
                controller.push_basic_input(InputKind::Ability(0));

                if matches!(self.char_state, CharacterState::BasicBeam(_)) {
                    // Resets action counter when using beam
                    agent.combat_state.counters
                        [ActionStateFCounters::FCounterYetiAttack as usize] = 0.0;
                }
            } else if attack_data.in_min_range() {
                // Basic attack if on top of them
                controller.push_basic_input(InputKind::Primary);
            } else {
                // Use ice spikes if too far for other abilities
                controller.push_basic_input(InputKind::Secondary);
            }
        } else if attack_data.dist_sqrd < ICE_SPIKES_RANGE.powi(2) && attack_data.angle < 60.0 {
            // Use ice spikes if in range
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.dist_sqrd < SNOWBALL_MAX_RANGE.powi(2) && attack_data.angle < 60.0 {
            // Otherwise, chuck all the snowballs
            controller.push_basic_input(InputKind::Ability(1));
        }

        // Always attempt to path towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Partial,
            attack_data.in_min_range().then_some(0.1),
        );
    }

    pub fn handle_rocksnapper_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const LEAP_TIMER: f32 = 3.0;
        const DASH_TIMER: f32 = 5.0;
        const LEAP_RANGE: f32 = 20.0;
        const MELEE_RANGE: f32 = 5.0;

        enum ActionStateTimers {
            TimerRocksnapperDash = 0,
            TimerRocksnapperLeap = 1,
        }
        agent.combat_state.timers[ActionStateTimers::TimerRocksnapperDash as usize] +=
            read_data.dt.0;
        agent.combat_state.timers[ActionStateTimers::TimerRocksnapperLeap as usize] +=
            read_data.dt.0;

        if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already dashing, keep dashing if not in recover stage
            controller.push_basic_input(InputKind::Secondary);
        } else if agent.combat_state.timers[ActionStateTimers::TimerRocksnapperDash as usize]
            > DASH_TIMER
        {
            // Use dash if timer has gone for long enough
            controller.push_basic_input(InputKind::Secondary);

            if matches!(self.char_state, CharacterState::DashMelee(_)) {
                // Resets action counter when using dash
                agent.combat_state.timers[ActionStateTimers::TimerRocksnapperDash as usize] = 0.0;
            }
        } else if attack_data.dist_sqrd < LEAP_RANGE.powi(2) && attack_data.angle < 90.0 {
            if agent.combat_state.timers[ActionStateTimers::TimerRocksnapperLeap as usize]
                > LEAP_TIMER
            {
                // Use shockwave if timer has gone for long enough
                controller.push_basic_input(InputKind::Ability(0));

                if matches!(self.char_state, CharacterState::LeapShockwave(_)) {
                    // Resets action timer when using leap shockwave
                    agent.combat_state.timers[ActionStateTimers::TimerRocksnapperLeap as usize] =
                        0.0;
                }
            } else if attack_data.dist_sqrd < MELEE_RANGE.powi(2) {
                // Basic attack if in melee range
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd < MELEE_RANGE.powi(2) && attack_data.angle < 135.0 {
            // Basic attack if in melee range
            controller.push_basic_input(InputKind::Primary);
        }

        // Always attempt to path towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Partial,
            None,
        );
    }

    pub fn handle_roshwalr_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const SLOW_CHARGE_RANGE: f32 = 12.5;
        const SHOCKWAVE_RANGE: f32 = 12.5;
        const SHOCKWAVE_TIMER: f32 = 15.0;
        const MELEE_RANGE: f32 = 4.0;

        enum ActionStateFCounters {
            FCounterRoshwalrAttack = 0,
        }

        agent.combat_state.counters[ActionStateFCounters::FCounterRoshwalrAttack as usize] +=
            read_data.dt.0;
        if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already charging, keep charging if not in recover
            controller.push_basic_input(InputKind::Ability(0));
        } else if attack_data.dist_sqrd < SHOCKWAVE_RANGE.powi(2) && attack_data.angle < 270.0 {
            if agent.combat_state.counters[ActionStateFCounters::FCounterRoshwalrAttack as usize]
                > SHOCKWAVE_TIMER
            {
                // Use shockwave if timer has gone for long enough
                controller.push_basic_input(InputKind::Ability(0));

                if matches!(self.char_state, CharacterState::Shockwave(_)) {
                    // Resets action counter when using shockwave
                    agent.combat_state.counters
                        [ActionStateFCounters::FCounterRoshwalrAttack as usize] = 0.0;
                }
            } else if attack_data.dist_sqrd < MELEE_RANGE.powi(2) && attack_data.angle < 135.0 {
                // Basic attack if in melee range
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd > SLOW_CHARGE_RANGE.powi(2) {
            // Use slow charge if outside the range
            controller.push_basic_input(InputKind::Secondary);
        }

        // Always attempt to path towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Partial,
            None,
        );
    }

    pub fn handle_harvester_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        // === reference ===
        // Inputs:
        //   Primary: scythe
        //   Secondary: firebreath
        //   Auxiliary
        //     0: explosivepumpkin
        //     1: ensaringvines_sparse
        //     2: ensaringvines_dense

        // === setup ===

        // --- static ---
        // behaviour parameters
        const FIRST_VINE_CREATION_THRESHOLD: f32 = 0.60;
        const SECOND_VINE_CREATION_THRESHOLD: f32 = 0.30;
        const PATH_RANGE_FACTOR: f32 = 0.4; // get comfortably in range, but give player room to breathe
        const SCYTHE_RANGE_FACTOR: f32 = 0.75; // start attack while suitably in range
        const SCYTHE_AIM_FACTOR: f32 = 0.7;
        const FIREBREATH_RANGE_FACTOR: f32 = 0.7;
        const FIREBREATH_AIM_FACTOR: f32 = 0.8;
        const FIREBREATH_TIME_LIMIT: f32 = 4.0;
        const FIREBREATH_SHORT_TIME_LIMIT: f32 = 2.5; // cutoff sooner at close range
        const FIREBREATH_COOLDOWN: f32 = 3.5;
        const PUMPKIN_RANGE_FACTOR: f32 = 0.75;
        const CLOSE_MIXUP_COOLDOWN_SPAN: [f32; 2] = [1.5, 7.0]; // variation in attacks at close range
        const MID_MIXUP_COOLDOWN_SPAN: [f32; 2] = [1.5, 4.5]; //   ^                       mid
        const FAR_PUMPKIN_COOLDOWN_SPAN: [f32; 2] = [3.0, 5.0]; // allows for pathing to player between throws

        // conditions
        const HAS_SUMMONED_FIRST_VINES: usize = 0;
        const HAS_SUMMONED_SECOND_VINES: usize = 1;
        // timers
        const FIREBREATH: usize = 0;
        const MIXUP: usize = 1;
        const FAR_PUMPKIN: usize = 2;
        //counters
        const CLOSE_MIXUP_COOLDOWN: usize = 0;
        const MID_MIXUP_COOLDOWN: usize = 1;
        const FAR_PUMPKIN_COOLDOWN: usize = 2;

        // line of sight check
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        // --- dynamic ---
        // attack data
        let (scythe_range, scythe_angle) = {
            if let Some(AbilityData::BasicMelee { range, angle, .. }) =
                self.extract_ability(AbilityInput::Primary)
            {
                (range, angle)
            } else {
                (0.0, 0.0)
            }
        };
        let (firebreath_range, firebreath_angle) = {
            if let Some(AbilityData::BasicBeam { range, angle, .. }) =
                self.extract_ability(AbilityInput::Secondary)
            {
                (range, angle)
            } else {
                (0.0, 0.0)
            }
        };
        let pumpkin_speed = {
            if let Some(AbilityData::BasicRanged {
                projectile_speed, ..
            }) = self.extract_ability(AbilityInput::Auxiliary(0))
            {
                projectile_speed
            } else {
                0.0
            }
        };
        // calculated attack data
        let pumpkin_max_range =
            projectile_flat_range(pumpkin_speed, self.body.map_or(0.0, |b| b.height()));

        // character state info
        let is_using_firebreath = matches!(self.char_state, CharacterState::BasicBeam(_));
        let is_using_pumpkin = matches!(self.char_state, CharacterState::BasicRanged(_));
        let is_in_summon_recovery = matches!(self.char_state, CharacterState::SpriteSummon(data) if matches!(data.stage_section, StageSection::Recover));
        let firebreath_timer = if let CharacterState::BasicBeam(data) = self.char_state {
            data.timer
        } else {
            Default::default()
        };
        let is_using_mixup = is_using_firebreath || is_using_pumpkin;

        // initialise randomised cooldowns
        if !agent.combat_state.initialized {
            agent.combat_state.initialized = true;
            agent.combat_state.counters[CLOSE_MIXUP_COOLDOWN] =
                rng_from_span(rng, CLOSE_MIXUP_COOLDOWN_SPAN);
            agent.combat_state.counters[MID_MIXUP_COOLDOWN] =
                rng_from_span(rng, MID_MIXUP_COOLDOWN_SPAN);
            agent.combat_state.counters[FAR_PUMPKIN_COOLDOWN] =
                rng_from_span(rng, FAR_PUMPKIN_COOLDOWN_SPAN);
        }

        // === main ===

        // --- timers ---
        if is_in_summon_recovery {
            // reset all timers when done summoning
            agent.combat_state.timers[FIREBREATH] = 0.0;
            agent.combat_state.timers[MIXUP] = 0.0;
            agent.combat_state.timers[FAR_PUMPKIN] = 0.0;
        } else {
            // handle state timers
            if is_using_firebreath {
                agent.combat_state.timers[FIREBREATH] = 0.0;
            } else {
                agent.combat_state.timers[FIREBREATH] += read_data.dt.0;
            }
            if is_using_mixup {
                agent.combat_state.timers[MIXUP] = 0.0;
            } else {
                agent.combat_state.timers[MIXUP] += read_data.dt.0;
            }
            if is_using_pumpkin {
                agent.combat_state.timers[FAR_PUMPKIN] = 0.0;
            } else {
                agent.combat_state.timers[FAR_PUMPKIN] += read_data.dt.0;
            }
        }

        // --- attacks ---
        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // second vine summon
        if health_fraction < SECOND_VINE_CREATION_THRESHOLD
            && !agent.combat_state.conditions[HAS_SUMMONED_SECOND_VINES]
        {
            // use the dense vine summon
            controller.push_basic_input(InputKind::Ability(2));
            // wait till recovery before finishing
            if is_in_summon_recovery {
                agent.combat_state.conditions[HAS_SUMMONED_SECOND_VINES] = true;
            }
        }
        // first vine summon
        else if health_fraction < FIRST_VINE_CREATION_THRESHOLD
            && !agent.combat_state.conditions[HAS_SUMMONED_FIRST_VINES]
        {
            // use the sparse vine summon
            controller.push_basic_input(InputKind::Ability(1));
            // wait till recovery before finishing
            if is_in_summon_recovery {
                agent.combat_state.conditions[HAS_SUMMONED_FIRST_VINES] = true;
            }
        }
        // close range
        else if attack_data.dist_sqrd
            < (attack_data.body_dist + scythe_range * SCYTHE_RANGE_FACTOR).powi(2)
        {
            // if using firebreath, keep going under short time limit
            if is_using_firebreath
                && firebreath_timer < Duration::from_secs_f32(FIREBREATH_SHORT_TIME_LIMIT)
            {
                controller.push_basic_input(InputKind::Secondary);
            }
            // in scythe angle
            if attack_data.angle < scythe_angle * SCYTHE_AIM_FACTOR {
                // on timer, randomly mixup attacks
                if agent.combat_state.timers[MIXUP]
                    > agent.combat_state.counters[CLOSE_MIXUP_COOLDOWN]
                // for now, no line of sight check for consitency in attacks
                {
                    // if on firebreath cooldown, throw pumpkin
                    if agent.combat_state.timers[FIREBREATH] < FIREBREATH_COOLDOWN {
                        controller.push_basic_input(InputKind::Ability(0));
                    }
                    // otherwise, randomise between firebreath and pumpkin
                    else if rng.gen_bool(0.5) {
                        controller.push_basic_input(InputKind::Secondary);
                    } else {
                        controller.push_basic_input(InputKind::Ability(0));
                    }
                    // reset mixup cooldown if actually being used
                    if is_using_mixup {
                        agent.combat_state.counters[CLOSE_MIXUP_COOLDOWN] =
                            rng_from_span(rng, CLOSE_MIXUP_COOLDOWN_SPAN);
                    }
                }
                // default to using scythe melee
                else {
                    controller.push_basic_input(InputKind::Primary);
                }
            }
        // mid range (line of sight not needed for these 'suppressing' attacks)
        } else if attack_data.dist_sqrd < firebreath_range.powi(2) {
            // if using firebreath, keep going under full time limit
            #[expect(clippy::if_same_then_else)]
            if is_using_firebreath
                && firebreath_timer < Duration::from_secs_f32(FIREBREATH_TIME_LIMIT)
            {
                controller.push_basic_input(InputKind::Secondary);
            }
            // start using firebreath if close enough, in angle, and off cooldown
            else if attack_data.dist_sqrd < (firebreath_range * FIREBREATH_RANGE_FACTOR).powi(2)
                && attack_data.angle < firebreath_angle * FIREBREATH_AIM_FACTOR
                && agent.combat_state.timers[FIREBREATH] > FIREBREATH_COOLDOWN
            {
                controller.push_basic_input(InputKind::Secondary);
            }
            // on mixup timer, throw a pumpkin
            else if agent.combat_state.timers[MIXUP]
                > agent.combat_state.counters[MID_MIXUP_COOLDOWN]
            {
                controller.push_basic_input(InputKind::Ability(0));
                // reset mixup cooldown if pumpkin is actually being used
                if is_using_pumpkin {
                    agent.combat_state.counters[MID_MIXUP_COOLDOWN] =
                        rng_from_span(rng, MID_MIXUP_COOLDOWN_SPAN);
                }
            }
        }
        // long range (with line of sight)
        else if attack_data.dist_sqrd < (pumpkin_max_range * PUMPKIN_RANGE_FACTOR).powi(2)
            && agent.combat_state.timers[FAR_PUMPKIN]
                > agent.combat_state.counters[FAR_PUMPKIN_COOLDOWN]
            && line_of_sight_with_target()
        {
            // throw pumpkin
            controller.push_basic_input(InputKind::Ability(0));
            // reset pumpkin cooldown if actually being used
            if is_using_pumpkin {
                agent.combat_state.counters[FAR_PUMPKIN_COOLDOWN] =
                    rng_from_span(rng, FAR_PUMPKIN_COOLDOWN_SPAN);
            }
        }

        // --- movement ---
        // closing gap
        if attack_data.dist_sqrd
            > (attack_data.body_dist + scythe_range * PATH_RANGE_FACTOR).powi(2)
        {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
        // closing angle
        else if attack_data.angle > 0.0 {
            // some movement is required to trigger re-orientation
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::zero)
                * 0.001; // scaled way down to minimise position change and keep close rotation consistent
        }
    }

    pub fn handle_frostgigas_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const GIGAS_MELEE_RANGE: f32 = 12.0;
        const GIGAS_SPIKE_RANGE: f32 = 16.0;
        const ICEBOMB_RANGE: f32 = 70.0;
        const GIGAS_LEAP_RANGE: f32 = 50.0;
        const MINION_SUMMON_THRESHOLD: f32 = 1. / 8.;
        const FLASHFREEZE_RANGE: f32 = 30.;

        enum ActionStateTimers {
            AttackChange,
            Bonk,
        }

        enum ActionStateFCounters {
            FCounterMinionSummonThreshold = 0,
        }

        enum ActionStateICounters {
            /// An ability that is forced to fully complete until moving on to
            /// other attacks.
            /// 1 = Leap shockwave, 2 = Flashfreeze, 3 = Spike summon,
            /// 4 = Whirlwind, 5 = Remote ice spikes, 6 = Ice bombs
            CurrentAbility = 0,
        }

        let should_use_targeted_spikes = || matches!(self.physics_state.in_fluid, Some(Fluid::Liquid { depth, .. }) if depth >= 2.0);
        let remote_spikes_action = || ControlAction::StartInput {
            input: InputKind::Ability(5),
            target_entity: None,
            select_pos: Some(tgt_data.pos.0),
        };

        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already initialized
        if !agent.combat_state.initialized {
            agent.combat_state.counters
                [ActionStateFCounters::FCounterMinionSummonThreshold as usize] =
                1.0 - MINION_SUMMON_THRESHOLD;
            agent.combat_state.initialized = true;
        }

        // Update timers
        if agent.combat_state.timers[ActionStateTimers::AttackChange as usize] > 6.0 {
            agent.combat_state.timers[ActionStateTimers::AttackChange as usize] = 0.0;
        } else {
            agent.combat_state.timers[ActionStateTimers::AttackChange as usize] += read_data.dt.0;
        }
        agent.combat_state.timers[ActionStateTimers::Bonk as usize] += read_data.dt.0;

        if health_fraction
            < agent.combat_state.counters
                [ActionStateFCounters::FCounterMinionSummonThreshold as usize]
        {
            // Summon minions at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(3));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterMinionSummonThreshold as usize] -=
                    MINION_SUMMON_THRESHOLD;
            }
        // Continue casting any attacks that are forced to complete
        } else if let Some(ability) = Some(
            &mut agent.combat_state.int_counters[ActionStateICounters::CurrentAbility as usize],
        )
        .filter(|i| **i != 0)
        {
            if *ability == 3 && should_use_targeted_spikes() {
                *ability = 5
            };

            let reset = match ability {
                // Must be rolled
                1 => {
                    controller.push_basic_input(InputKind::Ability(1));
                    matches!(self.char_state, CharacterState::LeapShockwave(c) if matches!(c.stage_section, StageSection::Recover))
                },
                // Attacker will have to run away here
                2 => {
                    controller.push_basic_input(InputKind::Ability(4));
                    matches!(self.char_state, CharacterState::Shockwave(c) if matches!(c.stage_section, StageSection::Recover))
                },
                // Avoid the spikes!
                3 => {
                    controller.push_basic_input(InputKind::Ability(0));
                    matches!(self.char_state, CharacterState::SpriteSummon(c)
                        if matches!((c.stage_section, c.static_data.anchor), (StageSection::Recover, SpriteSummonAnchor::Summoner)))
                },
                // Long whirlwind attack
                4 => {
                    controller.push_basic_input(InputKind::Ability(7));
                    matches!(self.char_state, CharacterState::RapidMelee(c) if matches!(c.stage_section, StageSection::Recover))
                },
                // Remote ice spikes
                5 => {
                    controller.push_action(remote_spikes_action());
                    matches!(self.char_state, CharacterState::SpriteSummon(c)
                        if matches!((c.stage_section, c.static_data.anchor), (StageSection::Recover, SpriteSummonAnchor::Target)))
                },
                // Ice bombs
                6 => {
                    controller.push_basic_input(InputKind::Ability(2));
                    matches!(self.char_state, CharacterState::BasicRanged(c) if matches!(c.stage_section, StageSection::Recover))
                },
                // Should never happen
                _ => true,
            };

            if reset {
                *ability = 0;
            }
        // If our target is nearby and above us, potentially cheesing, have a
        // chance of summoning remote ice spikes or throwing ice bombs.
        // Cheesing from less than 5 blocks away is usually not possible
        } else if attack_data.dist_sqrd > 5f32.powi(2)
            // Calculate the "cheesing factor" (height of the normalized position difference)
            && (tgt_data.pos.0 - self.pos.0).normalized().map(f32::abs).z > 0.6
            // Make it happen at about every 10 seconds!
            && rng.gen_bool((0.2 * read_data.dt.0).min(1.0) as f64)
        {
            agent.combat_state.int_counters[ActionStateICounters::CurrentAbility as usize] =
                rng.gen_range(5..=6);
        } else if attack_data.dist_sqrd < GIGAS_MELEE_RANGE.powi(2) {
            // Bonk the target every 10-8 s
            if agent.combat_state.timers[ActionStateTimers::Bonk as usize] > 10. {
                controller.push_basic_input(InputKind::Ability(6));

                if matches!(self.char_state, CharacterState::BasicMelee(c)
                    if matches!(c.stage_section, StageSection::Recover) &&
                    c.static_data.ability_info.ability.is_some_and(|meta| matches!(meta.ability, Ability::MainWeaponAux(6)))
                ) {
                    agent.combat_state.timers[ActionStateTimers::Bonk as usize] =
                        rng.gen_range(0.0..3.0);
                }
            // Have a small chance at starting a mixup attack
            } else if agent.combat_state.timers[ActionStateTimers::AttackChange as usize] > 4.0
                && rng.gen_bool(0.1 * read_data.dt.0.min(1.0) as f64)
            {
                agent.combat_state.int_counters[ActionStateICounters::CurrentAbility as usize] =
                    rng.gen_range(1..=4);
            // Melee the target, do a whirlwind whenever he is trying to go
            // behind or after every 5s
            } else if attack_data.angle > 90.0
                || agent.combat_state.timers[ActionStateTimers::AttackChange as usize] > 5.0
            {
                // If our target is *very* behind, punish with a whirlwind
                if attack_data.angle > 120.0 {
                    agent.combat_state.int_counters
                        [ActionStateICounters::CurrentAbility as usize] = 4;
                } else {
                    controller.push_basic_input(InputKind::Secondary);
                }
            } else {
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd < GIGAS_SPIKE_RANGE.powi(2)
            && agent.combat_state.timers[ActionStateTimers::AttackChange as usize] < 2.0
        {
            if should_use_targeted_spikes() {
                controller.push_action(remote_spikes_action());
            } else {
                controller.push_basic_input(InputKind::Ability(0));
            }
        } else if attack_data.dist_sqrd < FLASHFREEZE_RANGE.powi(2)
            && agent.combat_state.timers[ActionStateTimers::AttackChange as usize] < 4.0
        {
            controller.push_basic_input(InputKind::Ability(4));
        // Start a leap after either every 3s or our target is not in LoS
        } else if attack_data.dist_sqrd < GIGAS_LEAP_RANGE.powi(2)
            && agent.combat_state.timers[ActionStateTimers::AttackChange as usize] > 3.0
        {
            controller.push_basic_input(InputKind::Ability(1));
        } else if attack_data.dist_sqrd < ICEBOMB_RANGE.powi(2)
            && agent.combat_state.timers[ActionStateTimers::AttackChange as usize] < 3.0
        {
            controller.push_basic_input(InputKind::Ability(2));
        // Spawn ice sprites under distant attackers
        } else {
            controller.push_action(remote_spikes_action());
        }

        // Always attempt to path towards target
        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Partial,
            attack_data.in_min_range().then_some(0.1),
        );
    }

    pub fn handle_boreal_hammer_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            TimerHandleHammerAttack = 0,
        }

        let has_energy = |need| self.energy.current() > need;

        let use_leap = |controller: &mut Controller| {
            controller.push_basic_input(InputKind::Ability(0));
        };

        agent.combat_state.timers[ActionStateTimers::TimerHandleHammerAttack as usize] +=
            read_data.dt.0;

        if attack_data.in_min_range() && attack_data.angle < 45.0 {
            controller.inputs.move_dir = Vec2::zero();
            if agent.combat_state.timers[ActionStateTimers::TimerHandleHammerAttack as usize] > 4.0
            {
                controller.push_cancel_input(InputKind::Secondary);
                agent.combat_state.timers[ActionStateTimers::TimerHandleHammerAttack as usize] =
                    0.0;
            } else if agent.combat_state.timers[ActionStateTimers::TimerHandleHammerAttack as usize]
                > 3.0
            {
                controller.push_basic_input(InputKind::Secondary);
            } else if has_energy(50.0) && rng.gen_bool(0.9) {
                use_leap(controller);
            } else {
                controller.push_basic_input(InputKind::Primary);
            }
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );

            if attack_data.dist_sqrd < 32.0f32.powi(2)
                && entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
            {
                if rng.gen_bool(0.5) && has_energy(50.0) {
                    use_leap(controller);
                } else if agent.combat_state.timers
                    [ActionStateTimers::TimerHandleHammerAttack as usize]
                    > 2.0
                {
                    controller.push_basic_input(InputKind::Secondary);
                } else if agent.combat_state.timers
                    [ActionStateTimers::TimerHandleHammerAttack as usize]
                    > 4.0
                {
                    controller.push_cancel_input(InputKind::Secondary);
                    agent.combat_state.timers
                        [ActionStateTimers::TimerHandleHammerAttack as usize] = 0.0;
                }
            }
        }
    }

    pub fn handle_boreal_bow_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        let has_energy = |need| self.energy.current() > need;

        let use_trap = |controller: &mut Controller| {
            controller.push_basic_input(InputKind::Ability(0));
        };

        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            if rng.gen_bool(0.5) && has_energy(15.0) {
                controller.push_basic_input(InputKind::Secondary);
            } else if attack_data.angle < 15.0 {
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd < (4.0 * attack_data.min_attack_dist).powi(2)
            && line_of_sight_with_target()
        {
            if rng.gen_bool(0.5) && has_energy(15.0) {
                controller.push_basic_input(InputKind::Secondary);
            } else if has_energy(20.0) {
                use_trap(controller);
            }
        }

        if has_energy(50.0) {
            if attack_data.dist_sqrd < (10.0 * attack_data.min_attack_dist).powi(2) {
                // Attempt to circle the target if neither too close nor too far
                if let Some((bearing, speed)) = agent.chaser.chase(
                    &*read_data.terrain,
                    self.pos.0,
                    self.vel.0,
                    tgt_data.pos.0,
                    TraversalConfig {
                        min_tgt_dist: 1.25,
                        ..self.traversal_config
                    },
                ) {
                    if line_of_sight_with_target() && attack_data.angle < 45.0 {
                        controller.inputs.move_dir = bearing
                            .xy()
                            .rotated_z(rng.gen_range(0.5..1.57))
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * 2.0
                            * speed;
                    } else {
                        // Unless cannot see target, then move towards them
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(bearing.z > 1.5, controller);
                        controller.inputs.move_z = bearing.z;
                    }
                }
            } else {
                // Path to enemy if too far
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Partial,
                    None,
                );
            }
        } else {
            // Path to enemy for melee hits if need more energy
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_firegigas_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const MELEE_RANGE: f32 = 12.0;
        const RANGED_RANGE: f32 = 27.0;
        const LEAP_RANGE: f32 = 50.0;
        const MINION_SUMMON_THRESHOLD: f32 = 1.0 / 8.0;
        const OVERHEAT_DUR: f32 = 3.0;
        const FORCE_GAP_CLOSER_TIMEOUT: f32 = 10.0;

        enum ActionStateTimers {
            Special,
            Overheat,
            OutOfMeleeRange,
        }

        enum ActionStateFCounters {
            FCounterMinionSummonThreshold,
        }

        enum ActionStateConditions {
            VerticalStrikeCombo,
        }

        const FAST_SLASH: InputKind = InputKind::Primary;
        const FAST_THRUST: InputKind = InputKind::Secondary;
        const SLOW_SLASH: InputKind = InputKind::Ability(0);
        const SLOW_THRUST: InputKind = InputKind::Ability(1);
        const LAVA_LEAP: InputKind = InputKind::Ability(2);
        const VERTICAL_STRIKE: InputKind = InputKind::Ability(3);
        const OVERHEAT: InputKind = InputKind::Ability(4);
        const WHIRLWIND: InputKind = InputKind::Ability(5);
        const EXPLOSIVE_STRIKE: InputKind = InputKind::Ability(6);
        const FIRE_PILLARS: InputKind = InputKind::Ability(7);
        const TARGETED_FIRE_PILLAR: InputKind = InputKind::Ability(8);
        const ASHEN_SUMMONS: InputKind = InputKind::Ability(9);

        fn choose_weighted<const N: usize>(
            rng: &mut impl Rng,
            choices: [(InputKind, f32); N],
        ) -> InputKind {
            choices
                .choose_weighted(rng, |(_, weight)| *weight)
                .expect("weights should be valid")
                .0
        }

        // Basic melee strikes
        fn rand_basic(rng: &mut impl Rng, damage_fraction: f32) -> InputKind {
            choose_weighted(rng, [
                (FAST_SLASH, 2.0),
                (FAST_THRUST, 2.0),
                (SLOW_SLASH, 1.0 + damage_fraction),
                (SLOW_THRUST, 1.0 + damage_fraction),
            ])
        }

        // Less frequent mixup attacks
        fn rand_special(rng: &mut impl Rng) -> InputKind {
            choose_weighted(rng, [
                (LAVA_LEAP, 5.0),
                (VERTICAL_STRIKE, 5.0),
                (OVERHEAT, 5.0),
                (WHIRLWIND, 5.0),
                (EXPLOSIVE_STRIKE, 1.0),
                (FIRE_PILLARS, 1.0),
            ])
        }

        // Attacks capable of also hitting entities behind the gigas
        fn rand_aoe(rng: &mut impl Rng) -> InputKind {
            choose_weighted(rng, [
                (EXPLOSIVE_STRIKE, 1.0),
                (FIRE_PILLARS, 1.0),
                (WHIRLWIND, 2.0),
            ])
        }

        // Attacks capable of also hitting entities further away
        fn rand_ranged(rng: &mut impl Rng) -> InputKind {
            choose_weighted(rng, [
                (EXPLOSIVE_STRIKE, 1.0),
                (FIRE_PILLARS, 1.0),
                (OVERHEAT, 1.0),
            ])
        }

        let cast_targeted_fire_pillar = |c: &mut Controller| {
            c.push_action(ControlAction::StartInput {
                input: TARGETED_FIRE_PILLAR,
                target_entity: tgt_data.uid,
                select_pos: None,
            })
        };

        fn can_cast_new_ability(char_state: &CharacterState) -> bool {
            !matches!(
                char_state,
                CharacterState::LeapMelee(_)
                    | CharacterState::BasicMelee(_)
                    | CharacterState::BasicBeam(_)
                    | CharacterState::BasicSummon(_)
                    | CharacterState::SpriteSummon(_)
            )
        }

        // Initializes counters at start of combat
        if !agent.combat_state.initialized {
            agent.combat_state.counters
                [ActionStateFCounters::FCounterMinionSummonThreshold as usize] =
                1.0 - MINION_SUMMON_THRESHOLD;
            agent.combat_state.initialized = true;
        }

        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        let damage_fraction = 1.0 - health_fraction;
        // Calculate the "cheesing factor" (height of the normalized position
        // difference), unless the target is airborne from our hit
        // Cheesing from close range is usually not possible
        let cheesed_from_above = !agent.combat_state.conditions
            [ActionStateConditions::VerticalStrikeCombo as usize]
            && attack_data.dist_sqrd > 5f32.powi(2)
            && (tgt_data.pos.0 - self.pos.0).normalized().map(f32::abs).z > 0.6;
        // Being in water also triggers this as there are a lot of exploits with water
        let cheesed_in_water = matches!(self.physics_state.in_fluid, Some(Fluid::Liquid { kind: LiquidKind::Water, depth, .. }) if depth >= 2.0);
        let cheesed = cheesed_from_above || cheesed_in_water;
        let tgt_airborne = tgt_data
            .physics_state
            .is_some_and(|physics| physics.on_ground.is_none() && physics.in_liquid().is_none());
        let casting_beam = matches!(self.char_state, CharacterState::BasicBeam(_))
            && self.char_state.stage_section() != Some(StageSection::Recover);

        // Update timers
        agent.combat_state.timers[ActionStateTimers::Special as usize] += read_data.dt.0;
        if casting_beam {
            agent.combat_state.timers[ActionStateTimers::Overheat as usize] += read_data.dt.0;
        } else {
            agent.combat_state.timers[ActionStateTimers::Overheat as usize] = 0.0;
        }
        if attack_data.dist_sqrd > MELEE_RANGE.powi(2) {
            agent.combat_state.timers[ActionStateTimers::OutOfMeleeRange as usize] +=
                read_data.dt.0;
        } else {
            agent.combat_state.timers[ActionStateTimers::OutOfMeleeRange as usize] = 0.0;
        }

        // Cast abilities
        if casting_beam
            && agent.combat_state.timers[ActionStateTimers::Overheat as usize] < OVERHEAT_DUR
        {
            controller.push_basic_input(OVERHEAT);
            controller.inputs.look_dir = self
                .ori
                .look_dir()
                .to_horizontal()
                .unwrap_or_else(|| self.ori.look_dir());
        } else if health_fraction
            < agent.combat_state.counters
                [ActionStateFCounters::FCounterMinionSummonThreshold as usize]
        {
            // Summon minions at particular thresholds of health
            controller.push_basic_input(ASHEN_SUMMONS);

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterMinionSummonThreshold as usize] -=
                    MINION_SUMMON_THRESHOLD;
            }
        } else if can_cast_new_ability(self.char_state) {
            if cheesed {
                cast_targeted_fire_pillar(controller);
            } else if agent.combat_state.conditions
                [ActionStateConditions::VerticalStrikeCombo as usize]
            {
                // If landed vertical strike combo target while they are airborne
                if tgt_airborne {
                    controller.push_basic_input(FAST_THRUST);
                }

                agent.combat_state.conditions
                    [ActionStateConditions::VerticalStrikeCombo as usize] = false;
            } else if agent.combat_state.timers[ActionStateTimers::OutOfMeleeRange as usize]
                > FORCE_GAP_CLOSER_TIMEOUT
            {
                // Use a gap closer if the target has been out of melee distance for a while
                controller.push_basic_input(LAVA_LEAP);
            } else if attack_data.dist_sqrd < MELEE_RANGE.powi(2) {
                if agent.combat_state.timers[ActionStateTimers::Special as usize] > 10.0 {
                    // Use a special ability periodically
                    let rand_special = rand_special(rng);
                    if rand_special == VERTICAL_STRIKE {
                        agent.combat_state.conditions
                            [ActionStateConditions::VerticalStrikeCombo as usize] = true;
                    }
                    controller.push_basic_input(rand_special);

                    agent.combat_state.timers[ActionStateTimers::Special as usize] =
                        rng.gen_range(0.0..3.0 + 5.0 * damage_fraction);
                } else if attack_data.angle > 90.0 {
                    // Cast an aoe ability to hit the target if they are behind the entity
                    controller.push_basic_input(rand_aoe(rng));
                } else {
                    // Use a random basic melee hit
                    controller.push_basic_input(rand_basic(rng, damage_fraction));
                }
            } else if attack_data.dist_sqrd < RANGED_RANGE.powi(2) {
                // Use ranged ability if target is out of melee range
                if rng.gen_bool(0.05) {
                    controller.push_basic_input(rand_ranged(rng));
                }
            } else if attack_data.dist_sqrd < LEAP_RANGE.powi(2) {
                // Use a gap closer if the target is even further away
                controller.push_basic_input(LAVA_LEAP);
            } else if rng.gen_bool(0.1) {
                // Use a targeted fire pillar if the target is out of range of everything else
                cast_targeted_fire_pillar(controller);
            }
        }

        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Partial,
            attack_data.in_min_range().then_some(0.1),
        );

        // Get out of lava if submerged
        if self.physics_state.in_liquid().is_some() {
            controller.push_basic_input(InputKind::Jump);
        }
        if self.physics_state.in_liquid().is_some() {
            controller.inputs.move_z = 1.0;
        }
    }

    pub fn handle_ashen_axe_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const IMMOLATION_COOLDOWN: f32 = 50.0;
        const ABILITY_PREFERENCES: AbilityPreferences = AbilityPreferences {
            desired_energy: 30.0,
            combo_scaling_buildup: 0,
        };

        enum ActionStateTimers {
            SinceSelfImmolation,
        }

        const DOUBLE_STRIKE: InputKind = InputKind::Primary;
        const FLAME_WAVE: InputKind = InputKind::Secondary;
        const KNOCKBACK_COMBO: InputKind = InputKind::Ability(0);
        const SELF_IMMOLATION: InputKind = InputKind::Ability(1);

        fn can_cast_new_ability(char_state: &CharacterState) -> bool {
            !matches!(
                char_state,
                CharacterState::ComboMelee2(_)
                    | CharacterState::Shockwave(_)
                    | CharacterState::SelfBuff(_)
            )
        }

        let could_use = |input| {
            Option::<AbilityInput>::from(input)
                .and_then(|ability_input| self.extract_ability(ability_input))
                .is_some_and(|ability_data| {
                    ability_data.could_use(
                        attack_data,
                        self,
                        tgt_data,
                        read_data,
                        ABILITY_PREFERENCES,
                    )
                })
        };

        // Initialize immolation cooldown to 0
        if !agent.combat_state.initialized {
            agent.combat_state.timers[ActionStateTimers::SinceSelfImmolation as usize] =
                IMMOLATION_COOLDOWN;
            agent.combat_state.initialized = true;
        }

        agent.combat_state.timers[ActionStateTimers::SinceSelfImmolation as usize] +=
            read_data.dt.0;

        if self
            .char_state
            .ability_info()
            .map(|ai| ai.input)
            .is_some_and(|input_kind| input_kind == KNOCKBACK_COMBO)
        {
            controller.push_basic_input(KNOCKBACK_COMBO);
        } else if can_cast_new_ability(self.char_state)
            && agent.combat_state.timers[ActionStateTimers::SinceSelfImmolation as usize]
                >= IMMOLATION_COOLDOWN
            && could_use(SELF_IMMOLATION)
        {
            agent.combat_state.timers[ActionStateTimers::SinceSelfImmolation as usize] =
                rng.gen_range(0.0..5.0);

            controller.push_basic_input(SELF_IMMOLATION);
        } else if rng.gen_bool(0.35) && could_use(KNOCKBACK_COMBO) {
            controller.push_basic_input(KNOCKBACK_COMBO);
        } else if could_use(DOUBLE_STRIKE) {
            controller.push_basic_input(DOUBLE_STRIKE);
        } else if rng.gen_bool(0.2) && could_use(FLAME_WAVE) {
            controller.push_basic_input(FLAME_WAVE);
        }

        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Full,
            None,
        );
    }

    pub fn handle_ashen_staff_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const ABILITY_COOLDOWN: f32 = 50.0;
        const INITIAL_COOLDOWN: f32 = ABILITY_COOLDOWN - 10.0;
        const ABILITY_PREFERENCES: AbilityPreferences = AbilityPreferences {
            desired_energy: 40.0,
            combo_scaling_buildup: 0,
        };

        enum ActionStateTimers {
            SinceAbility,
        }

        const FIREBALL: InputKind = InputKind::Primary;
        const FLAME_WALL: InputKind = InputKind::Ability(0);
        const SUMMON_CRUX: InputKind = InputKind::Ability(1);

        fn can_cast_new_ability(char_state: &CharacterState) -> bool {
            !matches!(
                char_state,
                CharacterState::BasicRanged(_)
                    | CharacterState::BasicBeam(_)
                    | CharacterState::RapidMelee(_)
                    | CharacterState::BasicAura(_)
            )
        }

        let could_use = |input| {
            Option::<AbilityInput>::from(input)
                .and_then(|ability_input| self.extract_ability(ability_input))
                .is_some_and(|ability_data| {
                    ability_data.could_use(
                        attack_data,
                        self,
                        tgt_data,
                        read_data,
                        ABILITY_PREFERENCES,
                    )
                })
        };

        // Initialize special ability cooldown
        if !agent.combat_state.initialized {
            agent.combat_state.timers[ActionStateTimers::SinceAbility as usize] = INITIAL_COOLDOWN;
            agent.combat_state.initialized = true;
        }

        agent.combat_state.timers[ActionStateTimers::SinceAbility as usize] += read_data.dt.0;

        if can_cast_new_ability(self.char_state)
            && agent.combat_state.timers[ActionStateTimers::SinceAbility as usize]
                >= ABILITY_COOLDOWN
            && (could_use(FLAME_WALL) || could_use(SUMMON_CRUX))
        {
            agent.combat_state.timers[ActionStateTimers::SinceAbility as usize] =
                rng.gen_range(0.0..5.0);

            if could_use(FLAME_WALL) && (rng.gen_bool(0.5) || !could_use(SUMMON_CRUX)) {
                controller.push_basic_input(FLAME_WALL);
            } else {
                controller.push_basic_input(SUMMON_CRUX);
            }
        } else if rng.gen_bool(0.5) && could_use(FIREBALL) {
            controller.push_basic_input(FIREBALL);
        }

        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                ) && attack_data.angle < 45.0
                {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(rng.gen_range(-1.57..-0.5))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_cardinal_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const DESIRED_ENERGY_LEVEL: f32 = 50.0;
        const DESIRED_COMBO_LEVEL: u32 = 8;
        const MINION_SUMMON_THRESHOLD: f32 = 0.10;

        enum ActionStateConditions {
            ConditionCounterInitialized = 0,
        }

        enum ActionStateFCounters {
            FCounterHealthThreshold = 0,
        }

        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already intitialized
        if !agent.combat_state.conditions
            [ActionStateConditions::ConditionCounterInitialized as usize]
        {
            agent.combat_state.counters[ActionStateFCounters::FCounterHealthThreshold as usize] =
                1.0 - MINION_SUMMON_THRESHOLD;
            agent.combat_state.conditions
                [ActionStateConditions::ConditionCounterInitialized as usize] = true;
        }

        if agent.combat_state.counters[ActionStateFCounters::FCounterHealthThreshold as usize]
            > health_fraction
        {
            // Summon minions at particular thresholds of health
            controller.push_basic_input(InputKind::Ability(1));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterHealthThreshold as usize] -=
                    MINION_SUMMON_THRESHOLD;
            }
        }
        // Logic to use abilities
        else if attack_data.dist_sqrd > attack_data.min_attack_dist.powi(2)
            && entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        {
            // If far enough away, and can see target, check which skill is appropriate to
            // use
            if self.energy.current() > DESIRED_ENERGY_LEVEL
                && read_data
                    .combos
                    .get(*self.entity)
                    .is_some_and(|c| c.counter() >= DESIRED_COMBO_LEVEL)
                && !read_data.buffs.get(*self.entity).iter().any(|buff| {
                    buff.iter_kind(BuffKind::Regeneration)
                        .peekable()
                        .peek()
                        .is_some()
                })
            {
                // If have enough energy and combo to use healing aura, do so
                controller.push_basic_input(InputKind::Secondary);
            } else if self
                .skill_set
                .has_skill(Skill::Sceptre(SceptreSkill::UnlockAura))
                && self.energy.current() > DESIRED_ENERGY_LEVEL
                && !read_data.buffs.get(*self.entity).iter().any(|buff| {
                    buff.iter_kind(BuffKind::ProtectingWard)
                        .peekable()
                        .peek()
                        .is_some()
                })
            {
                // Use steam beam if target is far enough away, self is not buffed, and have
                // sufficient energy
                controller.push_basic_input(InputKind::Ability(0));
            } else {
                // If low on energy, use primary to attempt to regen energy
                // Or if at desired energy level but not able/willing to ward, just attack
                controller.push_basic_input(InputKind::Primary);
            }
        } else if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            if self.body.is_some_and(|b| b.is_humanoid())
                && self.energy.current()
                    > CharacterAbility::default_roll(Some(self.char_state)).energy_cost()
                && !matches!(self.char_state, CharacterState::BasicAura(c) if !matches!(c.stage_section, StageSection::Recover))
            {
                // Else use steam beam
                controller.push_basic_input(InputKind::Ability(0));
            } else if attack_data.angle < 15.0 {
                controller.push_basic_input(InputKind::Primary);
            }
        }
        // Logic to move. Intentionally kept separate from ability logic where possible
        // so duplicated work is less necessary.
        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                ) && attack_data.angle < 45.0
                {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(rng.gen_range(0.5..1.57))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            }
            // Sometimes try to roll
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && !matches!(self.char_state, CharacterState::BasicAura(_))
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && rng.gen::<f32>() < 0.01
            {
                controller.push_basic_input(InputKind::Roll);
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_sea_bishop_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        enum ActionStateTimers {
            TimerBeam = 0,
        }
        if agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] > 6.0 {
            agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] = 0.0;
        } else {
            agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] += read_data.dt.0;
        }

        // When enemy in sight beam for 3 seconds, every 6 seconds
        if line_of_sight_with_target()
            && agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] < 3.0
        {
            controller.push_basic_input(InputKind::Primary);
        }
        // Logic to move. Intentionally kept separate from ability logic where possible
        // so duplicated work is less necessary.
        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if line_of_sight_with_target() && attack_data.angle < 45.0 {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(rng.gen_range(0.5..1.57))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(bearing.z > 1.5, controller);
                    controller.inputs.move_z = bearing.z;
                }
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_cursekeeper_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            TimerBeam,
            TimerSummon,
            SelectSummon,
        }
        if tgt_data.pos.0.z - self.pos.0.z > 3.5 {
            controller.push_action(ControlAction::StartInput {
                input: InputKind::Ability(4),
                target_entity: agent
                    .target
                    .as_ref()
                    .and_then(|t| read_data.uids.get(t.target))
                    .copied(),
                select_pos: None,
            });
        } else if agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] > 12.0 {
            agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] = 0.0;
        } else {
            agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] += read_data.dt.0;
        }

        if matches!(self.char_state, CharacterState::BasicSummon(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            agent.combat_state.timers[ActionStateTimers::TimerSummon as usize] = 0.0;
            agent.combat_state.timers[ActionStateTimers::SelectSummon as usize] =
                rng.gen_range(0..=3) as f32;
        } else {
            agent.combat_state.timers[ActionStateTimers::TimerSummon as usize] += read_data.dt.0;
        }

        if agent.combat_state.timers[ActionStateTimers::TimerSummon as usize] > 32.0 {
            match agent.combat_state.timers[ActionStateTimers::SelectSummon as usize] as i32 {
                0 => controller.push_basic_input(InputKind::Ability(0)),
                1 => controller.push_basic_input(InputKind::Ability(1)),
                2 => controller.push_basic_input(InputKind::Ability(2)),
                _ => controller.push_basic_input(InputKind::Ability(3)),
            }
        } else if agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] < 6.0 {
            controller.push_basic_input(InputKind::Ability(5));
        } else if agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] < 9.0 {
            controller.push_basic_input(InputKind::Primary);
        } else {
            controller.push_basic_input(InputKind::Secondary);
        }

        if attack_data.dist_sqrd > 10_f32.powi(2)
            || agent.combat_state.timers[ActionStateTimers::TimerBeam as usize] > 4.0
        {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
    }

    pub fn handle_shamanic_spirit_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if tgt_data.pos.0.z - self.pos.0.z > 5.0 {
            controller.push_action(ControlAction::StartInput {
                input: InputKind::Secondary,
                target_entity: agent
                    .target
                    .as_ref()
                    .and_then(|t| read_data.uids.get(t.target))
                    .copied(),
                select_pos: None,
            });
        } else if attack_data.in_min_range() && attack_data.angle < 30.0 {
            controller.push_basic_input(InputKind::Primary);
            controller.inputs.move_dir = Vec2::zero();
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
    }

    pub fn handle_cursekeeper_fake_attack(
        &self,
        controller: &mut Controller,
        attack_data: &AttackData,
    ) {
        if attack_data.dist_sqrd < 25_f32.powi(2) {
            controller.push_basic_input(InputKind::Primary);
        }
    }

    pub fn handle_karkatha_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        _rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            RiposteTimer,
            SummonTimer,
        }

        agent.combat_state.timers[ActionStateTimers::RiposteTimer as usize] += read_data.dt.0;
        agent.combat_state.timers[ActionStateTimers::SummonTimer as usize] += read_data.dt.0;
        if matches!(self.char_state, CharacterState::RiposteMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::RiposteTimer as usize] = 0.0;
        }
        if matches!(self.char_state, CharacterState::BasicSummon(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::SummonTimer as usize] = 0.0;
        }
        // chase, move away from exiit if target is cheesing from below
        let home = agent.patrol_origin.unwrap_or(self.pos.0);
        let dest = if tgt_data.pos.0.z < self.pos.0.z {
            home
        } else {
            tgt_data.pos.0
        };
        if attack_data.in_min_range() {
            if agent.combat_state.timers[ActionStateTimers::RiposteTimer as usize] > 3.0 {
                controller.push_basic_input(InputKind::Ability(2));
            } else {
                controller.push_basic_input(InputKind::Primary);
            };
        } else if attack_data.dist_sqrd < 20.0_f32.powi(2) {
            if agent.combat_state.timers[ActionStateTimers::SummonTimer as usize] > 20.0 {
                controller.push_basic_input(InputKind::Ability(1));
            } else {
                controller.push_basic_input(InputKind::Secondary);
            }
        } else if attack_data.dist_sqrd < 30.0_f32.powi(2) {
            if agent.combat_state.timers[ActionStateTimers::SummonTimer as usize] < 10.0 {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Full,
                    None,
                );
            } else {
                controller.push_basic_input(InputKind::Ability(0));
            }
        } else {
            self.path_toward_target(agent, controller, dest, read_data, Path::Full, None);
        }
    }

    pub fn handle_dagon_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            TimerDagon = 0,
        }
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        // when cheesed from behind the entry, change position to retarget
        let home = agent.patrol_origin.unwrap_or(self.pos.0);
        let exit = Vec3::new(home.x - 6.0, home.y - 6.0, home.z);
        let (station_0, station_1) = (exit + 12.0, exit - 12.0);
        if agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] > 2.5 {
            agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] = 0.0;
        }
        if !line_of_sight_with_target()
            && (tgt_data.pos.0 - exit).xy().magnitude_squared() < (10.0_f32).powi(2)
        {
            let station = if (tgt_data.pos.0 - station_0).xy().magnitude_squared()
                < (tgt_data.pos.0 - station_1).xy().magnitude_squared()
            {
                station_0
            } else {
                station_1
            };
            self.path_toward_target(agent, controller, station, read_data, Path::Full, None);
        }
        // if target gets very close, shoot dagon bombs and lay out sea urchins
        else if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            if agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] > 1.0 {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] += read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Secondary);
                agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] += read_data.dt.0;
            }
            // if target in close range use steambeam and shoot dagon bombs
        } else if attack_data.dist_sqrd < (3.0 * attack_data.min_attack_dist).powi(2) {
            controller.inputs.move_dir = Vec2::zero();
            if agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] > 2.0 {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] += read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Ability(1));
            }
        } else if attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2) {
            // if enemy is far, heal and shoot bombs
            if agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] > 2.0 {
                controller.push_basic_input(InputKind::Primary);
            } else {
                controller.push_basic_input(InputKind::Ability(2));
            }
            agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] += read_data.dt.0;
        } else if line_of_sight_with_target() {
            // if enemy in mid range shoot dagon bombs and steamwave
            if agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] > 1.0 {
                controller.push_basic_input(InputKind::Primary);
                agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] += read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Ability(0));
                agent.combat_state.timers[ActionStateTimers::TimerDagon as usize] += read_data.dt.0;
            }
        }
        // chase
        let path = if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            Path::Separate
        } else {
            Path::Partial
        };
        self.path_toward_target(agent, controller, tgt_data.pos.0, read_data, path, None);
    }

    pub fn handle_snaretongue_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        read_data: &ReadData,
    ) {
        enum Timers {
            TimerAttack = 0,
        }
        let attack_timer = &mut agent.combat_state.timers[Timers::TimerAttack as usize];
        if *attack_timer > 2.5 {
            *attack_timer = 0.0;
        }
        // if target gets very close, use tongue attack and shockwave
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) {
            if *attack_timer > 0.5 {
                controller.push_basic_input(InputKind::Primary);
                *attack_timer += read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Secondary);
                *attack_timer += read_data.dt.0;
            }
            // if target in close range use beam and shoot dagon bombs
        } else if attack_data.dist_sqrd < (3.0 * attack_data.min_attack_dist).powi(2) {
            controller.inputs.move_dir = Vec2::zero();
            if *attack_timer > 2.0 {
                controller.push_basic_input(InputKind::Ability(0));
                *attack_timer += read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Ability(1));
            }
        } else {
            // if target in midrange range shoot dagon bombs and heal
            if *attack_timer > 1.0 {
                controller.push_basic_input(InputKind::Ability(0));
                *attack_timer += read_data.dt.0;
            } else {
                controller.push_basic_input(InputKind::Ability(2));
                *attack_timer += read_data.dt.0;
            }
        }
    }

    pub fn handle_deadwood(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const BEAM_RANGE: f32 = 20.0;
        const BEAM_TIME: Duration = Duration::from_secs(3);
        // combat_state.condition controls whether or not deadwood should beam or dash
        if matches!(self.char_state, CharacterState::DashMelee(s) if s.stage_section != StageSection::Recover)
        {
            // If already dashing, keep dashing and have move_dir set to forward
            controller.push_basic_input(InputKind::Secondary);
            controller.inputs.move_dir = self.ori.look_vec().xy();
        } else if attack_data.in_min_range() && attack_data.angle_xy < 10.0 {
            // If near target, dash at them and through them to get away
            controller.push_basic_input(InputKind::Secondary);
        } else if matches!(self.char_state, CharacterState::BasicBeam(s) if s.stage_section != StageSection::Recover && s.timer < BEAM_TIME)
        {
            // If already beaming, keep beaming if not beaming for over 5 seconds
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd < BEAM_RANGE.powi(2) {
            // Else if in beam range, beam them
            if attack_data.angle_xy < 5.0 {
                controller.push_basic_input(InputKind::Primary);
            } else {
                // If not in angle, apply slight movement so deadwood orients itself correctly
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .try_normalized()
                    .unwrap_or_else(Vec2::zero)
                    * 0.01;
            }
        } else {
            // Otherwise too far, move towards target
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_mandragora(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const SCREAM_RANGE: f32 = 10.0; // hard-coded from scream.ron

        enum ActionStateFCounters {
            FCounterHealthThreshold = 0,
        }

        enum ActionStateConditions {
            ConditionHasScreamed = 0,
        }

        if !agent.combat_state.initialized {
            agent.combat_state.counters[ActionStateFCounters::FCounterHealthThreshold as usize] =
                self.health.map_or(0.0, |h| h.maximum());
            agent.combat_state.initialized = true;
        }

        if !agent.combat_state.conditions[ActionStateConditions::ConditionHasScreamed as usize] {
            // If mandragora is still "sleeping" and hasn't screamed yet, do nothing until
            // target in range or until it's taken damage
            if self.health.is_some_and(|h| {
                h.current()
                    < agent.combat_state.counters
                        [ActionStateFCounters::FCounterHealthThreshold as usize]
            }) || attack_data.dist_sqrd < SCREAM_RANGE.powi(2)
            {
                agent.combat_state.conditions
                    [ActionStateConditions::ConditionHasScreamed as usize] = true;
                controller.push_basic_input(InputKind::Secondary);
            }
        } else {
            // Once mandragora has woken, move towards target and attack
            if attack_data.in_min_range() {
                controller.push_basic_input(InputKind::Primary);
            } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2)
                && entities_have_line_of_sight(
                    self.pos,
                    self.body,
                    self.scale,
                    tgt_data.pos,
                    tgt_data.body,
                    tgt_data.scale,
                    read_data,
                )
            {
                // If in pathing range and can see target, move towards them
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Partial,
                    None,
                );
            } else {
                // Otherwise, go back to sleep
                agent.combat_state.conditions
                    [ActionStateConditions::ConditionHasScreamed as usize] = false;
                agent.combat_state.counters
                    [ActionStateFCounters::FCounterHealthThreshold as usize] =
                    self.health.map_or(0.0, |h| h.maximum());
            }
        }
    }

    pub fn handle_wood_golem(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        // === reference ===

        // Inputs:
        //   Primary: strike
        //   Secondary: spin
        //   Auxiliary
        //     0: shockwave

        // === setup ===

        // --- static ---
        // behaviour parameters
        const PATH_RANGE_FACTOR: f32 = 0.3; // get comfortably in range, but give player room to breathe
        const STRIKE_RANGE_FACTOR: f32 = 0.6; // start attack while suitably in range
        const STRIKE_AIM_FACTOR: f32 = 0.7;
        const SPIN_RANGE_FACTOR: f32 = 0.6;
        const SPIN_COOLDOWN: f32 = 1.5;
        const SPIN_RELAX_FACTOR: f32 = 0.2;
        const SHOCKWAVE_RANGE_FACTOR: f32 = 0.7;
        const SHOCKWAVE_AIM_FACTOR: f32 = 0.4;
        const SHOCKWAVE_COOLDOWN: f32 = 5.0;
        const MIXUP_COOLDOWN: f32 = 2.5;
        const MIXUP_RELAX_FACTOR: f32 = 0.3;

        // timers
        const SPIN: usize = 0;
        const SHOCKWAVE: usize = 1;
        const MIXUP: usize = 2;

        // --- dynamic ---
        // behaviour parameters
        let shockwave_min_range = self.body.map_or(0.0, |b| b.height() * 1.1);

        // attack data
        let (strike_range, strike_angle) = {
            if let Some(AbilityData::BasicMelee { range, angle, .. }) =
                self.extract_ability(AbilityInput::Primary)
            {
                (range, angle)
            } else {
                (0.0, 0.0)
            }
        };
        let spin_range = {
            if let Some(AbilityData::BasicMelee { range, .. }) =
                self.extract_ability(AbilityInput::Secondary)
            {
                range
            } else {
                0.0
            }
        };
        let (shockwave_max_range, shockwave_angle) = {
            if let Some(AbilityData::Shockwave { range, angle, .. }) =
                self.extract_ability(AbilityInput::Auxiliary(0))
            {
                (range, angle)
            } else {
                (0.0, 0.0)
            }
        };

        // re-used checks (makes separating timers and attacks easier)
        let is_in_spin_range = attack_data.dist_sqrd
            < (attack_data.body_dist + spin_range * SPIN_RANGE_FACTOR).powi(2);
        let is_in_strike_range = attack_data.dist_sqrd
            < (attack_data.body_dist + strike_range * STRIKE_RANGE_FACTOR).powi(2);
        let is_in_strike_angle = attack_data.angle < strike_angle * STRIKE_AIM_FACTOR;

        // === main ===

        // --- timers ---
        // spin
        let current_input = self.char_state.ability_info().map(|ai| ai.input);
        if matches!(current_input, Some(InputKind::Secondary)) {
            // reset when spinning
            agent.combat_state.timers[SPIN] = 0.0;
            agent.combat_state.timers[MIXUP] = 0.0;
        } else if is_in_spin_range && !(is_in_strike_range && is_in_strike_angle) {
            // increment within spin range and not in strike range + angle
            agent.combat_state.timers[SPIN] += read_data.dt.0;
        } else {
            // relax towards zero otherwise
            agent.combat_state.timers[SPIN] =
                (agent.combat_state.timers[SPIN] - read_data.dt.0 * SPIN_RELAX_FACTOR).max(0.0);
        }
        // shockwave
        if matches!(self.char_state, CharacterState::Shockwave(_)) {
            // reset when using shockwave
            agent.combat_state.timers[SHOCKWAVE] = 0.0;
            agent.combat_state.timers[MIXUP] = 0.0;
        } else {
            // increment otherwise
            agent.combat_state.timers[SHOCKWAVE] += read_data.dt.0;
        }
        // mixup
        if is_in_strike_range && is_in_strike_angle {
            // increment within strike range and angle
            agent.combat_state.timers[MIXUP] += read_data.dt.0;
        } else {
            // relax towards zero otherwise
            agent.combat_state.timers[MIXUP] =
                (agent.combat_state.timers[MIXUP] - read_data.dt.0 * MIXUP_RELAX_FACTOR).max(0.0);
        }

        // --- attacks ---
        // strike range and angle
        if is_in_strike_range && is_in_strike_angle {
            // on timer, randomly mixup between all attacks
            if agent.combat_state.timers[MIXUP] > MIXUP_COOLDOWN {
                let randomise: u8 = rng.gen_range(1..=3);
                match randomise {
                    1 => controller.push_basic_input(InputKind::Ability(0)), // shockwave
                    2 => controller.push_basic_input(InputKind::Primary),    // strike
                    _ => controller.push_basic_input(InputKind::Secondary),  // spin
                }
            }
            // default to strike
            else {
                controller.push_basic_input(InputKind::Primary);
            }
        }
        // spin range (or out of angle in strike range)
        else if is_in_spin_range || (is_in_strike_range && !is_in_strike_angle) {
            // on timer, use spin attack to try and hit evasive target
            if agent.combat_state.timers[SPIN] > SPIN_COOLDOWN {
                controller.push_basic_input(InputKind::Secondary);
            }
            // otherwise, close angle (no action required)
        }
        // shockwave range and angle
        else if attack_data.dist_sqrd > shockwave_min_range.powi(2)
            && attack_data.dist_sqrd < (shockwave_max_range * SHOCKWAVE_RANGE_FACTOR).powi(2)
            && attack_data.angle < shockwave_angle * SHOCKWAVE_AIM_FACTOR
        {
            // on timer, use shockwave
            if agent.combat_state.timers[SHOCKWAVE] > SHOCKWAVE_COOLDOWN {
                controller.push_basic_input(InputKind::Ability(0));
            }
            // otherwise, close gap and/or angle (no action required)
        }

        // --- movement ---
        // closing gap
        if attack_data.dist_sqrd
            > (attack_data.body_dist + strike_range * PATH_RANGE_FACTOR).powi(2)
        {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
        // closing angle
        else if attack_data.angle > 0.0 {
            // some movement is required to trigger re-orientation
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::zero)
                * 0.001; // scaled way down to minimise position change and keep close rotation consistent
        }
    }

    pub fn handle_gnarling_chieftain(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        // === reference ===
        // Inputs
        //   Primary: flamestrike
        //   Secondary: firebarrage
        //   Auxiliary
        //     0: fireshockwave
        //     1: redtotem
        //     2: greentotem
        //     3: whitetotem

        // === setup ===

        // --- static ---
        // behaviour parameters
        const PATH_RANGE_FACTOR: f32 = 0.4;
        const STRIKE_RANGE_FACTOR: f32 = 0.7;
        const STRIKE_AIM_FACTOR: f32 = 0.8;
        const BARRAGE_RANGE_FACTOR: f32 = 0.8;
        const BARRAGE_AIM_FACTOR: f32 = 0.65;
        const SHOCKWAVE_RANGE_FACTOR: f32 = 0.75;
        const TOTEM_COOLDOWN: f32 = 25.0;
        const HEAVY_ATTACK_COOLDOWN_SPAN: [f32; 2] = [8.0, 13.0];
        const HEAVY_ATTACK_CHARGE_FACTOR: f32 = 3.3;
        const HEAVY_ATTACK_FAST_CHARGE_FACTOR: f32 = 5.0;

        // conditions
        const HAS_SUMMONED_FIRST_TOTEM: usize = 0;
        // timers
        const SUMMON_TOTEM: usize = 0;
        const HEAVY_ATTACK: usize = 1;
        // counters
        const HEAVY_ATTACK_COOLDOWN: usize = 0;

        // line of sight check
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };

        // --- dynamic ---
        // attack data
        let (strike_range, strike_angle) = {
            if let Some(AbilityData::BasicMelee { range, angle, .. }) =
                self.extract_ability(AbilityInput::Primary)
            {
                (range, angle)
            } else {
                (0.0, 0.0)
            }
        };
        let (barrage_speed, barrage_spread, barrage_count) = {
            if let Some(AbilityData::BasicRanged {
                projectile_speed,
                projectile_spread,
                num_projectiles,
                ..
            }) = self.extract_ability(AbilityInput::Secondary)
            {
                (
                    projectile_speed,
                    projectile_spread,
                    num_projectiles.compute(self.heads.map_or(1, |heads| heads.amount() as u32)),
                )
            } else {
                (0.0, 0.0, 0)
            }
        };
        let shockwave_range = {
            if let Some(AbilityData::Shockwave { range, .. }) =
                self.extract_ability(AbilityInput::Auxiliary(0))
            {
                range
            } else {
                0.0
            }
        };

        // calculated attack data
        let barrage_max_range =
            projectile_flat_range(barrage_speed, self.body.map_or(2.0, |b| b.height()));
        let barrange_angle = projectile_multi_angle(barrage_spread, barrage_count);

        // re-used checks
        let is_in_strike_range = attack_data.dist_sqrd
            < (attack_data.body_dist + strike_range * STRIKE_RANGE_FACTOR).powi(2);
        let is_in_strike_angle = attack_data.angle < strike_angle * STRIKE_AIM_FACTOR;

        // initialise randomised cooldowns
        if !agent.combat_state.initialized {
            agent.combat_state.initialized = true;
            agent.combat_state.counters[HEAVY_ATTACK_COOLDOWN] =
                rng_from_span(rng, HEAVY_ATTACK_COOLDOWN_SPAN);
        }

        // === main ===

        // --- timers ---
        // resets
        match self.char_state {
            CharacterState::BasicSummon(s) if s.stage_section == StageSection::Recover => {
                // reset when finished summoning
                agent.combat_state.timers[SUMMON_TOTEM] = 0.0;
                agent.combat_state.conditions[HAS_SUMMONED_FIRST_TOTEM] = true;
            },
            CharacterState::Shockwave(_) | CharacterState::BasicRanged(_) => {
                // reset heavy attack on either ability
                agent.combat_state.counters[HEAVY_ATTACK] = 0.0;
                agent.combat_state.counters[HEAVY_ATTACK_COOLDOWN] =
                    rng_from_span(rng, HEAVY_ATTACK_COOLDOWN_SPAN);
            },
            _ => {},
        }
        // totem (always increment)
        agent.combat_state.timers[SUMMON_TOTEM] += read_data.dt.0;
        // heavy attack (increment at different rates)
        if is_in_strike_range {
            // recharge at standard rate in strike range and angle
            if is_in_strike_angle {
                agent.combat_state.counters[HEAVY_ATTACK] += read_data.dt.0;
            } else {
                // If not in angle, charge heavy attack faster
                agent.combat_state.counters[HEAVY_ATTACK] +=
                    read_data.dt.0 * HEAVY_ATTACK_FAST_CHARGE_FACTOR;
            }
        } else {
            // If not in range, charge heavy attack faster
            agent.combat_state.counters[HEAVY_ATTACK] +=
                read_data.dt.0 * HEAVY_ATTACK_CHARGE_FACTOR;
        }

        // --- attacks ---
        // start by summoning green totem
        if !agent.combat_state.conditions[HAS_SUMMONED_FIRST_TOTEM] {
            controller.push_basic_input(InputKind::Ability(2));
        }
        // on timer, summon a new random totem
        else if agent.combat_state.timers[SUMMON_TOTEM] > TOTEM_COOLDOWN {
            controller.push_basic_input(InputKind::Ability(rng.gen_range(1..=3)));
        }
        // on timer and in range, use a heavy attack
        // assumes: barrange_max_range * BARRAGE_RANGE_FACTOR > shockwave_range *
        // SHOCKWAVE_RANGE_FACTOR
        else if agent.combat_state.counters[HEAVY_ATTACK]
            > agent.combat_state.counters[HEAVY_ATTACK_COOLDOWN]
            && attack_data.dist_sqrd < (barrage_max_range * BARRAGE_RANGE_FACTOR).powi(2)
        {
            // has line of sight
            if line_of_sight_with_target() {
                // out of barrage angle, use shockwave
                if attack_data.angle > barrange_angle * BARRAGE_AIM_FACTOR {
                    controller.push_basic_input(InputKind::Ability(0));
                }
                // in shockwave range, randomise between barrage and shockwave
                else if attack_data.dist_sqrd < (shockwave_range * SHOCKWAVE_RANGE_FACTOR).powi(2)
                {
                    if rng.gen_bool(0.5) {
                        controller.push_basic_input(InputKind::Secondary);
                    } else {
                        controller.push_basic_input(InputKind::Ability(0));
                    }
                }
                // in range and angle, use barrage
                else {
                    controller.push_basic_input(InputKind::Secondary);
                }
                // otherwise, close gap and/or angle (no action required)
            }
            // no line of sight
            else {
                //  in range, use shockwave
                if attack_data.dist_sqrd < (shockwave_range * SHOCKWAVE_RANGE_FACTOR).powi(2) {
                    controller.push_basic_input(InputKind::Ability(0));
                }
                // otherwise, close gap (no action required)
            }
        }
        // if viable, default to flamestrike
        else if is_in_strike_range && is_in_strike_angle {
            controller.push_basic_input(InputKind::Primary);
        }
        // otherwise, close gap and/or angle (no action required)

        // --- movement ---
        // closing gap
        if attack_data.dist_sqrd
            > (attack_data.body_dist + strike_range * PATH_RANGE_FACTOR).powi(2)
        {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
        // closing angle
        else if attack_data.angle > 0.0 {
            // some movement is required to trigger re-orientation
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::zero)
                * 0.001; // scaled way down to minimise position change and keep close rotation consistent
        }
    }

    pub fn handle_sword_simple_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const DASH_TIMER: usize = 0;
        agent.combat_state.timers[DASH_TIMER] += read_data.dt.0;
        if matches!(self.char_state, CharacterState::DashMelee(s) if !matches!(s.stage_section, StageSection::Recover))
        {
            controller.push_basic_input(InputKind::Secondary);
        } else if attack_data.in_min_range() && attack_data.angle < 45.0 {
            if agent.combat_state.timers[DASH_TIMER] > 2.0 {
                agent.combat_state.timers[DASH_TIMER] = 0.0;
            }
            controller.push_basic_input(InputKind::Primary);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2)
            && self
                .path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    None,
                )
                .is_some()
            && entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
            && agent.combat_state.timers[DASH_TIMER] > 4.0
            && attack_data.angle < 45.0
        {
            controller.push_basic_input(InputKind::Secondary);
            agent.combat_state.timers[DASH_TIMER] = 0.0;
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_adlet_hunter(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const ROTATE_TIMER: usize = 0;
        const ROTATE_DIR_CONDITION: usize = 0;
        agent.combat_state.timers[ROTATE_TIMER] -= read_data.dt.0;
        if agent.combat_state.timers[ROTATE_TIMER] < 0.0 {
            agent.combat_state.conditions[ROTATE_DIR_CONDITION] = rng.gen_bool(0.5);
            agent.combat_state.timers[ROTATE_TIMER] = rng.gen::<f32>() * 5.0;
        }
        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };
        let move_forwards = if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else if could_use_input(InputKind::Secondary) && attack_data.dist_sqrd > 8_f32.powi(2) {
            controller.push_basic_input(InputKind::Secondary);
            true
        } else {
            true
        };

        if move_forwards && attack_data.dist_sqrd > 3_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
            let dir = if agent.combat_state.conditions[ROTATE_DIR_CONDITION] {
                1.0
            } else {
                -1.0
            };
            controller.inputs.move_dir.rotate_z(PI / 2.0 * dir);
        }
    }

    pub fn handle_adlet_icepicker(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };
        let move_forwards = if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else if could_use_input(InputKind::Secondary) && attack_data.dist_sqrd > 5_f32.powi(2) {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else {
            true
        };

        if move_forwards && attack_data.dist_sqrd > 2_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_adlet_tracker(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const TRAP_TIMER: usize = 0;
        agent.combat_state.timers[TRAP_TIMER] += read_data.dt.0;
        if agent.combat_state.timers[TRAP_TIMER] > 20.0 {
            agent.combat_state.timers[TRAP_TIMER] = 0.0;
        }
        let primary = self.extract_ability(AbilityInput::Primary);
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };
        let move_forwards = if agent.combat_state.timers[TRAP_TIMER] < 3.0 {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else {
            true
        };

        if move_forwards && attack_data.dist_sqrd > 2_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_adlet_elder(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const TRAP_TIMER: usize = 0;
        agent.combat_state.timers[TRAP_TIMER] -= read_data.dt.0;
        if matches!(self.char_state, CharacterState::BasicRanged(_)) {
            agent.combat_state.timers[TRAP_TIMER] = 15.0;
        }
        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let abilities = [
            self.extract_ability(AbilityInput::Auxiliary(0)),
            self.extract_ability(AbilityInput::Auxiliary(1)),
        ];
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                a.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };
        let move_forwards = if matches!(self.char_state, CharacterState::DashMelee(s) if s.stage_section != StageSection::Recover)
        {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if agent.combat_state.timers[TRAP_TIMER] < 0.0 && !tgt_data.considered_ranged() {
            controller.push_basic_input(InputKind::Ability(0));
            false
        } else if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else if could_use_input(InputKind::Secondary) && rng.gen_bool(0.5) {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if could_use_input(InputKind::Ability(1)) {
            controller.push_basic_input(InputKind::Ability(1));
            false
        } else {
            true
        };

        if matches!(self.char_state, CharacterState::LeapMelee(_)) {
            let tgt_vec = tgt_data.pos.0.xy() - self.pos.0.xy();
            if tgt_vec.magnitude_squared() > 2_f32.powi(2) {
                if let Some(look_dir) = Dir::from_unnormalized(Vec3::from(tgt_vec)) {
                    controller.inputs.look_dir = look_dir;
                }
            }
        }

        if move_forwards && attack_data.dist_sqrd > 2_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_icedrake(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let abilities = [
            self.extract_ability(AbilityInput::Auxiliary(0)),
            self.extract_ability(AbilityInput::Auxiliary(1)),
        ];
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                a.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };

        let continued_attack = match self.char_state.ability_info().map(|ai| ai.input) {
            Some(input @ InputKind::Primary) => {
                if !matches!(self.char_state.stage_section(), Some(StageSection::Recover))
                    && could_use_input(input)
                {
                    controller.push_basic_input(input);
                    true
                } else {
                    false
                }
            },
            Some(input @ InputKind::Ability(1)) => {
                if self
                    .char_state
                    .timer()
                    .is_some_and(|t| t.as_secs_f32() < 3.0)
                    && could_use_input(input)
                {
                    controller.push_basic_input(input);
                    true
                } else {
                    false
                }
            },
            _ => false,
        };

        let move_forwards = if !continued_attack {
            if could_use_input(InputKind::Primary) && rng.gen_bool(0.4) {
                controller.push_basic_input(InputKind::Primary);
                false
            } else if could_use_input(InputKind::Secondary) && rng.gen_bool(0.8) {
                controller.push_basic_input(InputKind::Secondary);
                false
            } else if could_use_input(InputKind::Ability(1)) && rng.gen_bool(0.9) {
                controller.push_basic_input(InputKind::Ability(1));
                true
            } else if could_use_input(InputKind::Ability(0)) {
                controller.push_basic_input(InputKind::Ability(0));
                true
            } else {
                true
            }
        } else {
            false
        };

        if move_forwards {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_hydra(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        enum ActionStateTimers {
            RegrowHeadNoDamage,
            RegrowHeadNoAttack,
        }

        let could_use_input = |input| {
            Option::from(input)
                .and_then(|ability| {
                    Some(self.extract_ability(ability)?.could_use(
                        attack_data,
                        self,
                        tgt_data,
                        read_data,
                        AbilityPreferences::default(),
                    ))
                })
                .unwrap_or(false)
        };

        const FOCUS_ATTACK_RANGE: f32 = 5.0;

        if attack_data.dist_sqrd < FOCUS_ATTACK_RANGE.powi(2) {
            agent.combat_state.timers[ActionStateTimers::RegrowHeadNoAttack as usize] = 0.0;
        } else {
            agent.combat_state.timers[ActionStateTimers::RegrowHeadNoAttack as usize] +=
                read_data.dt.0;
        }

        if let Some(health) = self.health.filter(|health| health.last_change.amount < 0.0) {
            agent.combat_state.timers[ActionStateTimers::RegrowHeadNoDamage as usize] =
                (read_data.time.0 - health.last_change.time.0) as f32;
        } else {
            agent.combat_state.timers[ActionStateTimers::RegrowHeadNoDamage as usize] +=
                read_data.dt.0;
        }

        if let Some(input) = self.char_state.ability_info().map(|ai| ai.input) {
            match self.char_state {
                CharacterState::ChargedMelee(c) => {
                    if c.charge_frac() < 1.0 && could_use_input(input) {
                        controller.push_basic_input(input);
                    }
                },
                CharacterState::ChargedRanged(c) => {
                    if c.charge_frac() < 1.0 && could_use_input(input) {
                        controller.push_basic_input(input);
                    }
                },
                _ => {},
            }
        }

        let continued_attack = match self.char_state.ability_info().map(|ai| ai.input) {
            Some(input @ InputKind::Primary) => {
                if !matches!(self.char_state.stage_section(), Some(StageSection::Recover))
                    && could_use_input(input)
                {
                    controller.push_basic_input(input);
                    true
                } else {
                    false
                }
            },
            _ => false,
        };

        let has_heads = self.heads.is_none_or(|heads| heads.amount() > 0);

        let move_forwards = if !continued_attack {
            if could_use_input(InputKind::Ability(1))
                && rng.gen_bool(0.9)
                && (agent.combat_state.timers[ActionStateTimers::RegrowHeadNoDamage as usize] > 5.0
                    || agent.combat_state.timers[ActionStateTimers::RegrowHeadNoAttack as usize]
                        > 6.0)
                && self.heads.is_some_and(|heads| heads.amount_missing() > 0)
            {
                controller.push_basic_input(InputKind::Ability(2));
                false
            } else if has_heads && could_use_input(InputKind::Primary) && rng.gen_bool(0.8) {
                controller.push_basic_input(InputKind::Primary);
                true
            } else if has_heads && could_use_input(InputKind::Secondary) && rng.gen_bool(0.4) {
                controller.push_basic_input(InputKind::Secondary);
                false
            } else if has_heads && could_use_input(InputKind::Ability(1)) && rng.gen_bool(0.6) {
                controller.push_basic_input(InputKind::Ability(1));
                true
            } else if !has_heads && could_use_input(InputKind::Ability(3)) && rng.gen_bool(0.7) {
                controller.push_basic_input(InputKind::Ability(3));
                true
            } else if could_use_input(InputKind::Ability(0)) {
                controller.push_basic_input(InputKind::Ability(0));
                true
            } else {
                true
            }
        } else {
            true
        };

        if move_forwards {
            if has_heads {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    // Slow down if close to the target
                    (attack_data.dist_sqrd
                        < (2.5 + self.body.map_or(0.0, |b| b.front_radius())).powi(2))
                    .then_some(0.3),
                );
            } else {
                self.flee(agent, controller, read_data, tgt_data.pos);
            }
        }
    }

    pub fn handle_random_abilities(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
        primary_weight: u8,
        secondary_weight: u8,
        ability_weights: [u8; BASE_ABILITY_LIMIT],
    ) {
        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let abilities = [
            self.extract_ability(AbilityInput::Auxiliary(0)),
            self.extract_ability(AbilityInput::Auxiliary(1)),
            self.extract_ability(AbilityInput::Auxiliary(2)),
            self.extract_ability(AbilityInput::Auxiliary(3)),
            self.extract_ability(AbilityInput::Auxiliary(4)),
        ];
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                a.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };

        let primary_chance = primary_weight as f64
            / ((primary_weight + secondary_weight + ability_weights.iter().sum::<u8>()) as f64)
                .max(0.01);
        let secondary_chance = secondary_weight as f64
            / ((secondary_weight + ability_weights.iter().sum::<u8>()) as f64).max(0.01);
        let ability_chances = {
            let mut chances = [0.0; BASE_ABILITY_LIMIT];
            chances.iter_mut().enumerate().for_each(|(i, chance)| {
                *chance = ability_weights[i] as f64
                    / (ability_weights
                        .iter()
                        .enumerate()
                        .filter_map(|(j, weight)| if j >= i { Some(weight) } else { None })
                        .sum::<u8>() as f64)
                        .max(0.01)
            });
            chances
        };

        if let Some(input) = self.char_state.ability_info().map(|ai| ai.input) {
            match self.char_state {
                CharacterState::ChargedMelee(c) => {
                    if c.charge_frac() < 1.0 && could_use_input(input) {
                        controller.push_basic_input(input);
                    }
                },
                CharacterState::ChargedRanged(c) => {
                    if c.charge_frac() < 1.0 && could_use_input(input) {
                        controller.push_basic_input(input);
                    }
                },
                _ => {},
            }
        }

        let move_forwards = if could_use_input(InputKind::Primary) && rng.gen_bool(primary_chance) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else if could_use_input(InputKind::Secondary) && rng.gen_bool(secondary_chance) {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if could_use_input(InputKind::Ability(0)) && rng.gen_bool(ability_chances[0]) {
            controller.push_basic_input(InputKind::Ability(0));
            false
        } else if could_use_input(InputKind::Ability(1)) && rng.gen_bool(ability_chances[1]) {
            controller.push_basic_input(InputKind::Ability(1));
            false
        } else if could_use_input(InputKind::Ability(2)) && rng.gen_bool(ability_chances[2]) {
            controller.push_basic_input(InputKind::Ability(2));
            false
        } else if could_use_input(InputKind::Ability(3)) && rng.gen_bool(ability_chances[3]) {
            controller.push_basic_input(InputKind::Ability(3));
            false
        } else if could_use_input(InputKind::Ability(4)) && rng.gen_bool(ability_chances[4]) {
            controller.push_basic_input(InputKind::Ability(4));
            false
        } else {
            true
        };

        if move_forwards {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_simple_double_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MAX_ATTACK_RANGE: f32 = 20.0;

        if attack_data.angle < 60.0 && attack_data.dist_sqrd < MAX_ATTACK_RANGE.powi(2) {
            controller.inputs.move_dir = Vec2::zero();
            if attack_data.in_min_range() {
                controller.push_basic_input(InputKind::Primary);
            } else {
                controller.push_basic_input(InputKind::Secondary);
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Partial,
                None,
            );
        }
    }

    pub fn handle_clay_steed_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            AttackTimer,
        }
        const HOOF_ATTACK_RANGE: f32 = 1.0;
        const HOOF_ATTACK_ANGLE: f32 = 50.0;

        agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] > 10.0 {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
        }

        if attack_data.angle < HOOF_ATTACK_ANGLE
            && attack_data.dist_sqrd
                < (HOOF_ATTACK_RANGE + self.body.map_or(0.0, |b| b.max_radius())).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller.push_basic_input(InputKind::Primary);
        } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 5.0 {
            controller.push_basic_input(InputKind::Secondary);
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Full,
                None,
            );
        }
    }

    pub fn handle_ancient_effigy_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        enum ActionStateTimers {
            BlastTimer,
        }

        let home = agent.patrol_origin.unwrap_or(self.pos.0);
        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        agent.combat_state.timers[ActionStateTimers::BlastTimer as usize] += read_data.dt.0;

        if agent.combat_state.timers[ActionStateTimers::BlastTimer as usize] > 6.0 {
            agent.combat_state.timers[ActionStateTimers::BlastTimer as usize] = 0.0;
        }
        if line_of_sight_with_target() {
            if attack_data.in_min_range() {
                controller.push_basic_input(InputKind::Secondary);
            } else if agent.combat_state.timers[ActionStateTimers::BlastTimer as usize] < 2.0 {
                controller.push_basic_input(InputKind::Primary);
            } else {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Separate,
                    None,
                );
            }
        } else {
            // if target is hiding, don't follow, guard the room
            if (home - self.pos.0).xy().magnitude_squared() > (3.0_f32).powi(2) {
                self.path_toward_target(agent, controller, home, read_data, Path::Separate, None);
            }
        }
    }

    pub fn handle_clay_golem_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MIN_DASH_RANGE: f32 = 15.0;

        enum ActionStateTimers {
            AttackTimer,
        }

        let line_of_sight_with_target = || {
            entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            )
        };
        let spawn = agent.patrol_origin.unwrap_or(self.pos.0);
        let home = Vec3::new(spawn.x - 32.0, spawn.y - 12.0, spawn.z);
        let is_home = (home - self.pos.0).xy().magnitude_squared() < (3.0_f32).powi(2);
        agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] += read_data.dt.0;
        if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] > 8.0 {
            // Reset timer
            agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] = 0.0;
        }
        if line_of_sight_with_target() {
            controller.inputs.move_dir = Vec2::zero();
            if attack_data.in_min_range() {
                controller.push_basic_input(InputKind::Primary);
            } else if attack_data.dist_sqrd > MIN_DASH_RANGE.powi(2) {
                controller.push_basic_input(InputKind::Secondary);
            } else {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::Partial,
                    None,
                );
            }
        } else if agent.combat_state.timers[ActionStateTimers::AttackTimer as usize] < 4.0 {
            if !is_home {
                // if target is wall cheesing, reposition
                self.path_toward_target(agent, controller, home, read_data, Path::Separate, None);
            } else {
                self.path_toward_target(agent, controller, spawn, read_data, Path::Separate, None);
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_haniwa_soldier(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const DEFENSIVE_CONDITION: usize = 0;
        const RIPOSTE_TIMER: usize = 0;
        const MODE_CYCLE_TIMER: usize = 1;

        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };

        agent.combat_state.timers[RIPOSTE_TIMER] += read_data.dt.0;
        agent.combat_state.timers[MODE_CYCLE_TIMER] += read_data.dt.0;

        if agent.combat_state.timers[MODE_CYCLE_TIMER] > 7.0 {
            agent.combat_state.conditions[DEFENSIVE_CONDITION] =
                !agent.combat_state.conditions[DEFENSIVE_CONDITION];
            agent.combat_state.timers[MODE_CYCLE_TIMER] = 0.0;
        }

        if matches!(self.char_state, CharacterState::RiposteMelee(_)) {
            agent.combat_state.timers[RIPOSTE_TIMER] = 0.0;
        }

        let try_move = if agent.combat_state.conditions[DEFENSIVE_CONDITION] {
            controller.push_basic_input(InputKind::Block);
            true
        } else if agent.combat_state.timers[RIPOSTE_TIMER] > 10.0
            && could_use_input(InputKind::Secondary)
        {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else {
            true
        };

        if try_move && attack_data.dist_sqrd > 2_f32.powi(2) {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_haniwa_guard(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        const BACKPEDAL_DIST: f32 = 5.0;
        const ROTATE_CCW_CONDITION: usize = 0;
        const FLURRY_TIMER: usize = 0;
        const BACKPEDAL_TIMER: usize = 1;
        const SWITCH_ROTATE_TIMER: usize = 2;
        const SWITCH_ROTATE_COUNTER: usize = 0;

        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let abilities = [self.extract_ability(AbilityInput::Auxiliary(0))];
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                a.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };

        if !agent.combat_state.initialized {
            agent.combat_state.conditions[ROTATE_CCW_CONDITION] = rng.gen_bool(0.5);
            agent.combat_state.counters[SWITCH_ROTATE_COUNTER] = rng.gen_range(5.0..20.0);
            agent.combat_state.initialized = true;
        }

        let continue_flurry = match self.char_state {
            CharacterState::BasicMelee(_) => {
                agent.combat_state.timers[FLURRY_TIMER] += read_data.dt.0;
                false
            },
            CharacterState::RapidMelee(c) => {
                agent.combat_state.timers[FLURRY_TIMER] = 0.0;
                !matches!(c.stage_section, StageSection::Recover)
            },
            CharacterState::ComboMelee2(_) => {
                agent.combat_state.timers[BACKPEDAL_TIMER] = 0.0;
                false
            },
            _ => false,
        };
        agent.combat_state.timers[SWITCH_ROTATE_TIMER] += read_data.dt.0;
        agent.combat_state.timers[BACKPEDAL_TIMER] += read_data.dt.0;

        if agent.combat_state.timers[SWITCH_ROTATE_TIMER]
            > agent.combat_state.counters[SWITCH_ROTATE_COUNTER]
        {
            agent.combat_state.conditions[ROTATE_CCW_CONDITION] =
                !agent.combat_state.conditions[ROTATE_CCW_CONDITION];
            agent.combat_state.counters[SWITCH_ROTATE_COUNTER] = rng.gen_range(5.0..20.0);
        }

        let move_farther = attack_data.dist_sqrd < BACKPEDAL_DIST.powi(2);
        let move_closer = if continue_flurry && could_use_input(InputKind::Secondary) {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if agent.combat_state.timers[BACKPEDAL_TIMER] > 10.0
            && move_farther
            && could_use_input(InputKind::Ability(0))
        {
            controller.push_basic_input(InputKind::Ability(0));
            false
        } else if agent.combat_state.timers[FLURRY_TIMER] > 6.0
            && could_use_input(InputKind::Secondary)
        {
            controller.push_basic_input(InputKind::Secondary);
            false
        } else if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
            false
        } else {
            true
        };

        if let Some((bearing, speed)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            tgt_data.pos.0,
            TraversalConfig {
                min_tgt_dist: 1.25,
                ..self.traversal_config
            },
        ) {
            if entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                tgt_data.pos,
                tgt_data.body,
                tgt_data.scale,
                read_data,
            ) && attack_data.angle < 45.0
            {
                let angle = match (
                    agent.combat_state.conditions[ROTATE_CCW_CONDITION],
                    move_closer,
                    move_farther,
                ) {
                    (true, true, false) => rng.gen_range(-1.5..-0.5),
                    (true, false, true) => rng.gen_range(-2.2..-1.7),
                    (true, _, _) => rng.gen_range(-1.7..-1.5),
                    (false, true, false) => rng.gen_range(0.5..1.5),
                    (false, false, true) => rng.gen_range(1.7..2.2),
                    (false, _, _) => rng.gen_range(1.5..1.7),
                };
                controller.inputs.move_dir = bearing
                    .xy()
                    .rotated_z(angle)
                    .try_normalized()
                    .unwrap_or_else(Vec2::zero)
                    * speed;
            } else {
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                self.jump_if(bearing.z > 1.5, controller);
            }
        }
    }

    pub fn handle_haniwa_archer(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const KICK_TIMER: usize = 0;
        const EXPLOSIVE_TIMER: usize = 1;

        let primary = self.extract_ability(AbilityInput::Primary);
        let secondary = self.extract_ability(AbilityInput::Secondary);
        let abilities = [self.extract_ability(AbilityInput::Auxiliary(0))];
        let could_use_input = |input| match input {
            InputKind::Primary => primary.as_ref().is_some_and(|p| {
                p.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Secondary => secondary.as_ref().is_some_and(|s| {
                s.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            InputKind::Ability(x) => abilities[x].as_ref().is_some_and(|a| {
                a.could_use(
                    attack_data,
                    self,
                    tgt_data,
                    read_data,
                    AbilityPreferences::default(),
                )
            }),
            _ => false,
        };

        agent.combat_state.timers[KICK_TIMER] += read_data.dt.0;
        agent.combat_state.timers[EXPLOSIVE_TIMER] += read_data.dt.0;

        match self.char_state.ability_info().map(|ai| ai.input) {
            Some(InputKind::Secondary) => {
                agent.combat_state.timers[KICK_TIMER] = 0.0;
            },
            Some(InputKind::Ability(0)) => {
                agent.combat_state.timers[EXPLOSIVE_TIMER] = 0.0;
            },
            _ => {},
        }

        if agent.combat_state.timers[KICK_TIMER] > 4.0 && could_use_input(InputKind::Secondary) {
            controller.push_basic_input(InputKind::Secondary);
        } else if agent.combat_state.timers[EXPLOSIVE_TIMER] > 15.0
            && could_use_input(InputKind::Ability(0))
        {
            controller.push_basic_input(InputKind::Ability(0));
        } else if could_use_input(InputKind::Primary) {
            controller.push_basic_input(InputKind::Primary);
        } else {
            self.path_toward_target(
                agent,
                controller,
                tgt_data.pos.0,
                read_data,
                Path::Separate,
                None,
            );
        }
    }

    pub fn handle_terracotta_statue_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        read_data: &ReadData,
    ) {
        enum Conditions {
            AttackToggle,
        }
        let home = agent.patrol_origin.unwrap_or(self.pos.0.round());
        // stay centered
        if (home - self.pos.0).xy().magnitude_squared() > (2.0_f32).powi(2) {
            self.path_toward_target(agent, controller, home, read_data, Path::Full, None);
        } else if !agent.combat_state.conditions[Conditions::AttackToggle as usize] {
            // always begin with sprite summon
            controller.push_basic_input(InputKind::Primary);
        } else {
            controller.inputs.move_dir = Vec2::zero();
            if attack_data.dist_sqrd < 8.5f32.powi(2) {
                // sprite summon
                controller.push_basic_input(InputKind::Primary);
            } else {
                // projectile
                controller.push_basic_input(InputKind::Secondary);
            }
        }
        if matches!(self.char_state, CharacterState::SpriteSummon(c) if matches!(c.stage_section, StageSection::Recover))
        {
            agent.combat_state.conditions[Conditions::AttackToggle as usize] = true;
        }
    }

    pub fn handle_jiangshi_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if tgt_data.pos.0.z - self.pos.0.z > 5.0 {
            controller.push_action(ControlAction::StartInput {
                input: InputKind::Secondary,
                target_entity: agent
                    .target
                    .as_ref()
                    .and_then(|t| read_data.uids.get(t.target))
                    .copied(),
                select_pos: None,
            });
        } else if attack_data.dist_sqrd < 12.0f32.powi(2) {
            controller.push_basic_input(InputKind::Primary);
        }

        self.path_toward_target(
            agent,
            controller,
            tgt_data.pos.0,
            read_data,
            Path::Full,
            None,
        );
    }
}

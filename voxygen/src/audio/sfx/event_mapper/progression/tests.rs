use super::*;
use common::event::SfxEvent;

#[test]
fn no_change_returns_none() {
    let mut mapper = ProgressionEventMapper::new();
    let next_client_state = ProgressionState::default();

    assert_eq!(mapper.map_event(&next_client_state), None);
}

#[test]
fn change_level_returns_levelup() {
    let mut mapper = ProgressionEventMapper::new();
    let next_client_state = ProgressionState { level: 2, exp: 0 };

    assert_eq!(
        mapper.map_event(&next_client_state),
        Some(SfxEvent::LevelUp)
    );
}

#[test]
fn change_exp_returns_expup() {
    let mut mapper = ProgressionEventMapper::new();
    let next_client_state = ProgressionState { level: 1, exp: 100 };

    assert_eq!(
        mapper.map_event(&next_client_state),
        Some(SfxEvent::ExperienceGained)
    );
}

#[test]
fn level_up_and_gained_exp_prioritises_levelup() {
    let mut mapper = ProgressionEventMapper::new();
    let next_client_state = ProgressionState { level: 2, exp: 100 };

    assert_eq!(
        mapper.map_event(&next_client_state),
        Some(SfxEvent::LevelUp)
    );
}

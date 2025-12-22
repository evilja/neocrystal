use pancurses::MEVENT;
#[cfg(feature = "mouse")]
use super::crystal_manager::{PLAY, SHUFFLE, LOOP};
use crate::modules::general::Action;
#[cfg(feature = "mouse")]
use crate::modules::curses::Ownership;
#[cfg(feature = "mouse")]
use crate::modules::curses::draw_rpc_indc;
use crate::modules::general::GeneralState;
#[cfg(feature = "mouse")]
use crate::modules::tui_ir::UI;
use pancurses::Input;
#[cfg(feature = "mouse")]
#[derive(Copy, Clone)]
pub struct MouseHit {
    pub owner: Ownership,
    pub local_x: usize,
    pub local_y: usize,
}
#[cfg(not(feature = "mouse"))]
pub fn handle_mouse(    
    _: MEVENT,
    _: &GeneralState
) -> Option<Action> {
    None
}
#[cfg(not(feature = "mouse"))]
pub fn action_to_key(
    _: Action,
    _: &mut GeneralState,
) -> Option<Input> {
    None
}

#[cfg(feature = "mouse")]
pub fn resolve_hit(
    ui: &UI<Ownership>,
    x: usize,
    y: usize,
) -> Option<MouseHit> {
    ui.get_ownership()
        .iter()
        .rev() 
        .find(|e| {
            x >= e.range_x.0 &&
            x <  e.range_x.0 + e.range_x.1 &&
            y >= e.range_y.0 &&
            y <  e.range_y.0 + e.range_y.1
        })
        .map(|e| MouseHit {
            owner: *e.get_id(),
            local_x: x - e.range_x.0,
            local_y: y - e.range_y.0,
        })
}
#[cfg(feature = "mouse")]
pub fn hit_to_action(
    hit: MouseHit,
    general: &GeneralState,
) -> Action {
    match hit.owner {
        Ownership::Songs => {
            let row = hit.local_y;
            Action::Play(general.index.page, row)
        }

        Ownership::SongInd => {
            let row = hit.local_y;
            Action::Play(general.index.page, row)
        }

        Ownership::Page => {
            if hit.local_x < 3 {
                Action::PgUp
            } else {
                Action::PgDown
            }
        }

        Ownership::ShuInd => Action::Shuffle,
        Ownership::LoopInd => Action::Repeat,
        Ownership::RpcInd => Action::Rpc,

        _ => Action::Nothing,
    }
}

#[cfg(feature = "mouse")]
pub fn handle_mouse(
    mevent: MEVENT,
    general: &GeneralState,
) -> Option<Action> {
    if mevent.bstate & 0x2 == 0 {
        return None;
    }

    let x = mevent.x as usize;
    let y = mevent.y as usize;

    let hit = resolve_hit(&general.ui, x, y)?;
    Some(hit_to_action(hit, general))
}
#[cfg(feature = "mouse")]
pub fn action_to_key(
    action: Action,
    general: &mut GeneralState,
) -> Option<Input> {
    match action {
        Action::Play(page, row) => {
            general.index.page  = page;
            general.index.index = row;
            Some(Input::Character(PLAY))
        }

        Action::Shuffle => Some(Input::Character(SHUFFLE)),
        Action::Repeat  => Some(Input::Character(LOOP)),
        Action::Rpc     => {
            general.rpc.renew();
            draw_rpc_indc(general);
            None
        }

        Action::PgDown => Some(Input::KeyNPage),
        Action::PgUp   => Some(Input::KeyPPage),

        Action::Nothing => None,
    }
}

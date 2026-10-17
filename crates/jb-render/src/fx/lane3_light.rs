//! Flashes, glows and light washes fitted on Filmora's exports with the
//! two-pass flash recipe (`kit::FlashFx`): per frame a zoom of the current
//! clip, a mix with its own gaussian bloom and an additive wash.

use super::data as D;
use super::kit::{FlashFx, Item};
use crate::registry::Registry;

/// (item, table, progress of the switch to the incoming clip).
fn items() -> Vec<(Item, &'static [[f32; 8]], f32)> {
    let f = |n: f32| (n - 0.5) / 60.0;
    vec![(
        Item::new(
            "jb.bright_flash",
            "Bright Flash",
            "Light",
            "The picture blooms and overexposes into a soft white flash; the new clip appears inside it already \
             punched in by 11% and settles back while the bloom fades. Fitted on Filmora's export of Flash.",
            String::new(),
        ),
        D::FL_BRIGHT_FLASH,
        f(21.0),
    )]
}

pub fn register(r: &mut Registry) {
    for (it, rows, cut) in items() {
        r.register(FlashFx::new(it, rows, cut));
    }
}

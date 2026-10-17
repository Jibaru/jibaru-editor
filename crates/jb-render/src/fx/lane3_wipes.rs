//! Shape and line wipes fitted on Filmora's exports: the new clip is revealed
//! through a parametric mask whose threshold (and edge width) follows a
//! keyframe table fitted frame by frame.

use super::data as D;
use super::kit::{edge_wipe, ramp_wipe, register_items, Item};
use crate::registry::Registry;

fn items() -> Vec<Item> {
    vec![
        Item::new(
            "jb.line_erase_down",
            "Line Erase Down",
            "Wipes",
            "A hard horizontal edge sweeps down revealing the new clip: it jumps in quickly, slows to a crawl \
             through the middle and speeds off at the bottom. Fitted on Filmora's export of Erase Line Down.",
            edge_wipe("uv.y", D::ERASE_DOWN),
        ),
        Item::new(
            "jb.line_erase_right",
            "Line Erase Right",
            "Wipes",
            "A hard vertical edge sweeps left to right revealing the new clip, fast at both ends and slow in the \
             middle. Fitted on Filmora's export of Erase Line Right.",
            edge_wipe("uv.x", D::ERASE_RIGHT),
        ),
        Item::new(
            "jb.sides_merge",
            "Sides Merge",
            "Wipes",
            "The new clip closes in from the left and right edges towards a vertical centre line. Fitted on \
             Filmora's export of Col Merge.",
            edge_wipe("1.0 - 2.0 * abs(uv.x - 0.5)", D::COL_MERGE),
        ),
        Item::new(
            "jb.chalk_swipe_left",
            "Chalk Swipe Left",
            "Wipes",
            "A slightly slanted edge sweeps from right to left in stop-motion steps (about 12 per second), as \
             if wiped by hand. Fitted on Filmora's export of Chalk Swipe Left.",
            edge_wipe("(1.0 - uv.x) - 0.08 * uv.y", D::CHALK_SWIPE),
        ),
        Item::new(
            "jb.diagonal_dissolve",
            "Diagonal Dissolve",
            "Dissolve",
            "A soft diagonal front crosses from the top-left corner to the bottom-right, dissolving into the new \
             clip; the front widens as it goes. Fitted on Filmora's export of Dissolve Skew 01.",
            ramp_wipe("(uv.x * A + uv.y) / (A + 1.0)", D::SKEW_DISSOLVE),
        ),
    ]
}

pub fn register(r: &mut Registry) {
    register_items(r, items());
}

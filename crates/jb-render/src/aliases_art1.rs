//! Filmora items recreated with our own procedural artwork in the art1 lane
//! (`fx/art1_*.rs`, `shaders/art1/`): light leaks, flares, film burns, bokeh,
//! sparkles, particles, glows, fire and energy overlays. GUIDs keep Filmora's
//! exact case (`register` also adds the upper-case form). An item is mapped
//! here only when it measured no worse than the stand-in it replaces
//! (docs/fidelity-art1.md); each GUID has exactly one mapping across the
//! alias files.

use crate::registry::Registry;

/// (Filmora GUID, our id, Filmora item name for reference).
pub const FILMORA_ART1: &[(&str, &str, &str)] = &[
    ("9EE833DD-A002-4583-8ECB-AB2CD8A8FD7C", "jb.cyan_comet_glitter", "Sparkle Motion Transition 07"),
    ("E8A47581-CE66-4BF5-AFC8-685A3F68487F", "jb.pink_sparkler_rise", "Sparkle Motion Transition 11"),
    ("B35B100A-817A-499C-BA07-8E6E3C7DC4C8", "jb.glitter_swirl", "Sparkle Motion Transition 13"),
    ("F8A69A99-8CDD-42D3-9B38-9937F55EEEF5", "jb.golden_comet_arc", "Sparkle Motion Transition 15"),
    ("8B70FA91-0F25-442F-9624-DE656D5E0112", "jb.twin_comet_burst", "Sparkle Motion Transition 20"),
    ("F1FF9C55-AEDE-4F01-A698-3ABC9C69B682", "jb.cyan_splash_wipe", "Ice Firework Effect Vol 02 Transition 10"),
    ("C90D0AD6-EECC-496A-8254-2BF38D46D6A1", "jb.ember_flash_leak", "Light Leaks Transition 01"),
    ("B288EAEA-E306-4E5A-8BBB-4AE8F52B65AC", "jb.peach_orb_leak", "Light Leaks Transition 02"),
    ("5FCF4746-3646-415B-9884-A9ECED01EBE7", "jb.sunlit_haze_leak", "Light Leaks Transition 11"),
    ("DE3A553C-34A6-4EA4-B65D-BAC68391C2A1", "jb.golden_bokeh_band", "Light Leaks Transition Pack Transition 03"),
    ("DF9D7C57-52C1-43D3-9723-6E4759D4134E", "jb.fast_light_leaks", "Fast Light Leaks 2"),
    ("CE74AA1F-07F6-46A9-9D59-C4B1D7802A53", "jb.focus_pull_bokeh", "Camera Focus Bokeh"),
    ("3F5DEB07-5EBC-493A-BF42-9B2C8B5AEC22", "jb.cine_film_burn", "Movie Light Effects 2"),
    ("B619E435-F8D2-455C-8002-C31BEB023459", "jb.cine_film_gate_sun", "Movie Light Effects 6"),
    ("19AC37E2-3DE3-45AA-8323-47076A75B5E8", "jb.cine_light_bands", "Movie Light Effects 7"),
    ("22BFA34A-7C7E-4C0E-A007-6204E9E6F336", "jb.edge_glow_pulse", "Edge Glow 01"),
    ("6c7c67cf-ad75-ef11-b430-d8bbc19410ef", "jb.flash_tint_leak", "Aesthetic Light Leak 04 16-9"),
    ("EEC217D3-E1F3-4F9E-A2AC-3D61DFE0D97F", "jb.cine_light_rainbow", "Movie Light Effects 1"),
    ("C6EAFE28-6D0B-46D5-A92B-DA22A986432F", "jb.cine_scribble_leak", "Movie Light Effects 4"),
    ("D03BAEF2-F8F6-4E6B-9E30-4F2C0B7D53AC", "jb.cine_scribble_aurora", "Movie Light Effects 5"),
    ("3D6A60D4-179C-47D2-BA55-4FA3AD3C0665", "jb.cine_paint_ring", "Movie Light Effects 8"),
    ("ed8298c1-3a59-ef11-9cc6-50ebf65600b6", "jb.glow_ink_rise", "Glow Ink Up"),
    ("758d17c3-eec2-f111-9ef4-50ebf65600b6", "jb.flaming_ball_dash_1", "Burning Dash Transition 01"),
    ("748d17c3-eec2-f111-9ef4-50ebf65600b6", "jb.flaming_ball_dash_2", "Burning Dash Transition 02"),
    ("738d17c3-eec2-f111-9ef4-50ebf65600b6", "jb.flaming_ball_dash_3", "Burning Dash Transition 03"),
    ("728d17c3-eec2-f111-9ef4-50ebf65600b6", "jb.flaming_ball_dash_4", "Burning Dash Transition 04"),
];

pub fn register(r: &mut Registry) {
    for (fid, to, _) in FILMORA_ART1 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_art1_alias_resolves() {
        let r = crate::registry::Registry::builtin();
        for (fid, to, name) in super::FILMORA_ART1 {
            assert!(r.get(to).is_some(), "{name}: {to} is not registered");
            assert!(r.get(&format!("filmora:{fid}")).is_some(), "{name}: alias missing");
        }
    }
}

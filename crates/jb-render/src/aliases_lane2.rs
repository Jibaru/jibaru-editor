//! Filmora transitions of lane 2 (slides, pushes, rolls, cubes, flips, doors,
//! cards, folds, stacks, grids) mapped to their fitted implementations
//! (`fx/lane2_layers.rs`). GUIDs keep Filmora's exact case; both the exact
//! and the upper-cased form are registered.

use crate::registry::Registry;

/// (Filmora resource GUID, our id).
pub const FILMORA_LANE2: &[(&str, &str)] = &[
    // 3D Box Close
    ("16BFABD5-AC48-42AF-BAF9-149934D9B990", "jb.box_close_3d"),
    // 3D Box Down
    ("86437F55-4674-4809-9B7A-0D39DCF9E313", "jb.box_down_3d"),
    // 3D Box Drop
    ("3B310488-6093-4A52-B87E-0367ADF1641A", "jb.box_drop_3d"),
    // 3D Box Fall
    ("0C615FD6-6C3D-469A-955C-5FDF8FA6A23A", "jb.box_fall_3d"),
    // 3D Box Flip 1
    ("A84BB82F-36F0-4E6A-8488-FBC2F1AA0BD0", "jb.box_flip_3d_1"),
    // 3D Box Flip 2
    ("D1334A82-581D-42C6-B3CD-7B0DF3E5F565", "jb.box_flip_3d_2"),
    // 3D Box Flip 3
    ("E23C0AF1-1BCB-4083-908D-1952E6FC7494", "jb.box_flip_3d_3"),
    // 3D Box Flip 4
    ("B88000C0-71DF-4B92-B4DE-A63C7733978D", "jb.box_flip_3d_4"),
    // 3D Box Flip 5
    ("64D165BC-C38F-400F-965D-A9E33BA1128C", "jb.box_flip_3d_5"),
    // 3D Box Flip 6
    ("95EB31D4-DA7A-4642-8E35-CEA17DD93B35", "jb.box_flip_3d_6"),
    // Box Flip Down
    ("bf05541f-7e2b-4ee3-af3c-f2b5608879a8", "jb.box_flip_down"),
    // Box Flip Left
    ("B0FA9DF9-6C71-4B4D-8102-8F106A9E1385", "jb.box_flip_left"),
    // Box Flip Right
    ("6C491663-ADFE-49B5-855F-FF2AE53E8F34", "jb.box_flip_right"),
    // Box Flip Up
    ("c76663b6-72d0-4d87-9f35-0e4c57ce37da", "jb.box_flip_up"),
    // 3D Box Horizontal
    ("EBC160A4-C522-4DEE-8FF2-24596EFF5CDA", "jb.box_horizontal_3d"),
    // 3D Box Left
    ("75E87EE4-EC9E-4398-BA61-0653057F144A", "jb.box_left_3d"),
    // 3D Box Right
    ("9BF9CABE-864C-43D2-A282-24A578AF699F", "jb.box_turn_right_3d"),
    // 3D Box Up
    ("F9DB5BCD-C24E-43A4-BAE3-7FC4BE93A25F", "jb.box_up_3d"),
    // 3D Box Vertical
    ("1C7CC9AD-ACD1-4D08-9A39-82682F5BE775", "jb.box_vertical_3d"),
    // Boxed Reality
    ("95AE988D-EDE9-4C4F-B813-A24BE7DE7415", "jb.boxed_reality"),
    // 3D Camera Transitions 7
    ("F163DDE7-90AF-077C-84B2-1498C379F735", "jb.camera_cards_7"),
    // 3D Card
    ("1C56ABE1-F948-4426-AC71-6710D6362312", "jb.card_3d"),
    // Card Switch
    ("7F1B4870-2C3C-4E61-9066-2F993F00594E", "jb.card_switch"),
    // 3D Carousel Spin Left
    ("8E484E8F-4097-45F5-9857-90E0646776D6", "jb.carousel_spin_left"),
    // 3D Carousel Spin Left Close
    ("E5FCA5B4-4D4F-46D3-9C6A-930B3431F675", "jb.carousel_spin_left_close"),
    // 3D Carousel Spin Right
    ("1D999F3D-EA93-43EC-8CAB-A9694AF03CC8", "jb.carousel_spin_right"),
    // 3D Carousel Spin Right Close
    ("0BC474A4-AFFE-470D-8E52-6981F32C7003", "jb.carousel_spin_right_close"),
    // Cross Split
    ("75A09E8D-21D1-4c4e-AEA9-EB9CEC11A644", "jb.cross_split"),
    // Cube Facets Down
    ("FAC11A67-7C08-497E-B979-1EF1F923DEF6", "jb.cube_facets_down"),
    // Cube Facets Left
    ("55C7899F-8326-4E0E-8A14-BAD17A8757E5", "jb.cube_facets_left"),
    // Cube Facets Right
    ("C73CB5B2-6885-48E1-A97D-9DC8C8D1E3BD", "jb.cube_facets_right"),
    // Rotating Cube
    ("F23CEC10-EB99-4085-BA42-86CD41883CEE", "jb.cube_rotate"),
    // Cube Down
    ("59A8A58F-31CB-4740-9F9D-0DCD3CB7AF92", "jb.cube_turn_down"),
    // Cube Left
    ("A6B6286F-F9FE-4F9F-8C45-538EEC05F620", "jb.cube_turn_left"),
    // Cube Right
    ("0FCB240F-5328-4e85-869A-2D989F50D387", "jb.cube_turn_right"),
    // Cube Up
    ("5107B490-0EA4-47E1-8343-BD779895685F", "jb.cube_turn_up"),
    // Diagonal Slice Reveal
    ("E5C74593-D113-47B5-B174-252E1716826C", "jb.diagonal_slice_reveal"),
    // DoorWay
    ("976B2502-3B22-4dd4-AEB4-0AAA2C1E5AC8", "jb.doorway"),
    // Erase Slide Right
    ("4CA4EF88-5997-41a6-AAE6-0C2226D5A96E", "jb.erase_slide_right"),
    // 3D Fast Box Right 01
    ("93240F33-7900-4971-B90D-442502ED3A48", "jb.fast_box_right_1"),
    // 3D Fast Box Right 02
    ("A9CD0FFC-0A62-42BC-B595-2E85DD1F8F3A", "jb.fast_box_right_2"),
    // 3D Flip Right
    ("B04A402D-5C04-4110-AEAD-2E3DBEC1FEFF", "jb.flip_3d_right"),
    // 3D Flip
    ("EC14B694-D0AD-425A-806C-BA9D288341CC", "jb.flip_3d_tilt"),
    // Flip Down
    ("804982E3-3FC8-425E-B43A-6444F2C53B79", "jb.flip_down"),
    // Flip Down
    ("0A8EF5CE-6993-474F-9609-F0C3B2ADF0C1", "jb.flip_down_fast"),
    // Fast Flip
    ("175D2B82-F671-4D65-AF17-134C941C3A03", "jb.flip_fast"),
    // Flip Left
    ("F309557C-8B97-499A-9696-29B0FDD6367C", "jb.flip_left"),
    // Flip Pages Right
    ("BE89907B-B1DB-4D6B-A00C-D0F268F3F852", "jb.flip_pages_right"),
    // Flip Right
    ("7B680F51-1B11-480D-A4DE-C5065188BBCC", "jb.flip_right"),
    // Flip Roll Down
    ("4057dc68-ff07-4561-90b7-97762af89945", "jb.flip_roll_down"),
    // Flip Roll Left
    ("ae8e58de-6e52-4fca-851f-f8a6dfd41647", "jb.flip_roll_left"),
    // Flip Roll Right
    ("2787c059-6baa-4819-9298-9ee22071cc95", "jb.flip_roll_right"),
    // Flip Roll Up
    ("af113622-985e-4ce3-a807-9601607ee43f", "jb.flip_roll_up"),
    // Slow Flip
    ("F2C5B762-7F0D-4A06-BC2C-FBB8727AADA5", "jb.flip_slow"),
    // Fold Transition 01
    ("6912AFD8-AF5E-4E98-9C99-1D627627B4D6", "jb.fold_1"),
    // Fold Transition 02
    ("381884E7-ADD2-43EE-9316-D0C7F159C3B5", "jb.fold_2"),
    // Fold Transition 03
    ("621D9BE3-3FBD-488E-823E-A48E37FE24F7", "jb.fold_3"),
    // Fold Transition 04
    ("75729E2E-A47F-4E0C-8AC8-4DDA210C158B", "jb.fold_4"),
    // Fold Transition 08
    ("96262F03-7368-4610-91AB-6B25D29322D2", "jb.fold_8"),
    // Glass Panel Flash
    ("293A55BF-E78C-4F2A-A0C5-2B8E2B023C51", "jb.glass_panel_flash"),
    // 3D Page Flip Left
    ("A985CC65-0A2B-440F-B437-6F802A0305B0", "jb.page_flip_left_3d"),
    // 3D Horizontal Closing
    ("B070D206-0B16-4B67-BCF6-98414767AEA8", "jb.panels_close_3d"),
    // Photo Slide 1
    ("5561D5AD-7B58-498D-B099-66769FD3B7E3", "jb.photo_slide_1"),
    // Photo Slide 2
    ("3B896470-09FA-4D5D-A23A-1340920ADFCD", "jb.photo_slide_2"),
    // Photo Slide 3
    ("8E4DCEF8-3E13-4D9E-896A-EFD2C7969698", "jb.photo_slide_3"),
    // Photo Slide 4
    ("BD576272-6A8F-41C4-8643-8404A0B16D3E", "jb.photo_slide_4"),
    // Photo Slide 5
    ("B75481DA-EB86-48E6-805C-888F6C051913", "jb.photo_slide_5"),
    // Pivot Flip
    ("D9864B5B-CAF1-4C24-9124-49B8F7926046", "jb.pivot_flip"),
    // Fast Push Down
    ("255239EC-BB3E-4B54-B74B-A51F15AF381F", "jb.push_down_fast"),
    // Fast Push Left
    ("2DD72324-4654-456D-9EA8-9931F6B6A1FD", "jb.push_left_fast"),
    // Fast Push Right
    ("95DF8005-6F31-48DD-9713-3FB2EECF102F", "jb.push_right_fast"),
    // Retro Box 01
    ("7DB28C3E-D012-470E-B872-F71BA20437E2", "jb.retro_box"),
    // Simple Roll 10
    ("691302d8-de75-4f40-82e3-5dd30fc0dc55", "jb.roll_10"),
    // Simple Roll 11
    ("490e0bdb-f7b7-4fb2-a92a-ed5927ca13b3", "jb.roll_11"),
    // Simple Roll 6
    ("3e2c9408-4b1e-4e78-afc4-4d6c837f6c6e", "jb.roll_6"),
    // Simple Roll 7
    ("0fbaebd7-c224-4ca5-a9f0-d16307f19854", "jb.roll_7"),
    // Simple Roll 8
    ("93a969d4-9108-45d6-9c09-f08350b2dca7", "jb.roll_8"),
    // Simple Roll 9
    ("1568e098-ca48-4861-9667-6d3d6ed19b60", "jb.roll_9"),
    // Simple Roll 2
    ("d8867ccc-4762-49c4-93fd-616208d056cf", "jb.roll_down_2"),
    // Simple Roll 3
    ("f03e2971-f66f-4481-b308-a640d451d2ca", "jb.roll_left_3"),
    // Screen Flip 3D
    ("8D3025A8-8F60-42E7-B0AA-0C06F5608C76", "jb.screen_flip_3d"),
    // 3D Screens
    ("94860FC5-0D3E-42F4-A8BB-AE99CFBE5DF5", "jb.screens_3d"),
    // Skew Left Split
    ("5003D879-7930-44fa-A333-F21F62564709", "jb.skew_split_left"),
    // Skew Right Split
    ("2D4099D2-9B28-437f-8FEB-2A7E25C0B549", "jb.skew_split_right"),
    // 3D Slideshow Style 2
    ("DE2B8F85-34CE-43B3-A326-293F597C2B0F", "jb.slideshow_tilt_2"),
];

pub fn register(r: &mut Registry) {
    for (fid, to) in FILMORA_LANE2 {
        if r.get(to).is_some() {
            r.alias(format!("filmora:{fid}"), to);
            r.alias(format!("filmora:{}", fid.to_ascii_uppercase()), to);
        }
    }
}

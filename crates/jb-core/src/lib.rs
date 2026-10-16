//! Headless core of Jibaru Editor: the project model, timeline evaluation and
//! the undoable command bus shared by the UI, the CLI and agents.

pub mod command;
pub mod eval;
pub mod frame;
pub mod model;
pub mod param;
pub mod time;

pub use command::{apply, Command, CommandError, History};
pub use frame::{FrameSource, PixelFormat, VideoFrame};
pub use model::*;
pub use param::{Interpolation, Keyframe, Param, Value};
pub use time::{Rational, Ticks, TICKS_PER_SECOND};

/// Extension of the native project format.
pub const NATIVE_EXTENSION: &str = "jbp";

/// Serializes a project to the native JSON format.
pub fn to_native(project: &Project) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&serde_json::json!({ "format": "jibaru-project", "version": 1, "project": project }))
}

/// Parses a project from the native JSON format.
pub fn from_native(text: &str) -> serde_json::Result<Project> {
    #[derive(serde::Deserialize)]
    struct Doc {
        project: Project,
    }
    Ok(serde_json::from_str::<Doc>(text)?.project)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (History, Id) {
        let mut p = Project::new(
            "t",
            ProjectSettings {
                width: 1920,
                height: 1080,
                frame_rate: Rational::new(30, 1),
                sample_rate: 48000,
                background: [0.0, 0.0, 0.0, 1.0],
            },
        );
        p.media.push(MediaAsset {
            id: "m".into(),
            name: "m".into(),
            path: "/x.mp4".into(),
            kind: MediaKind::Video,
            duration: 100 * TICKS_PER_SECOND,
            width: 1920,
            height: 1080,
            frame_rate: None,
            has_audio: true,
            extra: Extra::Null,
        });
        let mut h = History::new(p);
        h.execute(&Command::AddTrack { sequence_id: None, kind: TrackKind::Video, index: None }).unwrap();
        let track = h.project.main().tracks[0].id.clone();
        (h, track)
    }

    fn clip(id: &str, start: Ticks, end: Ticks) -> Clip {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "source": {"type": "media", "media_id": "m"},
            "start": start, "end": end
        }))
        .unwrap()
    }

    #[test]
    fn add_split_undo() {
        let (mut h, track) = sample();
        h.execute(&Command::AddClip { track_id: track.clone(), clip: clip("a", 0, 10 * TICKS_PER_SECOND) }).unwrap();
        h.execute(&Command::SplitClip { clip_id: "a".into(), at: 4 * TICKS_PER_SECOND }).unwrap();
        let clips = &h.project.main().tracks[0].clips;
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[1].source_in, 4 * TICKS_PER_SECOND);
        assert!(h.undo());
        assert_eq!(h.project.main().tracks[0].clips.len(), 1);
    }

    #[test]
    fn overlap_is_rejected_atomically() {
        let (mut h, track) = sample();
        h.execute(&Command::AddClip { track_id: track.clone(), clip: clip("a", 0, 10) }).unwrap();
        assert!(h.execute(&Command::AddClip { track_id: track, clip: clip("b", 5, 15) }).is_err());
        assert_eq!(h.project.main().tracks[0].clips.len(), 1);
    }

    #[test]
    fn native_round_trip() {
        let (h, _) = sample();
        let text = to_native(&h.project).unwrap();
        assert_eq!(from_native(&text).unwrap(), h.project);
    }

    #[test]
    fn transition_blends_neighbors() {
        let (mut h, track) = sample();
        let mut a = clip("a", 0, 10 * TICKS_PER_SECOND);
        a.transition_out = Some(Transition {
            id: "t".into(),
            kind: "jb.dissolve".into(),
            duration: 2 * TICKS_PER_SECOND,
            alignment: TransitionAlignment::Center,
            params: Default::default(),
            extra: Extra::Null,
        });
        h.execute(&Command::AddClip { track_id: track.clone(), clip: a }).unwrap();
        h.execute(&Command::AddClip { track_id: track, clip: clip("b", 10 * TICKS_PER_SECOND, 20 * TICKS_PER_SECOND) })
            .unwrap();
        let plan = eval::evaluate(&h.project, h.project.main(), 10 * TICKS_PER_SECOND);
        let tr = plan.layers[0].transition.as_ref().unwrap();
        assert!((tr.progress - 0.5).abs() < 1e-9);
        assert_eq!(tr.incoming.clip_id, "b");
    }
}

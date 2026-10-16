//! Prints the FFmpeg version and the hardware decode devices and video
//! encoders usable on this machine: `cargo run -p jb-media --example hwinfo`

fn main() {
    jb_media::init();
    println!("FFmpeg: {}", jb_media::ffmpeg_version());
    println!("hardware decode devices: {:?}", jb_media::hw::available_devices());
    println!("video encoders (preference order): {:?}", jb_media::encode::available_video_encoders());
}

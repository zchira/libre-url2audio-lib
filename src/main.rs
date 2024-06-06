use std::thread::sleep;
use crate::player::Player;

mod url_source;
mod player_engine;
mod player;
mod pulseaudio;

fn main()   {
    let src = "https://podcast.daskoimladja.com/media/2024-05-27-PONEDELJAK_27.05.2024.mp3";
    // let src = "https://stream.daskoimladja.com:9000/stream";

    let p = Player::new();
    let res = p.open(src);
    println!("Res: {:#?}", res);

    println!("duration: {}", p.duration());
    sleep(std::time::Duration::from_secs(3));
    println!("Paused at: {}", p.current_position());
    p.pause();

    sleep(std::time::Duration::from_secs(3));
    p.play();
    println!("Resume at: {}", p.current_position());

    sleep(std::time::Duration::from_secs(3));
    p.seek(600.0);
    println!("seek 600: {}", p.current_position());

    sleep(std::time::Duration::from_secs(5));
    p.seek(1200.0);
    println!("seek 1200: {}", p.current_position());

    sleep(std::time::Duration::from_secs(5));
    p.seek(600.0);
    println!("seek back 600: {}", p.current_position());
}

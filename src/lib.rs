mod url_source;
mod player_engine;
mod pulseaudio;

use std::sync::{Arc, RwLock};

use crossbeam_channel::{unbounded, Receiver, Sender};

use crate::player_engine::{PlayerActions, PlayerEngine, PlayerState, PlayerStatus};

pub struct Player {
    inner_player: Arc<RwLock<PlayerEngine>>,
    tx: Sender<PlayerActions>,
    rx_status: Receiver<PlayerStatus>,
    state: Arc<RwLock<PlayerState>>
}

impl Player {
    pub fn new() -> Self {
        let (tx, rx) = unbounded(); 
        let (tx_status, rx_status) = unbounded(); 
        Player {
            inner_player: Arc::new(
                              RwLock::new(
                                  PlayerEngine::new(
                                      // tx.clone(),
                                      rx.clone(),
                                      tx_status.clone(),
                                      // rx_status.clone()
                                      ))),
            tx,
            rx_status,
            state: Arc::new(RwLock::new(PlayerState {
                playing: true,
                duration: 0.0,
                position: 0.0,
            }))
        }
    }

    pub fn open(&self, src: &str) {
        let player = self.inner_player.clone();
        let path = src.to_string();

        let _ = std::thread::spawn(move || {
            let mut p = player.write().unwrap();
            let _result = p.open(&path);
        });

        let rx1 = self.rx_status.clone();
        let s = self.state.clone();
        let _ = std::thread::spawn(move || {
            loop {
                let a = rx1.try_recv();

                match a {
                    Ok(a) => {
                        let mut state = s.write().unwrap();
                        match a {
                            PlayerStatus::SendPlaying(playing) => {
                                state.playing = playing;
                            }
                            PlayerStatus::SendDuration(duration) => {
                                state.duration = duration;
                            }
                            PlayerStatus::SendPosition(position) => {
                                state.position = position;
                            }
                        }
                    },
                    Err(_) => { 
                    },
                }
            }
        });
    }

    pub fn close(&self) {
    }

    pub fn play(&self) {
        let _ = self.tx.send(PlayerActions::Resume);
    }

    pub fn pause(&self) {
        let _ = self.tx.send(PlayerActions::Pause);
    }

    pub fn toggle_play(&self) {
    }

    /// seek to time from the beginning.
    /// `time` is in seconds
    pub fn seek(&self,time: f64) {
        let _ = self.tx.send(PlayerActions::Seek(time));
    }

    /// seek to time relative from current position
    pub fn seek_relative(&self, dt: f64) {
        let new_pos = self.current_position() + dt;
        let _ = self.tx.send(PlayerActions::Seek(new_pos));

    }

    pub fn current_position(&self) -> f64 {
        self.state.read().unwrap().position
    }

    pub fn duration(&self) -> f64 {
        self.state.read().unwrap().duration
    }

    

}

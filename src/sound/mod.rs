use std::{error::Error, fs::File, io::BufReader, num::NonZero, path::Path, time::Duration};

use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor,
};
use rodio::{
    Decoder, DeviceTrait,
    buffer::SamplesBuffer,
    cpal::{DeviceId, Host, traits::HostTrait},
};

pub enum PlayingStatus {
    Stopped,
    Paused,
    Played,
}

pub struct Player {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub duration: Duration,

    pub status: PlayingStatus,
    pub volume: f32,

    host: Host,
    device_id: DeviceId,
    sink: rodio::MixerDeviceSink,
    player: rodio::Player,
    current_path: String,
}

impl Player {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let host = rodio::cpal::default_host();
        let default_device = host.default_output_device().unwrap();
        let device_id = default_device.id()?;

        let mut sink = rodio::DeviceSinkBuilder::from_device(default_device)?.open_stream()?;
        sink.log_on_drop(false);
        let player = rodio::Player::connect_new(sink.mixer());

        Ok(Self {
            artist: String::new(),
            title: String::new(),
            album: String::new(),
            duration: Duration::ZERO,
            status: PlayingStatus::Stopped,
            volume: 0.5,
            host,
            device_id,
            sink,
            player,
            current_path: String::new(),
        })
    }

    pub fn check_device(&mut self) -> Result<(), Box<dyn Error>> {
        let default_device = self.host.default_output_device().unwrap();
        let Ok(device_id) = default_device.id() else {
            return Ok(());
        };

        if self.device_id != device_id {
            self.device_id = device_id;

            let position = self.get_position();

            self.sink = rodio::DeviceSinkBuilder::from_device(default_device)?.open_stream()?;
            self.sink.log_on_drop(false);
            self.player = rodio::Player::connect_new(self.sink.mixer());

            let current = self.current_path.clone();
            self.play(Path::new(current.as_str()))?;
            self.move_position(position.as_secs_f64())?;
        }

        Ok(())
    }

    fn get_mod(path: &Path) -> (SamplesBuffer, f64) {
        // TODO fix remove println in read_mod_file
        let song = mod_player::read_mod_file(path.to_str().unwrap());
        let mut player_state: mod_player::PlayerState =
            mod_player::PlayerState::new(song.format.num_channels, 44100);
        let mut samples = Vec::new();
        loop {
            let (left, right) = mod_player::next_sample(&song, &mut player_state);
            samples.push(left);
            samples.push(right);
            if player_state.song_has_ended || player_state.has_looped {
                break;
            }
        }

        let duration = (samples.len() / 2) as f64 / 44100 as f64 / 2.;

        let source = SamplesBuffer::new(
            NonZero::new(song.format.num_channels as u16).unwrap(),
            NonZero::new(44100).unwrap(),
            samples,
        );

        (source, duration)
    }

    pub fn play(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.player.stop();

        self.current_path = path.to_str().unwrap().to_string();

        if let Some(extension) = path.extension()
            && extension.to_str().unwrap() == "mod"
        {
            let (source, duration) = Self::get_mod(path);
            self.title = path.file_name().unwrap().to_str().unwrap().to_string();
            self.duration = Duration::from_secs_f64(duration);
            self.player.append(source);
        } else {
            let file = File::open(path)?;
            let source = Decoder::try_from(BufReader::new(file))?;

            let tagged_file = lofty::read_from_path(path)?;

            let props = tagged_file.properties();
            let Some(tag) = tagged_file.first_tag() else {
                return Ok(());
            };

            if let Some(title) = tag.title() {
                self.title = title.to_string();
            }

            if let Some(artist) = tag.artist() {
                self.artist = artist.to_string();
            }

            if let Some(album) = tag.album() {
                self.album = album.to_string();
            }

            self.duration = props.duration();

            self.player.append(source);
        }

        self.player.set_volume(self.volume);
        self.player.play();

        self.status = PlayingStatus::Played;

        Ok(())
    }

    pub fn pause(&mut self) {
        self.status = PlayingStatus::Paused;
        self.player.pause();
    }

    pub fn resume(&mut self) {
        self.status = PlayingStatus::Played;
        self.player.play();
    }

    pub fn move_position(&mut self, position: f64) -> Result<(), Box<dyn Error>> {
        let cur = self.get_position().as_secs_f64();
        let new = cur + position;
        if new < cur {
            let p = self.current_path.clone();
            let path = Path::new(p.as_str());
            self.play(path)?;
        }
        if 0. > new {
            self.player.try_seek(Duration::from_millis(1000))?;
        } else if new >= self.duration.as_secs_f64() {
            self.player.try_seek(self.duration)?;
        } else {
            self.player
                .try_seek(Duration::try_from_secs_f64(new).unwrap())?;
        }
        Ok(())
    }

    pub fn get_position(&self) -> Duration {
        self.player.get_pos()
    }

    pub fn set_volume(&mut self, volume: f32) {
        if 0. < volume && volume <= 1. {
            self.player.set_volume(volume);
            self.volume = volume;
        }
    }
}

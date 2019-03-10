extern crate cpal;
use hound;

use std::fs;
use std::thread;
use std::sync::mpsc;
use std::sync;

mod mod_player;

enum PlayerCommand{
    PlayInstrument{ index : u8 }
}

/**
 * Write the mod file into a wav file 
 */
fn write_song_to_wav( file_name : &str, song : &mod_player::Song ) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create( file_name, spec).unwrap();
    let mut player_state : mod_player::PlayerState = mod_player::PlayerState::new( song.format.num_channels, spec.sample_rate );
    loop {
        let ( left, right ) = mod_player::next_sample(&song, &mut player_state);
        writer.write_sample( left  );
        writer.write_sample( right  );
        if player_state.song_has_ended || player_state.has_looped { 
            break;
        }
    }
}

fn setup_stream( song : sync::Arc<mod_player::Song> ) -> mpsc::Sender<PlayerCommand> {
    let device = cpal::default_output_device().expect("Failed to get default output device");
    println!("Sound device: {}", device.name());

    let format  = device.default_output_format().expect("Failed to get default output format");
    let fmt = match format.data_type {
        cpal::SampleFormat::I16 => "i16",
        cpal::SampleFormat::U16 => "u16",
        cpal::SampleFormat::F32 => "f32"
    };

    println!("Sample rate: {}    Sample format: {}       Channels: {}", format.sample_rate.0, fmt, format.channels);
    
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
    event_loop.play_stream(stream_id.clone());

    let mut instrument_number = 0;
    let mut instrument_pos = 0;
    let mut player_state : mod_player::PlayerState = mod_player::PlayerState::new( song.format.num_channels, format.sample_rate.0);
    let mut last_line_pos = 999;

    let (tx,rx) = mpsc::channel();
    thread::spawn( move || {
        event_loop.run(move |_, data| {
            let message = rx.try_recv();
            if message.is_ok() {
                match message.unwrap() {
                    PlayerCommand::PlayInstrument{ index } => { 
                        instrument_number = index;
                        instrument_pos = 0;
                        println!( "Playing instrument {}", index );   //Set up instrument playing here
                    }
                };
            }
            if player_state.current_line != last_line_pos {
                if player_state.current_line == 0 {
                    println!("");
                }
                mod_player::textout::print_line( player_state.get_song_line(&song));
                last_line_pos = player_state.current_line;
            }
            match data {
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        let ( left, right ) = mod_player::next_sample(&song, &mut player_state);
                        sample[0] = left;
                        sample[1] = right;
                    }
                }
                _ => (),
            }
        });
    });
    tx
}

fn main() {
    let song = sync::Arc::new( mod_player::read_mod_file("stardstm.MOD") );
//    write_song_to_wav( "test.wav", &song );           // use this to write the file to WAV 
    let tx = setup_stream(song.clone());
    loop{
        let mut command = String::new();
        std::io::stdin().read_line(& mut command);
        command = command.trim_end().to_string();
        let res  = command.parse::<u8>();
        if res.is_ok() {
            tx.send(PlayerCommand::PlayInstrument{ index : res.unwrap() } );
        }
    }
}

extern crate cpal;

use std::fs;
use std::thread;
use std::sync::mpsc;
use std::sync;

const CLOCK_TICKS_PERS_SECOND: f32 = 3579545.0;      // Amiga hw clcok ticks per second

pub struct Sample {
    name: String,
    size: u32,
    volume: u8,
    fine_tune: u8,
    repeat_offset: u32,
    repeat_size: u32,
    samples: Vec<i8>, 
}

impl Sample{
    fn new( sample_info : &[u8] ) -> Sample {
        let sample_name = String::from_utf8_lossy(&sample_info[0..22]);
        let sample_size: u32 = ((sample_info[23] as u32) + (sample_info[22] as u32) * 256) * 2;
        let fine_tune = sample_info[24];
        let volume = sample_info[25];

        let repeat_offset: u32 = (sample_info[27] as u32) + (sample_info[26] as u32) * 256;
        let repeat_size: u32 = (sample_info[29] as u32) + (sample_info[28] as u32) * 256;

        Sample {
            name: String::from(sample_name),
            size: sample_size,
            volume: volume,
            fine_tune: fine_tune,
            repeat_offset: repeat_offset,
            repeat_size: repeat_size,
            samples: Vec::new(),
        }
    }
    fn print(&self) {
        println!("   sample Name: {}", self.name);
        println!("   sample Size: {}", self.size);
        println!("   sample volume: {}, fine tune {}", self.volume, self.fine_tune);
        println!("   repeat Offset: {}, repeat Size {}", self.repeat_offset, self.repeat_size);
    }
}

enum Effect{
/*  SlideUp = 1,
    SlideDown = 2,
    TonePortamento = 3,
    Vibrato = 4,
    TonePortamentoVolumeSlide = 5,
    VibratoVolumeSlide = 6,
    Tremolo = 7,
    SetPanningPosition = 8,
    SetSampleOffset = 9,
    VolumeSlide = 10,
    PositionJump = 11,
    SetVolume = 12,
    PatternBreak = 13,
    ExtendedEffects = 14,
*/    
    None, // 0
    SetVolume{ volume : u8 },           // 12
    SetSpeed{ speed : u8 },             // 15
}

impl Effect{
    fn new( effect_number : u8, effect_argument : i8 ) -> Effect {
        match effect_number  {
            0 => match effect_argument {
                0 => Effect::None,
                _ => panic!( format!( "unhandled arpeggio effect: {}", effect_number ) )
            },
            12 => Effect::SetVolume{ volume : effect_argument as u8 },
            15 => Effect::SetSpeed{ speed : effect_argument as u8 }, 
            _ => panic!( format!( "unhandled effect number: {}", effect_number ) )
        }
    }
}

struct Note{
    sample_number: u8,
    period: u32,            // how many clock ticks each sample is held for
    effect: Effect,
}

impl Note{
    fn new( note_data : &[u8]) -> Note {
        let sample_number = (note_data[2] & 0xf0) >> 4;
        let period = ((note_data[0] & 0x0f) as u32) * 256 + (note_data[1] as u32);
        let effect_argument = note_data[3] as i8;
        let effect_number = note_data[ 2] & 0x0f;
        let effect = Effect::new(effect_number, effect_argument);
        Note{
            sample_number, period, effect
        }
    }
}

pub struct Pattern {
    lines: Vec<Vec<Note>>       // outer vector is the lines (64). Inner vector holds the notes for the line             
}

impl Pattern{
    fn new( ) -> Pattern {
        let mut lines : Vec<Vec<Note>> = Vec::new();
        for _line in 0..64 {
            lines.push( Vec::new() );
        }
        Pattern{ lines }
    }

    fn print_line( &self, line_number : usize ) {
        let line = &self.lines[ line_number ];
        for note in line.iter() {
            print!("#{:2} - {:4}   ",  note.sample_number, note.period );
        }
        println!(""); 
    }
}

struct FormatDescription{
    num_channels : u32,
    num_samples : u32,
    has_tag : bool      // Is the format description based on a tag
}

pub struct Song {
    name: String,
    format : FormatDescription,
    samples: Vec<Sample>,
    patterns: Vec<Pattern>,
    pattern_table: Vec<u8>,
}

struct ChannelInfo {
    sample_num: u8,         // which sample is playing 
    sample_pos: f32,         
    period : f32,           //
    volume: f32,            // max 1.0
}

impl ChannelInfo{
    fn new() -> ChannelInfo {
        ChannelInfo {
            sample_num: 0,
            sample_pos: 0.0,
            period : 0.0,
            volume: 0.0,
        }
    }
}

struct PlayerState{
    channels: Vec<ChannelInfo>,
    song_pattern_position: u32,             // where in the pattern table are we currently
    current_line: u32,                      // current position in the pattern
    song_speed: u32,                        // in vblanks
    current_vblank : u32,                   // how many vblanks since last play line
    current_vblank_sample : u32,            // how many device samples have we played for the current 'vblank'
    samples_per_vblank: u32,                // how many device samples per 'vblank'
    clock_ticks_per_device_sample : f32,    // how many amiga hardware clock ticks per device sample
}

impl PlayerState{
    fn new( num_channels : u32, device_sample_rate : u32 ) -> PlayerState {
        let mut channels = Vec::new();
        for _channel in 0..num_channels {
            channels.push(ChannelInfo::new())
        }
        PlayerState{
            channels,
            song_pattern_position : 0,
            current_line: 0,
            current_vblank : 0,             
            current_vblank_sample : 0,      
            song_speed: 6,                  
            samples_per_vblank: device_sample_rate / 50,
            clock_ticks_per_device_sample : CLOCK_TICKS_PERS_SECOND / device_sample_rate as f32,
        }
    }
}

fn play_note(note: &Note, player_state: &mut PlayerState, channel_num: usize, song: &Song) {
    if note.sample_number > 0 {
        // sample number 0, means that the sample keeps playing. The sample indices starts at one, so subtract 1 to get to 0 based index
        player_state.channels[channel_num].volume = song.samples[(note.sample_number - 1) as usize].volume as f32;    // Get volume from sample
        player_state.channels[channel_num].sample_num = note.sample_number;
        player_state.channels[channel_num].sample_pos = 0.0;
        if note.period != 0 {
            player_state.channels[channel_num].period = note.period as f32;
        }
    }

    match note.effect {
        Effect::SetSpeed{ speed } => {
            player_state.song_speed = speed as u32;
        }
        Effect::SetVolume{ volume } => {
            player_state.channels[channel_num].volume = volume as f32;
        }
        Effect::None => {}
        _ => {
            println!("Unhandled effect" );
        }
    }
}


fn play_line(song: &Song, player_state: &mut PlayerState ) {
    // use the pattern table to work out which pattern we are in    
    let pattern_idx = song.pattern_table[player_state.song_pattern_position as usize];
    let pattern = &song.patterns[ pattern_idx as usize];

    pattern.print_line(player_state.current_line as usize);
    let line = &pattern.lines[ player_state.current_line as usize ];
    for channel_number in 0..line.len(){
        play_note(&line[ channel_number as usize ], player_state, channel_number, song);
    }
    player_state.current_line += 1;
    if player_state.current_line >= 64 {
        player_state.song_pattern_position += 1;
        player_state.current_line = 0;
        println!("");
    }
}

fn next_sample(song: &Song, player_state: &mut PlayerState) -> (f32, f32) {
    let mut left = 0.0;
    let mut right = 0.0;


    // Have we reached a new vblank
    if player_state.current_vblank_sample >= player_state.samples_per_vblank {
        player_state.current_vblank_sample = 0;

        // Is it time to play a new note line
        if player_state.current_vblank >= player_state.song_speed {
            player_state.current_vblank = 0;
            play_line( song, player_state );

        }
        // apply on every vblank but only after the line has been processed
        player_state.current_vblank += 1;
    }
    player_state.current_vblank_sample += 1;


    for channel_number in 0..player_state.channels.len() {
        let channel_info: &mut ChannelInfo = &mut player_state.channels[channel_number];
        if channel_info.sample_num != 0 {
            let current_sample: &Sample = &song.samples[(channel_info.sample_num - 1) as usize];
            let mut channel_value: f32 = current_sample.samples[(channel_info.sample_pos as u32) as usize] as f32;   // [ -127, 127 ] 
            // max channel vol (64), sample range [ -128,127] scaled to [-1,1] 
            channel_value *= channel_info.volume / (128.0*64.0);

            // update position and check if we have reached the end of the sample
            channel_info.sample_pos +=  player_state.clock_ticks_per_device_sample / channel_info.period;
            if channel_info.sample_pos >= current_sample.size as f32 {
                channel_info.sample_num = 0;
            }
            let channel_selector = ( channel_number as u8 ) & 0x0003; 
            if channel_selector == 0 || channel_number as u32 == 0 || channel_number == 3 {
                left += channel_value;
            } else {
                right += channel_value;
            }
        }
    }

    (left, right )
}

/**
 * Identify the mod format version based on the tag. If there is not identifiable that it is assumed to be an original mod.
 */
fn get_format(file_data: &Vec<u8> ) -> FormatDescription {
    let format_tag = String::from_utf8_lossy(&file_data[1080..1084]);
    match format_tag.as_ref() {
        "M.K." | "FLT4" | "M!K!" | "4CHN" => FormatDescription{ num_channels : 4, num_samples : 31, has_tag : true },
        _ => FormatDescription{ num_channels : 4, num_samples : 15, has_tag : false }
    }
}

fn read_mod_file(file_name: &str) -> Song {
    let file_data: Vec<u8> = fs::read(file_name).expect( &format!(  "Cant open file {}", &file_name ) );

    let song_name = String::from_utf8_lossy(&file_data[0..20]);
    println!("Song: {}", song_name);

    let format = get_format(&file_data);
    println!("Number of samples: {}", format.num_samples);

    let mut samples: Vec<Sample> = Vec::new();
    let mut offset : usize = 20;
    for _sample_num in 0..format.num_samples {
        samples.push(Sample::new( &file_data[ offset  .. ( offset + 30 ) as usize  ]));
        offset += 30;
    }

    for sample in &samples {
        sample.print()
    }

    let num_patterns: u8 = file_data[offset];
    let end_position: u8 = file_data[offset + 1];
    offset += 2;
    let pattern_table: Vec<u8> = file_data[offset..(offset + 128)].to_vec();
    offset += 128;

    println!(" num patterns in song: {}", num_patterns);
    println!(" end position: {}", end_position);

    println!(" num patterns in song: {}", num_patterns);
    println!(" end position: {}", end_position);

    // Skip the tag if one has been identified
    if format.has_tag {
        let format_tag = String::from_utf8_lossy(&file_data[offset..(offset + 4)]);
        offset += 4;
        println!(" TAG: {}", format_tag);
    }

    // Work out how the total size of the sample data at tbe back od the file 
    let mut total_sample_size = 0;
    for sample in &mut samples {
        total_sample_size = total_sample_size + sample.size;
    }
    println!("Total sample size: {}", total_sample_size);

    // The pattern take up all the space that remains after everything else has been accounted for
    let total_pattern_size = file_data.len() as u32  - (offset as u32) - total_sample_size;
    let single_pattern_size = format.num_channels *  4 * 64;
    let num_patterns = total_pattern_size / single_pattern_size;
    // The pattern space should account for all the remaining space
    if total_pattern_size % single_pattern_size != 0 {
        panic!( "Unrecognized file format. Pattern space does not match expected size")
    }

    println!(" offset: {}", offset);
    println!(" pattern space: {}", total_pattern_size);
    println!(" num patterns: {}", num_patterns);

    // Read the patterns
    let mut patterns: Vec<Pattern> = Vec::new();
    for _pattern_number in 0..num_patterns {
        let mut pattern = Pattern::new();
        for line in 0..64 {
            for _channel in 0..format.num_channels {
                let note = Note::new( &file_data[ offset..(offset+4)]);
                pattern.lines[ line ].push( note );
                offset += 4;
            }
        }
        patterns.push(pattern);
    }

    //Read the sample data
    for sample_number in 0..samples.len() {
        let length = samples[sample_number].size;
        for _idx in 0..length {
            samples[sample_number].samples.push(file_data[offset] as i8);
            offset += 1;
        }
    }

    Song {
        name: String::from(song_name),
        format : format,
        samples: samples,
        patterns: patterns,
        pattern_table: pattern_table,
    }
}

enum PlayerCommand{
    PlayInstrument{ index : u8 }
}

fn setup_stream( song : sync::Arc<Song> ) -> mpsc::Sender<PlayerCommand> {
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
    let mut player_state : PlayerState = PlayerState::new( song.format.num_channels, format.sample_rate.0);
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
            match data {
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        let ( left, right ) = next_sample(&song, &mut player_state);
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
    let song = sync::Arc::new( read_mod_file("AXELF.MOD") );
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

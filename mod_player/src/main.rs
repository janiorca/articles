use std::fs;

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

fn identify_num_samples(file_data: &Vec<u8>) -> u32 {
    let format_tag = String::from_utf8_lossy(&file_data[1080..1084]);
    match format_tag.as_ref() {
        "M.K." => 31,
        _ => 15
    }
}

fn read_mod_file(file_name: &str){
    let file_data: Vec<u8> = fs::read(file_name).expect( &format!(  "Cant open file {}", &file_name ) );

    let song_name = String::from_utf8_lossy(&file_data[0..20]);
    println!("Song: {}", song_name);

    let num_samples = identify_num_samples(&file_data);
    println!("Number of samples: {}", num_samples);

    let mut samples: Vec<Sample> = Vec::new();
    let mut offset : usize = 20;
    for _sample_num in 0..num_samples {
        samples.push(Sample::new( &file_data[ offset  .. ( offset + 30 ) as usize  ]));
        offset += 30;
    }

    for sample in samples {
        sample.print()
    }

    let num_patterns: u8 = file_data[offset];
    let end_position: u8 = file_data[offset + 1];
    offset += 2;
    let pattern_table: Vec<u8> = file_data[offset..(offset + 128)].to_vec();
    offset += 128;

    println!(" num patterns in song: {}", num_patterns);
    println!(" end position: {}", end_position);

}

fn main() {
    read_mod_file("AXELF.MOD");
}

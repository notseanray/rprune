use miniz_oxide::inflate;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::error::Error;
use std::fs::read_dir;
use std::fs::File;
use std::fs::remove_file;
use std::io::SeekFrom::Start;
use std::io::{Read, Seek};
use std::path::PathBuf;

pub(crate) struct World(PathBuf);

impl World {
    #[inline]
    pub(crate) fn new(path: &str) -> Self {
        Self(PathBuf::from(path))
    }

    pub(crate) fn run(&self, limit: usize) -> Result<(), Box<dyn Error>> {
        for part in &["region", "DIM-1/region", "DIM1/region"] {
            let regions: Vec<(String, PathBuf)> = read_dir(self.0.join(part))?
                .map(|f| {
                    // should fix this unwrap probably
                    let f = f.unwrap();
                    (f.file_name().to_string_lossy().to_string(), f.path())
                })
                .collect();
            regions.par_iter().for_each(|c| {
                let name: Vec<&str> = c.0.split('.').collect();
                if name.len() < 4 {
                    return
                }
                if let (Ok(x), Ok(z)) = (name[1].parse(), name[2].parse()) {
                    let chunk = Chunk::new(x, z, limit);
                    if chunk.load_data_and_decode(&c.1).is_err() {
                        println!("failed to decode region r.{}.{}.mca", chunk.0, chunk.1);
                    }
                }
            });
        }
        Ok(())
    }
}

pub(crate) struct Chunk(i32, i32, usize);

impl Chunk {
    #[inline]
    pub(crate) fn new(x: i32, z: i32, limit: usize) -> Self {
        Self(x, z, limit)
    }

    pub(crate) fn load_data_and_decode(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let mut region = File::open(path)?;
        let location_offset = 4 * ((self.0 & 31) + (self.1 & 31) * 32);
        let mut location_header = [0; 4];

        // some of the code for this conversion is *borrowed* from
        // https://github.com/samipourquoi/overviewer/blob/main/src/region/chunk.rs
        region.seek(Start(location_offset as u64))?;
        region.read_exact(&mut location_header)?;
        let chunk_offset: usize = ((65536 * location_header[0] as u32
            + 256 * location_header[1] as u32
            + location_header[2] as u32)
            * 4096) as usize;
        if chunk_offset == 0 {
            return Ok(());
        }
        let mut chunk_header = [0; 5];

        region.seek(Start(chunk_offset as u64))?;
        region.read_exact(&mut chunk_header)?;

        let chunk_data_length: usize = (16777216 * chunk_header[0] as u32
            + 65536 * chunk_header[1] as u32
            + 256 * chunk_header[2] as u32
            + chunk_header[3] as u32
            - 1) as usize;
        let mut chunk_data: Vec<u8> = vec![0; chunk_data_length];

        region.read_exact(&mut chunk_data)?;
        if let Ok(data) = &inflate::decompress_to_vec_zlib(&chunk_data) {
            self.check_time(data, path)?;
        }
        Ok(())
    }

    #[inline]
    fn check_time(&self, mut data: &[u8], path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let nbt = nbt::Blob::from_reader(&mut data)?;
        if let Some(nbt::Value::Compound(v)) = nbt.get("Level") {
            if let Some(nbt::Value::Long(t)) = v.get("InhabitedTime") {
                if t < &(self.2 as i64) {
                    remove_file(path)?;
                    println!("removed r.{}.{}.mca with time {}", self.0, self.1, t);
                }
            }
        }
        Ok(())
    }
}

use prisirv::{
    Prisirv,
    config::{Config, Mode},
};

/// Create a new Config and call Prisirv API.
fn main() {
    let cfg = Config::new(&std::env::args().skip(1).collect::<Vec<String>>());
    match cfg.mode {
        Mode::Compress => { 
            Prisirv::new(cfg).create_archive();  
        }
        Mode::Decompress => { 
            Prisirv::new(cfg).extract_archive(); 
        }
        Mode::AddFiles => { 
            Prisirv::new(cfg).add_archive();
        }
        Mode::ExtractFiles => { 
            Prisirv::new(cfg).extract_file(); 
        }
    }
}

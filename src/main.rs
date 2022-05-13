use prisirv::{
    Prisirv,
    config::{Config, Mode},
};

/// Create a new Config and call Prisirv API.
fn main() {
    match Config::new(&std::env::args().skip(1).collect::<Vec<String>>()) {
        Ok(cfg) => {
            match cfg.mode {
                Mode::CreateArchive => { 
                    Prisirv::new(cfg).create_archive();  
                }
                Mode::ExtractArchive => { 
                    Prisirv::new(cfg).extract_archive(); 
                }
                Mode::AddFiles => { 
                    Prisirv::new(cfg).add_files();
                }
                Mode::ExtractFiles => { 
                    Prisirv::new(cfg).extract_files(); 
                }
            }
        }
        Err(err) => {
            println!("{err}");
        }
    }
    
}

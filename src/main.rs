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
                    if let Err(err) = Prisirv::new(cfg).create_archive() {
                        println!("{err}");
                    } 
                }
                Mode::ExtractArchive => { 
                    if let Err(err) = Prisirv::new(cfg).extract_archive() {
                        println!("{err}");
                    } 
                }
                Mode::AppendFiles => { 
                    if let Err(err) = Prisirv::new(cfg).append_files() {
                        println!("{err}");
                    }
                }
                Mode::ExtractFiles => { 
                    if let Err(err) = Prisirv::new(cfg).extract_files() {
                        println!("{err}");
                    }  
                }
                Mode::ListArchive => {
                    if let Err(err) = Prisirv::new(cfg).info() {
                        println!("{err}");
                    } 
                }
                Mode::Fv => {
                    if let Err(err) = Prisirv::new(cfg).fv() {
                        println!("{err}");
                    }
                }
                
            }
        }
        Err(err) => {
            println!("{err}");
        }
    }
}

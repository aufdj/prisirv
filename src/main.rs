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
                        print!("An error occurred while creating archive.");
                        print!("{err}");
                    } 
                }
                Mode::ExtractArchive => { 
                    if let Err(err) = Prisirv::new(cfg).extract_archive() {
                        print!("An error occurred while extracting archive.");
                        print!("{err}");
                    } 
                }
                Mode::AppendFiles => { 
                    if let Err(err) = Prisirv::new(cfg).append_files() {
                        print!("An error occurred while appending files.");
                        print!("{err}");
                    }
                }
                Mode::ExtractFiles => { 
                    if let Err(err) = Prisirv::new(cfg).extract_files() {
                        print!("An error occurred while extracting files.");
                        print!("{err}");
                    }  
                }
                Mode::ListArchive => {
                    let verbose = cfg.verbose;
                    match Prisirv::new(cfg).info() {
                        Ok(info) => {
                            if verbose {
                                println!("{:?}", info);
                            }
                            else {
                                println!("{}", info);
                            }   
                        },
                        Err(err) => println!("{err}"),
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

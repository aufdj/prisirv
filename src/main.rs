use prisirv::{
    Prisirv,
    config::{Config, Mode},
    filedata::FileData,
};

/// Create a new Config and call Prisirv API.
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    if args.len() == 1 {
        let mut cfg = Config::default();
        let file = FileData::from(&args[0]);
        
        if let Some(ext) = file.path.extension() {
            if ext == "prsv" {
                cfg.arch = file;
                Prisirv::new(cfg).extract_archive().unwrap();
            }
            else {
                cfg.inputs.push(file);
                Prisirv::new(cfg).create_archive().unwrap();
            }
        }
        else {
            cfg.inputs.push(file);
            Prisirv::new(cfg).create_archive().unwrap();
        }
    }
    match Config::new(args) {
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
                Mode::MergeArchives => { 
                    if let Err(err) = Prisirv::new(cfg).merge_archives() {
                        print!("An error occurred while merging archives.");
                        print!("{err}");
                    }
                }
                Mode::ListArchive => {
                    let verbose = cfg.verbose;
                    match Prisirv::new(cfg).info() {
                        Ok(info) => {
                            if verbose {
                                println!("{info:?}");
                            }
                            else {
                                println!("{info}");
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
                Mode::None => {
                    print!("{}", Prisirv::default());
                }
            }
        }
        Err(err) => {
            println!("{err}");
        }
    }
}

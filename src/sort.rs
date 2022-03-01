use std::{
    path::Path,
    cmp::Ordering,
    ffi::OsStr,
};

/// Sort files to improve compression of solid archives.

#[derive(Debug, Clone)]
pub enum Sort {    // Sort By:
    None,          // No sorting
    Ext,           // Extension
    Name,          // Name
    Len,           // Length
    PrtDir(usize), // Parent Directory
    Created,       // Creation Time
    Accessed,      // Last Access Time
    Modified,      // Last Modification Time
}

pub fn sort_files(f1: &str, f2: &str, sorting_method: &Sort) -> Ordering {
    match sorting_method {
        Sort::Ext => {
            let ext1 = match Path::new(f1).extension() {
                Some(ext) => ext,
                None => OsStr::new(""),
            };
            let ext2 = match Path::new(f2).extension() {
                Some(ext) => ext,
                None => OsStr::new(""),
            };
            (ext1.to_ascii_lowercase()).cmp(&ext2.to_ascii_lowercase())
        }
        Sort::Name => {
            let name1 = match Path::new(f1).file_name() {
                Some(name) => name,
                None => OsStr::new(""),
            };
            let name2 = match Path::new(f2).file_name() {
                Some(name) => name,
                None => OsStr::new(""),
            };
            (name1.to_ascii_lowercase()).cmp(&name2.to_ascii_lowercase())
        }
        Sort::Len => {
            let len1 = match Path::new(f1).metadata() {
                Ok(data) => data.len(),
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            let len2 = match Path::new(f2).metadata() {
                Ok(data) => data.len(),
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            (len1).cmp(&len2)
        }
        Sort::PrtDir(lvl) => {
            let parent1 = match Path::new(f1).ancestors().nth(*lvl) {
                Some(path) => path,
                None => Path::new(""),
            };
            let parent2 = match Path::new(f2).ancestors().nth(*lvl) {
                Some(path) => path,
                None => Path::new(""),
            };
            (parent1).cmp(parent2)
        }
        Sort::Created => {
            let creation_time1 = match Path::new(f1).metadata() {
                Ok(data) => {
                    match data.created() {
                        Ok(creation_time) => creation_time,
                        Err(_) => {
                            println!("Creation time metadata not supported on this platform.");
                            std::process::exit(0);
                        }
                    }
                }  
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            let creation_time2 = match Path::new(f2).metadata() {
                Ok(data) => {
                    match data.created() {
                        Ok(creation_time) => creation_time,
                        Err(_) => {
                            println!("Creation time metadata not supported on this platform.");
                            std::process::exit(0);
                        }
                    }
                }  
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            (creation_time1).cmp(&creation_time2)
        }
        Sort::Accessed => {
            let access_time1 = match Path::new(f1).metadata() {
                Ok(data) => {
                    match data.accessed() {
                        Ok(access_time) => access_time,
                        Err(_) => {
                            println!("Last accessed time metadata not supported on this platform.");
                            std::process::exit(0);
                        }
                    }
                }  
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            let access_time2 = match Path::new(f2).metadata() {
                Ok(data) => {
                    match data.accessed() {
                        Ok(access_time) => access_time,
                        Err(_) => {
                            println!("Last accessed time metadata not supported on this platform.");
                            std::process::exit(0);
                        }
                    }
                }  
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            (access_time1).cmp(&access_time2)
        }
        Sort::Modified => {
            let modified_time1 = match Path::new(f1).metadata() {
                Ok(data) => {
                    match data.modified() {
                        Ok(modified_time) => modified_time,
                        Err(_) => {
                            println!("Last modified time metadata not supported on this platform.");
                            std::process::exit(0);
                        }
                    }
                }  
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            let modified_time2 = match Path::new(f2).metadata() {
                Ok(data) => {
                    match data.modified() {
                        Ok(modified_time) => modified_time,
                        Err(_) => {
                            println!("Last modified time metadata not supported on this platform.");
                            std::process::exit(0);
                        }
                    }
                }  
                Err(_) => {
                    println!("Couldn't get metadata.");
                    std::process::exit(0);
                }     
            };
            (modified_time1).cmp(&modified_time2)
        }
        Sort::None => Ordering::Equal,
    }
}
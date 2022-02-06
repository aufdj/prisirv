use std::{
    path::Path,
    cmp::Ordering,
    ffi::OsStr,
};

// Sort files to improve compression of solid archives.

#[derive(Debug)]
pub enum Sort { // Sort By:
    None,       // No sorting
    Ext,        // Extension
    Name,       // Name
    PrtDir,     // Parent Directory
    Created,    // Creation Time
    Accessed,   // Last Access Time
    Modified,   // Last Modification Time
}
pub fn sort_ext(f1: &str, f2: &str) -> Ordering { 
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
pub fn sort_name(f1: &str, f2: &str) -> Ordering {
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
pub fn sort_prtdir(f1: &str, f2: &str) -> Ordering {
    let parent1 = match Path::new(f1).parent() {
        Some(path) => path,
        None => Path::new(""),
    };
    let parent2 = match Path::new(f2).parent() {
        Some(path) => path,
        None => Path::new(""),
    };
    (parent1).cmp(parent2)
}
pub fn sort_created(f1: &str, f2: &str) -> Ordering {
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
pub fn sort_accessed(f1: &str, f2: &str) -> Ordering {
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
pub fn sort_modified(f1: &str, f2: &str) -> Ordering {
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
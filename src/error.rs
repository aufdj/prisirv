use std::{
    path::Path,
    process::exit,
};


pub fn invalid_sort_criteria(method: &str) -> ! {
    println!("{} is not a valid sort criteria.", method);
    println!();
    println!("Sorting Methods:");
    println!("  -sort ext      Sort by extension");
    println!("  -sort name     Sort by name");
    println!("  -sort len      Sort by length");
    println!("  -sort prt n    Sort by nth parent directory");
    println!("  -sort crtd     Sort by creation time");
    println!("  -sort accd     Sort by last access time");
    println!("  -sort mod      Sort by last modification time");
    println!();
    exit(0);
}

pub fn invalid_lvl() -> ! {
    println!("Couldn't parse parent directory level.");
    println!();
    println!("Sorting Methods:");
    println!("  -sort ext      Sort by extension");
    println!("  -sort name     Sort by name");
    println!("  -sort len      Sort by length");
    println!("  -sort prt n    Sort by nth parent directory");
    println!("  -sort crtd     Sort by creation time");
    println!("  -sort accd     Sort by last access time");
    println!("  -sort mod      Sort by last modification time");
    println!();
    exit(0);
}

pub fn out_of_range_memory_option(option: u64) -> ! {
    println!("{} is outside the valid range of memory options (0..9).", option);
    println!();
    println!("Memory Options:");
    println!("  -mem 0  6 MB   -mem 5  99 MB");
    println!("  -mem 1  9 MB   -mem 6  195 MB");
    println!("  -mem 2  15 MB  -mem 7  387 MB");
    println!("  -mem 3  27 MB  -mem 8  771 MB");
    println!("  -mem 4  51 MB  -mem 9  1539 MB");
    println!();
    exit(0);
}

pub fn invalid_memory_option() -> ! {
    println!("Invalid memory option."); 
    println!();
    println!("Memory Options:");
    println!("  -mem 0  6 MB   -mem 5  99 MB");
    println!("  -mem 1  9 MB   -mem 6  195 MB");
    println!("  -mem 2  15 MB  -mem 7  387 MB");
    println!("  -mem 3  27 MB  -mem 8  771 MB");
    println!("  -mem 4  51 MB  -mem 9  1539 MB");
    println!();
    exit(0);
}

pub fn invalid_scale() -> ! {
    println!("Invalid scale."); 
    exit(0);
}

pub fn invalid_block_size() -> ! {
    println!("Invalid block size."); 
    exit(0);
}

pub fn invalid_thread_count() -> ! {
    println!("Invalid threads option.");
    exit(0);
}



pub fn no_inputs() -> ! {
    println!("No inputs found.");
    exit(0);
}

pub fn invalid_input(path: &Path) -> ! {
    println!("{} is not a valid input.", path.display());
    exit(0);
}

pub fn no_prisirv_archive() -> ! {
    println!("Not a prisirv archive.");
    exit(0);
}



pub fn metadata_not_supported() -> ! {
    println!("Couldn't get metadata.");
    exit(0);
}

pub fn creation_time_not_supported() -> ! {
    println!("Creation time metadata not supported on this platform.");
    exit(0);
}

pub fn access_time_not_supported() -> ! {
    println!("Last accessed time metadata not supported on this platform.");
    exit(0);
}

pub fn modified_time_not_supported() -> ! {
    println!("Last modified time metadata not supported on this platform.");
    exit(0);
}



pub fn file_general(path: &Path) -> ! {
    println!("Couldn't open file {}", path.display());
    exit(0);
}

pub fn dir_general(path: &Path) -> ! {
    println!("Couldn't create directory {}", path.display());
    exit(0);
}

pub fn file_already_exists(path: &Path) -> ! {
    println!("A file at location {} already exists.", path.display());
    println!("To overwrite existing files, enable file clobbering via '-clb' or '-clobber'.");
    exit(0);
}

pub fn file_not_found(path: & Path) -> ! {
    println!("Couldn't open file {}: Not Found", path.display());
    exit(0);
}

pub fn permission_denied(path: &Path) -> ! {
    println!("Couldn't open file {}: Permission Denied", path.display());
    exit(0);
}

pub fn dir_already_exists(path: &Path) -> ! {
    println!("A directory at location {} already exists.", path.display());
    println!("To overwrite existing directories, enable file clobbering via '-clb' or '-clobber'.");
    exit(0);
}
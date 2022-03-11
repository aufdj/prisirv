use std::{
    path::Path,
    process::exit,
};

pub fn invalid_sort_criteria(method: &str) {
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

pub fn invalid_lvl() {
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

pub fn out_of_range_memory_option(option: u64) {
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

pub fn invalid_memory_option() {
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

pub fn invalid_block_size() {
    println!("Invalid block size."); 
    exit(0);
}

pub fn max_thread_count(option: usize) {
    println!("{} exceeds the maximum number of threads (128).", option);
    exit(0);
}

pub fn invalid_thread_count() {
    println!("Invalid threads option.");
    exit(0);
}

pub fn no_inputs() {
    println!("No inputs found.");
    exit(0);
}

pub fn invalid_input(input: &Path) {
    println!("{} is not a valid input.", input.display());
    exit(0);
}

pub fn found_solid_archive() {
    println!("Expected non-solid archive, found solid archive.");
    exit(0);
}

pub fn found_non_solid_archive() {
    println!("Expected solid archive, found non-solid archive.");
    exit(0);
}

pub fn no_prisirv_archive() {
    println!("Not a prisirv archive.");
    exit(0);
}

pub fn not_solid_archive(input: &Path) {
    println!("Input {} is not a solid archive.", input.display());
    println!("To extract a non-solid archive, omit option '-sld'.");
    exit(0);
}
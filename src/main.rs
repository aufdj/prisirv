mod buffered_io;
mod logistic;
mod statemap;
mod apm;
mod mixer;
mod match_model;
mod hash_table;
mod metadata;
mod predictor;
mod encoder;
mod decoder;
mod archive;
mod tables;


use std::{
    path::{Path, PathBuf},
    cmp::Ordering,
    time::Instant,
    env,  
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata,
    buffered_io::{
        new_input_file, new_output_file, new_dir
    },
    archive::{
        Archiver, Extractor,
        SolidArchiver, SolidExtractor
    },
};

const MEM: usize = 1 << 23;


fn print_program_info() {
    println!();
    println!("     ______   ______     ________  ______    ________  ______    __   __     
    /_____/\\ /_____/\\   /_______/\\/_____/\\  /_______/\\/_____/\\  /_/\\ /_/\\    
    \\:::_ \\ \\\\:::_ \\ \\  \\__.::._\\/\\::::_\\/_ \\__.::._\\/\\:::_ \\ \\ \\:\\ \\\\ \\ \\   
     \\:(_) \\ \\\\:(_) ) )_   \\::\\ \\  \\:\\/___/\\   \\::\\ \\  \\:(_) ) )_\\:\\ \\\\ \\ \\  
      \\: ___\\/ \\: __ `\\ \\  _\\::\\ \\__\\_::._\\:\\  _\\::\\ \\__\\: __ `\\ \\\\:\\_/.:\\ \\ 
       \\ \\ \\    \\ \\ `\\ \\ \\/__\\::\\__/\\ /____\\:\\/__\\::\\__/\\\\ \\ `\\ \\ \\\\ ..::/ / 
        \\_\\/     \\_\\/ \\_\\/\\________\\/ \\_____\\/\\________\\/ \\_\\/ \\_\\/ \\___/_(  
                                                                             ");
    println!();
    println!("Prisirv is a context mixing archiver based on lpaq1");
    println!("Source code available at https://github.com/aufdj/prisirv");
    println!();
    println!("USAGE: PROG_NAME [c|d] [-out [path]] [-sld] [-sort [..]] [-i [files|dirs]] [-q]");
    println!();
    println!("OPTIONS:");
    println!("   c      Compress");
    println!("   d      Decompress");
    println!("  -out    Specify output path");
    println!("  -sld    Create solid archive");
    println!("  -sort   Sort files (solid archives only)");
    println!("  -i      Denotes list of input files/dirs");
    println!("  -q      Suppresses output other than errors");
    println!();
    println!("      Sorting Methods:");
    println!("          -sort ext      Sort by extension");
    println!("          -sort prtdir   Sort by parent directory");
    println!("          -sort crtd     Sort by creation time");
    println!("          -sort accd     Sort by last access time");
    println!("          -sort mod      Sort by last modification time");
    println!();
    println!("EXAMPLE:");
    println!("  Compress file [\\foo\\bar.txt] and directory [\\baz] into solid archive [\\foo\\arch], \n  sorting files by creation time:");
    println!();
    println!("      prisirv c -out arch -sld -sort crtd -i \\foo\\bar.txt \\baz");
    println!();
    println!("  Decompress the archive:");
    println!();
    println!("      prisirv d -sld -i \\foo\\arch.pri");
    std::process::exit(0);
}


// Get file name or path without extension
pub fn file_name_no_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
    .split('.').next().unwrap()
}
pub fn file_path_no_ext(path: &Path) -> &str {
    path.to_str().unwrap()
    .split('.').next().unwrap()
}
// Get file name or path with extension 
pub fn file_name_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
}
pub fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}
pub fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}


#[derive(Debug)]
enum Sort {   // Sort By:
    None,     // No sorting
    Ext,      // Extension
    PrtDir,   // Parent Directory
    Created,  // Creation Time
    Accessed, // Last Access Time
    Modified, // Last Modification Time
}
fn sort_ext(f1: &str, f2: &str) -> Ordering { // TODO: Handle files with no extension
         (Path::new(f1).extension().unwrap().to_ascii_lowercase())
    .cmp(&Path::new(f2).extension().unwrap().to_ascii_lowercase())
}
fn sort_prtdir(f1: &str, f2: &str) -> Ordering {
        (Path::new(f1).parent().unwrap())
    .cmp(Path::new(f2).parent().unwrap())
}
fn sort_created(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).metadata().unwrap().created().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().created().unwrap())
}
fn sort_accessed(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).metadata().unwrap().accessed().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().accessed().unwrap())
}
fn sort_modified(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).metadata().unwrap().modified().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().modified().unwrap())
}


#[derive(PartialEq, Eq)]
enum Format {
    Archive,
    Extract,
    ArchiveSolid,
    ExtractSolid,
}

// An -out option containing \'s will be treated as an absolute path.
// An -out option with no \'s creates a new archive inside the same directory as the first input.
// i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive \baz\arch,
// while option '-out arch' creates archive \foo\arch.
fn format_dir_out(fmt: Format, user_out: &str, arg: &Path) -> String {
    let mut dir_out = String::new();
    if user_out.is_empty() {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_pri", file_path_no_ext(arg)),
            Format::Extract =>      format!("{}_d",   file_path_no_ext(arg)),
            Format::ArchiveSolid => format!("{}.pri", file_path_no_ext(arg)),
            Format::ExtractSolid => format!("{}_d",   file_path_no_ext(arg)),
        }    
    }
    else if user_out.contains('\\') {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_pri", user_out),
            Format::Extract =>      format!("{}_d",   user_out),
            Format::ArchiveSolid => format!("{}.pri", user_out),
            Format::ExtractSolid => format!("{}_d",   user_out),
        }    
    }
    else {
        // Replace final path component with user option
        let s: Vec<String> = 
            file_path_ext(arg)
            .split('\\').skip(1)
            .map(|s| s.to_string())
            .collect();
        for cmpnt in s.iter().take(s.len()-1) {
            dir_out.push_str(format!("\\{}", cmpnt).as_str());
        }
        dir_out = match fmt {
            Format::Archive =>      format!("{}\\{}_pri", dir_out, user_out),
            Format::Extract =>      format!("{}\\{}_d",   dir_out, user_out),
            Format::ArchiveSolid => format!("{}\\{}.pri", dir_out, user_out),
            Format::ExtractSolid => format!("{}\\{}_d",   dir_out, user_out),
        }    
    }   
    dir_out
}


fn collect_files(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push(
            (file_path_ext(file), 0, 0)
        );
    }
    if !dirs.is_empty() {
        for dir in dirs.iter() {
            collect_files(dir, mta);
        }
    }
}

enum Parse {
    Mode,
    DirOut,
    Solid,
    Sort,
    Inputs,
    Quiet,
}

fn main() {
    let args = env::args().skip(1);

    if args.len() == 0 { print_program_info(); }

    #[allow(unused_assignments)]
    let mut dir_out = String::new();

    // Initialize settings
    let mut parser = Parse::Mode;
    let mut sort = Sort::None;
    let mut user_out = String::new();
    let mut inputs: Vec<String> = Vec::new();
    let mut solid = false;
    let mut quiet = false;
    let mut mode = "c";
    
    for arg in args {
        match arg.as_str() {
            "-sort" => {
                parser = Parse::Sort;
                continue;
            }, 
            "-out" => {
                parser = Parse::DirOut;
                continue;
            },     
            "-i" => { 
                parser = Parse::Inputs;
                continue;
            },
            "-sld" =>  parser = Parse::Solid,
            "-q" =>    parser = Parse::Quiet,
            "-help" => print_program_info(),
            _ => {},
        }
        match parser {
            Parse::Sort => {
                sort = match arg.as_str() {
                    "ext"    => Sort::Ext,
                    "prtdir" => Sort::PrtDir,
                    "crtd"   => Sort::Created,
                    "accd"   => Sort::Accessed,
                    "mod"    => Sort::Modified,
                    _ => panic!("No valid sort criteria found."),
                }
            }
            Parse::DirOut => user_out = arg,
            Parse::Inputs => inputs.push(arg),
            Parse::Solid =>  solid = true,
            Parse::Quiet =>  quiet = true,
            Parse::Mode => {
                mode = match arg.as_str() {
                    "c" => "c",
                    "d" => "d",
                    _ => panic!("Invalid mode."),
                };
            }   
        }
    }

    // Filter invalid inputs
    let inputs: Vec<PathBuf> = 
        inputs.iter()
        .map(PathBuf::from)
        .filter(|i| i.is_file() || i.is_dir())
        .collect();
    
    // Format output directory
    dir_out = match mode {
        "c" => {
            if solid { format_dir_out(Format::ArchiveSolid, &user_out, &inputs[0]) }
            else     { format_dir_out(Format::Archive,      &user_out, &inputs[0]) }
        }
        "d" => {
            if solid { format_dir_out(Format::ExtractSolid, &user_out, &inputs[0]) }
            else     { format_dir_out(Format::Extract,      &user_out, &inputs[0]) }
        }
        _ => String::new(),
    };
    
    if !quiet {
        println!();
        println!("//////////////////////////////////////////////////////////////");
        println!(
            "{} {} archive {} of inputs:\n{:#?},\nsorting by {}.",
            if mode == "c" { "Creating" } else { "Extracting" },
            if solid { "solid" } else { "non-solid" },
            dir_out, 
            inputs,
            match sort {
                Sort::None     => "none",
                Sort::Ext      => "extension",
                Sort::PrtDir   => "parent directory",
                Sort::Created  => "creation time",
                Sort::Accessed => "last accessed time",
                Sort::Modified => "last modified time",
            }
        );
        println!("//////////////////////////////////////////////////////////////");
        println!();
    }

    if solid {
        let mut mta: Metadata = Metadata::new();
        match mode {
            "c" => {
                // Sort files and directories
                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
                    inputs.into_iter().partition(|f| f.is_file());

                // Add file paths and lengths to metadata
                for file in files.iter() {
                    mta.files.push(
                        (file_path_ext(file), 0, 0)
                    );
                }
                // Walk through directories and gather rest of files
                if !dirs.is_empty() {
                    for dir in dirs.iter() {
                        collect_files(dir, &mut mta);
                    }
                }

                // Sort files to potentially improve compression of solid archives
                match sort {
                    Sort::None     => {},
                    Sort::Ext      => mta.files.sort_by(|f1, f2| sort_ext(&f1.0, &f2.0)),
                    Sort::PrtDir   => mta.files.sort_by(|f1, f2| sort_prtdir(&f1.0, &f2.0)),
                    Sort::Created  => mta.files.sort_by(|f1, f2| sort_created(&f1.0, &f2.0)),
                    Sort::Accessed => mta.files.sort_by(|f1, f2| sort_accessed(&f1.0, &f2.0)),
                    Sort::Modified => mta.files.sort_by(|f1, f2| sort_modified(&f1.0, &f2.0)),
                }

                let enc = Encoder::new(new_output_file(4096, Path::new(&dir_out)));
                let mut sld_arch = SolidArchiver::new(enc, mta, quiet);

                sld_arch.create_archive();
                sld_arch.write_metadata();    
            }
            "d" => {
                new_dir(&dir_out);

                let dec = Decoder::new(new_input_file(4096, &inputs[0]));
                let mut sld_extr = SolidExtractor::new(dec, mta, quiet);

                sld_extr.read_metadata();
                sld_extr.extract_archive(&dir_out);    
            }
            _ => println!("Couldn't parse input. For help, type PROG_NAME."),
        }
    }
    else {
        match mode {
            "c" => {
                let mut arch = Archiver::new(quiet);
                new_dir(&dir_out);

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    inputs.into_iter().partition(|f| f.is_file());

                for file_in in files.iter() {
                    let time = Instant::now();
                    if !quiet { println!("Compressing {}", file_in.display()); }
                    let file_in_size  = file_len(file_in); 
                    let file_out_size = arch.compress_file(file_in, &dir_out);
                    if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                        file_in_size, file_out_size, time.elapsed()); }
                }
                for dir_in in dirs.iter() {
                    arch.compress_dir(dir_in, &mut dir_out);      
                }
            }
            "d" => {
                let extr = Extractor::new(quiet);
                new_dir(&dir_out);

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    inputs.into_iter().partition(|f| f.is_file());

                for file_in in files.iter() {
                    let time = Instant::now();
                    if !quiet { println!("Decompressing {}", file_in.display()); }
                    let file_in_size  = file_len(file_in); 
                    let file_out_size = extr.decompress_file(file_in, &dir_out);
                    if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                        file_in_size, file_out_size, time.elapsed()); } 
                }
                for dir_in in dirs.iter() {
                    extr.decompress_dir(dir_in, &mut dir_out, true);      
                }
            }
            _ => println!("Couldn't parse input. For help, type PROG_NAME."),
        }
    }     
}

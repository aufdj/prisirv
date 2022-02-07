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
mod sort;
mod formatting;


use std::{
    path::{Path, PathBuf},
    time::Instant,
    io::{Seek, SeekFrom},
    env,  
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata,
    archive::{
        Archiver, Extractor,
        SolidArchiver, SolidExtractor
    },
    buffered_io::{
        new_input_file, new_output_file, 
        new_dir_checked,
    },
    sort::{
        Sort, sort_ext, sort_name, sort_prtdir,
        sort_created, sort_accessed, sort_modified,
        sort_len,
    },
    formatting::format_root_output_dir,
};


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
    println!("USAGE: PROG_NAME [c|d] [-out [path]] [-mem [0..9]] [-sld] [-sort [..]] [-i [files|dirs]] [-q]");
    println!();
    println!("Option [c|d] must be first, all other options can be in any order.");
    println!();
    println!("OPTIONS:");
    println!("   c      Compress");
    println!("   d      Decompress");
    println!("  -out    Specify output path");
    println!("  -sld    Create solid archive");
    println!("  -mem    Specifies memory usage");
    println!("  -sort   Sort files (solid archives only)");
    println!("  -i      Denotes list of input files/dirs");
    println!("  -q      Suppresses output other than errors");
    println!();
    println!("      Sorting Methods (Default - none):");
    println!("          -sort ext      Sort by extension");
    println!("          -sort prtdir   Sort by parent directory");
    println!("          -sort crtd     Sort by creation time");
    println!("          -sort accd     Sort by last access time");
    println!("          -sort mod      Sort by last modification time");
    println!();
    println!("      Memory Options (Default - 3):");
    println!("          -mem 0  6 MB   -mem 5  99 MB");
    println!("          -mem 1  9 MB   -mem 6  195 MB");
    println!("          -mem 2  15 MB  -mem 7  387 MB");
    println!("          -mem 3  27 MB  -mem 8  771 MB");
    println!("          -mem 4  51 MB  -mem 9  1539 MB");
    println!();
    println!("      Decompression requires same memory option used for compression.");
    println!("      Any memory option specified for decompression will be ignored.");
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


pub fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}
pub fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}

// Recursively collect all files into a vector for sorting before compression.
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
    for dir in dirs.iter() {
        collect_files(dir, mta);
    }
}

enum Parse {
    Mode,
    DirOut,
    Solid,
    Sort,
    Inputs,
    Quiet,
    Clobber,
    Mem,
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
    let mut mem = 1 << 23;
    let mut clbr = false;
    
    // Parse arguments
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
            "-mem" => {
                parser = Parse::Mem;
                continue;
            }
            "-sld"  => parser = Parse::Solid,
            "-q"    => parser = Parse::Quiet,
            "-clbr" => parser = Parse::Clobber,
            "-help" => print_program_info(),
            _ => {},
        }
        match parser {
            Parse::Sort => {
                sort = match arg.as_str() {
                    "ext"    => Sort::Ext,
                    "name"   => Sort::Name,
                    "len"    => Sort::Len,
                    "prtdir" => Sort::PrtDir,
                    "crtd"   => Sort::Created,
                    "accd"   => Sort::Accessed,
                    "mod"    => Sort::Modified,
                    _ => {
                        println!("No valid sort criteria found.");
                        std::process::exit(0);
                    }
                }
            }
            Parse::DirOut  => user_out = arg,
            Parse::Inputs  => inputs.push(arg),
            Parse::Solid   => solid = true,
            Parse::Quiet   => quiet = true,
            Parse::Clobber => clbr = true,
            Parse::Mode => {
                mode = match arg.as_str() {
                    "c" => "c",
                    "d" => "d",
                    _ => {
                        println!("Invalid mode.");
                        std::process::exit(0);
                    }
                };
            }  
            Parse::Mem => {
                // Parse memory option. If input is not a number
                // or not 0..9, ignore and use default option.
                mem = match arg.parse::<usize>() {
                    Ok(opt) => match opt {
                        0..=9 => 1 << (20 + opt),
                        _ => {
                            println!();
                            println!("Invalid memory option.");
                            println!("Using default of 27 MB.");
                            1 << 23
                        }
                    }
                    Err(_) => {
                        println!();
                        println!("Invalid memory option.");
                        println!("Using default of 27 MB.");
                        1 << 23
                    },
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
    
    dir_out = format_root_output_dir(mode, solid, &user_out, &inputs[0]);

    if !quiet {
        println!();
        println!("//////////////////////////////////////////////////////////////");
        println!(
            "{} {} archive {} of inputs:\n{:#?},\nsorting by {}{}.",
            if mode == "c" { "Creating" } else { "Extracting" },
            if solid { "solid" } else { "non-solid" },
            dir_out, 
            inputs,
            match sort {
                Sort::None     => "none",
                Sort::Ext      => "extension",
                Sort::Name     => "name",
                Sort::Len      => "length",
                Sort::PrtDir   => "parent directory",
                Sort::Created  => "creation time",
                Sort::Accessed => "last accessed time",
                Sort::Modified => "last modified time",
            },
            if mode == "c" {
                format!(", using {} MB of memory", 3 + (mem >> 20) * 3)
            } else { String::from("") }
        );
        println!("//////////////////////////////////////////////////////////////");
        println!();
    }

    if solid {
        let mut mta: Metadata = Metadata::new();
        match mode {
            "c" => {
                // Group files and directories 
                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
                    inputs.into_iter().partition(|f| f.is_file());

                // Walk through directories and collect all files
                for file in files.iter() {
                    mta.files.push(
                        (file_path_ext(file), 0, 0)
                    );
                }
                for dir in dirs.iter() {
                    collect_files(dir, &mut mta);
                }
                
                // Sort files to potentially improve compression of solid archives
                match sort {
                    Sort::None     => {},
                    Sort::Ext      => mta.files.sort_by(|f1, f2| sort_ext(&f1.0, &f2.0)),
                    Sort::Name     => mta.files.sort_by(|f1, f2| sort_name(&f1.0, &f2.0)),
                    Sort::Len      => mta.files.sort_by(|f1, f2| sort_len(&f1.0, &f2.0)),
                    Sort::PrtDir   => mta.files.sort_by(|f1, f2| sort_prtdir(&f1.0, &f2.0)),
                    Sort::Created  => mta.files.sort_by(|f1, f2| sort_created(&f1.0, &f2.0)),
                    Sort::Accessed => mta.files.sort_by(|f1, f2| sort_accessed(&f1.0, &f2.0)),
                    Sort::Modified => mta.files.sort_by(|f1, f2| sort_modified(&f1.0, &f2.0)),
                }

                let path = Path::new(&dir_out);
                // If file doesn't exist or is empty, ignore clobber option.
                if !path.exists() || file_len(path) == 0 {}
                // If file exists or is not empty, abort if user disallowed clobbering (default)
                else if !clbr {
                    println!("Archive {} already exists.", dir_out);
                    println!("To overwrite existing archives, use option '-clbr'.");
                    std::process::exit(0);
                }
                // If file exists and is not empty and user allowed clobbering, continue as normal.
                else {}

                let enc = Encoder::new(new_output_file(4096, path), mem, true);
                let mut sld_arch = SolidArchiver::new(enc, mta, quiet);

                sld_arch.create_archive();
                sld_arch.write_metadata();

                // Return final archive size including footer
                println!("Final archive size: {}", 
                    sld_arch.enc.file_out.seek(SeekFrom::End(0)).unwrap());
            }
            "d" => {
                if !inputs[0].is_file() {
                    println!("Input {} is not a solid archive.", inputs[0].display());
                    println!("To extract a non-solid archive, omit option '-sld'.");
                    std::process::exit(0);
                }
                let dec = Decoder::new(new_input_file(4096, &inputs[0]));
                let mut sld_extr = SolidExtractor::new(dec, mta, quiet, clbr);

                sld_extr.read_metadata();
                sld_extr.extract_archive(&dir_out);    
            }
            _ => println!("Couldn't parse input. For help, type PROG_NAME."),
        }
    }
    else {
        match mode {
            "c" => {
                let mut arch = Archiver::new(quiet, mem, clbr);
                new_dir_checked(&dir_out, clbr);

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
                let extr = Extractor::new(quiet, clbr);
                new_dir_checked(&dir_out, clbr);

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
use std::path::PathBuf;
use crate::{sort::Sort, Mode, Arch};


/// Parse command line arguments.
enum Parse {
    Mode,
    DirOut,
    Solid,
    Sort,
    Inputs,
    Quiet,
    Clobber,
    Mem,
    Lvl,
    BlkSz,
    Threads,
}

pub struct Config { 
    pub sort:      Sort,         // Sorting method (solid archives only)
    pub user_out:  String,       // User specified output directory (optional)
    pub inputs:    Vec<PathBuf>, // Inputs to be archived or extracted
    pub arch:      Arch,         // Solid or non-solid archive
    pub quiet:     bool,         // Suppresses output other than errors
    pub mode:      Mode,         // Compress or decompress
    pub mem:       usize,        // Memory usage
    pub clbr:      bool,         // Allow clobbering files
    pub blk_sz:    usize,        // Block size
    pub threads:   usize,        // Maximum number of threads
}
impl Config {
    pub fn new(args: &[String]) -> Config {
        if args.is_empty() { print_program_info(); }

        let mut parser = Parse::Mode;
        let mut sort = Sort::None;
        let mut user_out = String::new();
        let mut inputs: Vec<String> = Vec::new();
        let mut mem = 1 << 23;
        let mut arch = Arch::NonSolid;
        let mut quiet = false;
        let mut clbr = false;
        let mut mode = Mode::Compress;
        let mut blk_sz = 1 << 20;
        let mut threads = 4;
        
        for arg in args.iter() {
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
                "-blk" => {
                    parser = Parse::BlkSz;
                    continue;
                }
                "-threads" => {
                    parser = Parse::Threads;
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
                        "crtd"   => Sort::Created,
                        "accd"   => Sort::Accessed,
                        "mod"    => Sort::Modified,
                        "prtdir" => {
                            parser = Parse::Lvl;
                            Sort::PrtDir(1)
                        },
                        _ => {
                            println!("No valid sort criteria found.");
                            std::process::exit(0);
                        }
                    }
                }
                Parse::DirOut  => user_out = arg.to_string(),
                Parse::Inputs  => inputs.push(arg.to_string()),
                Parse::Solid   => arch = Arch::Solid,
                Parse::Quiet   => quiet = true,
                Parse::Clobber => clbr = true,
                Parse::Mode => {
                    mode = match arg.as_str() {
                        "c" => Mode::Compress,
                        "d" => Mode::Decompress,
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
                Parse::Lvl => {
                    match arg.parse::<usize>() {
                        Ok(lvl) => sort = Sort::PrtDir(lvl),
                        Err(_) => {},
                    }
                }
                Parse::BlkSz => {
                    match arg.parse::<usize>() {
                        Ok(size) => blk_sz = size*1024*1024,
                        Err(_) => {},
                    }
                }
                Parse::Threads => {
                    threads = match arg.parse::<usize>() {
                        Ok(opt) => match opt {
                            0..=128 => opt,
                            _ => {
                                println!();
                                println!("Maximum number of threads is 128.");
                                println!("Using default of 4 threads.");
                                4
                            }
                        }
                        Err(_) => {
                            println!();
                            println!("Invalid threads option.");
                            println!("Using default of 4 threads.");
                            4
                        },
                    };
                }
            }
        }

        if inputs.is_empty() {
            println!("No inputs found.");
            std::process::exit(0);
        }
        // Filter invalid inputs
        let inputs: Vec<PathBuf> = 
        inputs.iter()
        .map(PathBuf::from)
        .filter(|i| i.is_file() || i.is_dir())
        .collect();

        Config {
            sort, user_out, inputs,  
            arch, quiet,    mode,
            mem,  clbr,     blk_sz,
            threads,
        }
    }
}

/// Print information about prisirv.
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
    println!("USAGE: PROG_NAME [c|d] [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("   c        Compress");
    println!("   d        Decompress");
    println!("  -out      Specify output path");
    println!("  -sld      Create solid archive");
    println!("  -mem      Specify memory usage");
    println!("  -sort     Sort files (solid archives only)");
    println!("  -i        Specify list of input files/dirs");
    println!("  -q        Suppresses output other than errors");
    println!("  -clbr     Allows clobbering files");
    println!("  -threads  Specify thread count (non-solid archives only)");
    println!();
    println!("      Sorting Methods (Default - none):");
    println!("          -sort ext       Sort by extension");
    println!("          -sort name      Sort by name");
    println!("          -sort len       Sort by length");
    println!("          -sort prtdir n  Sort by nth parent directory");
    println!("          -sort crtd      Sort by creation time");
    println!("          -sort accd      Sort by last access time");
    println!("          -sort mod       Sort by last modification time");
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
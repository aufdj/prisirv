use std::{
    path::PathBuf,
    process::exit,
};
use crate::{
    sort::Sort, Mode, Arch,
    formatting::fmt_root_output_dir,
};


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

#[derive(Clone)]
pub struct Config {
    pub sort:      Sort,         // Sorting method (solid archives only)
    pub user_out:  String,       // User specified output directory (optional)
    pub dir_out:   String,       // Output directory
    pub inputs:    Vec<PathBuf>, // Inputs to be archived or extracted
    pub arch:      Arch,         // Solid or non-solid archive
    pub quiet:     bool,         // Suppresses output other than errors
    pub mode:      Mode,         // Compress or decompress
    pub mem:       u64,          // Memory usage
    pub clbr:      bool,         // Allow clobbering files
    pub blk_sz:    usize,        // Block size
    pub threads:   usize,        // Maximum number of threads
}
impl Config {
    pub fn new(args: &[String]) -> Config {
        if args.is_empty() { print_program_info(); }

        let mut parser   = Parse::Mode;
        let mut sort     = Sort::None;
        let mut user_out = String::new();
        let mut blk_sz   = 1 << 20;
        let mut mem      = 1 << 23;
        let mut arch     = Arch::NonSolid;
        let mut mode     = Mode::Compress;
        let mut quiet    = false;
        let mut clbr     = false;
        let mut threads  = 4;
        let mut inputs: Vec<String> = Vec::new();
        
        for arg in args.iter() {
            match arg.as_str() {
                "-sort" => {
                    parser = Parse::Sort;
                    continue;
                }, 
                "-out" | "-outputdir" => {
                    parser = Parse::DirOut;
                    continue;
                },     
                "-i" | "-inputs" => { 
                    parser = Parse::Inputs;
                    continue;
                },
                "-mem" | "-memory" => {
                    parser = Parse::Mem;
                    continue;
                }
                "-blk" | "-blocksize" => {
                    parser = Parse::BlkSz;
                    continue;
                }
                "-threads" => {
                    parser = Parse::Threads;
                    continue;
                }
                "-sld"  | "-solid"   => parser = Parse::Solid,
                "-q"    | "-quiet"   => parser = Parse::Quiet,
                "-clb"  | "-clobber" => parser = Parse::Clobber,
                "help" => print_program_info(),
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
                        "prt"    => {
                            parser = Parse::Lvl;
                            Sort::PrtDir(1)
                        },
                        _ => {
                            println!("No valid sort criteria found.");
                            exit(0);
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
                        "c" | "compress"   => Mode::Compress,
                        "d" | "decompress" => Mode::Decompress,
                        _ => {
                            println!("Invalid mode.");
                            exit(0);
                        }
                    };
                }  
                Parse::Mem => {
                    // Parse memory option. If input is not a number
                    // or not 0..9, ignore and use default option.
                    mem = match arg.parse::<u64>() {
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
            exit(0);
        }
        // Filter invalid inputs
        let inputs: Vec<PathBuf> = 
        inputs.iter()
        .map(PathBuf::from)
        .filter(|i| i.is_file() || i.is_dir())
        .collect();

        let dir_out = fmt_root_output_dir(arch, mode, &user_out, &inputs[0]);

        let cfg = Config {
            sort,    user_out, inputs,  
            arch,    quiet,    mode,
            mem,     clbr,     blk_sz,
            threads, dir_out,
        };
        cfg.print();
        cfg
    }
    fn print(&self) {
        if !self.quiet {
            println!();
            println!("=======================================================================");
            println!(" {} {} Archive of Inputs:", 
                if self.mode == Mode::Compress { "Creating" } else { "Extracting" },
                if self.arch == Arch::Solid { "Solid" } else { "Non-Solid" });
            for input in self.inputs.iter() {
                println!("    {} ({})", 
                    input.display(),
                    if input.is_file() { "File" }
                    else if input.is_dir() { "Directory" }
                    else { "" }
                );
            }
            println!();
            println!(" Output Directory: {}", self.dir_out);
            if self.mode == Mode::Compress {
                println!(" Sorting by: {}", 
                match self.sort {
                    Sort::None     => "None",
                    Sort::Ext      => "Extension",
                    Sort::Name     => "Name",
                    Sort::Len      => "Length",
                    Sort::Created  => "Creation time",
                    Sort::Accessed => "Last accessed time",
                    Sort::Modified => "Last modified time",
                    Sort::PrtDir(_) => "Parent Directory",
                });
                println!(" Memory Usage: {} MiB", 3 + (self.mem >> 20) * 3);
                println!(" Block Size: {} MiB", self.blk_sz/1024/1024);    
            }
            println!(" Threads: Up to {}", self.threads);
            println!("=======================================================================");
            println!();
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
    println!("  
    Prisirv, Context Mixing File Archiver
    Copyright (C) 2022 aufdj

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.");
    println!("
    Source code available at https://github.com/aufdj/prisirv");
    println!();
    println!();
    println!("  USAGE: PROG_NAME [c|d] [-i [..]] [OPTIONS|FLAGS]");
    println!();
    println!("  REQUIRED:");
    println!("      c,     compress      Compress");
    println!("      d,     decompress    Decompress");
    println!("     -i,    -inputs        Specify list of input files/dirs");
    println!();
    println!("  OPTIONS:");
    println!("    -out,   -outputdir     Specify output path");
    println!("    -mem,   -memory        Specify memory usage (Default 3)");
    println!("    -blk,   -blocksize     Specify block size (Default - 1MiB");
    println!("    -threads               Specify thread count (Default 4)");
    println!("    -sort                  Sort files (solid archives only) (Default - none)");
    println!();
    println!("  FLAGS:");
    println!("    -sld,   -solid         Create solid archive");
    println!("    -q,     -quiet         Suppresses output other than errors");
    println!("    -clb,   -clobber       Allows clobbering files");
    println!();
    println!("  Sorting Methods:");
    println!("      -sort ext      Sort by extension");
    println!("      -sort name     Sort by name");
    println!("      -sort len      Sort by length");
    println!("      -sort prt n    Sort by nth parent directory");
    println!("      -sort crtd     Sort by creation time");
    println!("      -sort accd     Sort by last access time");
    println!("      -sort mod      Sort by last modification time");
    println!();
    println!("  Memory Options:");
    println!("      -mem 0  6 MB   -mem 5  99 MB");
    println!("      -mem 1  9 MB   -mem 6  195 MB");
    println!("      -mem 2  15 MB  -mem 7  387 MB");
    println!("      -mem 3  27 MB  -mem 8  771 MB");
    println!("      -mem 4  51 MB  -mem 9  1539 MB");
    println!();
    println!("  Decompression requires same memory option used for compression.");
    println!("  Any memory option specified for decompression will be ignored.");
    println!();
    println!("  EXAMPLE:");
    println!("      Compress file [\\foo\\bar.txt] and directory [\\baz] into solid archive [\\foo\\arch], \n      sorting files by creation time:");
    println!();
    println!("          prisirv c -i \\foo\\bar.txt \\baz -out arch -sld -sort crtd ");
    println!();
    println!("      Decompress the archive:");
    println!();
    println!("          prisirv d -i \\foo\\arch.prsv -sld ");
    exit(0);
}

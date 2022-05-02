use crate::{
    block::Block,
    buffered_io::new_input_file,
    config::Config,
};

// Count number of blocks in an archive.
pub fn block_count(cfg: &Config) -> usize {
    let mut count = 0;
    let mut blk = Block::new(&cfg);
    let mut archive = new_input_file(4096, &cfg.ex_arch.path);
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 { return count; }
        count += 1;
        blk.next();
    }
}

// Print archive information.
pub fn list_archive(cfg: &Config) -> ! {
    let mut blk = Block::new(&cfg);
    let mut archive = new_input_file(4096, &cfg.ex_arch.path);
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 { break; }
        blk.print();
        blk.next();
    }
    std::process::exit(0);
}
use prisirv::{
    Prisirv, Mode,
    config::Config,
};

/// Create a new Config and call Prisirv API.
fn main() {
    let cfg = Config::new(&std::env::args().skip(1).collect::<Vec<String>>());
    match cfg.mode {
        Mode::Compress   => { Prisirv::new_with_cfg(cfg).create_archive();  }
        Mode::Decompress => { Prisirv::new_with_cfg(cfg).extract_archive(); }
    }
}

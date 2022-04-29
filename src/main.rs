use prisirv::{
    Prisirv, Mode,
    config::Config,
};

/// Create a new Config and call Prisirv API.
fn main() {
    let cfg = Config::new(&std::env::args().skip(1).collect::<Vec<String>>());
    match cfg.mode {
        Mode::Compress   => { Prisirv::new(cfg).create_archive();  }
        Mode::Decompress => { Prisirv::new(cfg).extract_archive(); }
        Mode::Add        => { Prisirv::new(cfg).add_archive();     }
    }
}

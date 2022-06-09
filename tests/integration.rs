
#[cfg(test)]
mod tests {
    use prisirv::Prisirv;
    use prisirv::error::PrisirvError;
    use prisirv::crc32::Crc32;
    use std::{fs, path::Path};

    #[test]
    fn append() -> Result<(), PrisirvError> {
        Prisirv::default()
        .clobber()
        .inputs(&["tests/data/calgary.tar"])?
        .create_archive()?;

        Prisirv::default()
        .ex_arch("tests/data/calgary.prsv")?
        .inputs(&["tests/data/canterbury"])?
        .append_files()?;

        Prisirv::default()
        .clobber()
        .ex_arch("tests/data/calgary.prsv")?
        .extract_archive()?;
        
        let calgary_crc = Path::new("tests/data/calgary/calgary.tar").crc32();
        let lsp_crc = Path::new("tests/data/calgary/canterbury/code/lsp/grammar.lsp").crc32();

        fs::remove_dir_all("tests/data/calgary").unwrap();
        fs::remove_file("tests/data/calgary.prsv").unwrap();

        println!();
        println!("calgary.tar CRC: {:x}", calgary_crc);
        println!("grammar.lsp CRC: {:x}", lsp_crc);
        println!();
        
        assert!(calgary_crc == 0xbda30921);
        assert!(lsp_crc == 0xd313977d);
        Ok(())
    }
}

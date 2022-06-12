
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
        
        let calgary_crc  = Path::new("tests/data/calgary/calgary.tar").crc32();
        let lsp_crc      = Path::new("tests/data/calgary/canterbury/code/lsp/grammar.lsp").crc32();
        let fields_crc   = Path::new("tests/data/calgary/canterbury/code/c/fields.c").crc32();
        let asyoulik_crc = Path::new("tests/data/calgary/canterbury/text/asyoulik.txt").crc32();

        fs::remove_dir_all("tests/data/calgary").unwrap();
        fs::remove_file("tests/data/calgary.prsv").unwrap();

        println!();
        println!("calgary.tar CRC:  {:x}", calgary_crc);
        println!("grammar.lsp CRC:  {:x}", lsp_crc);
        println!("fields.c CRC:     {:x}", fields_crc);
        println!("asyoulik.txt CRC: {:x}", asyoulik_crc);
        println!();
        
        assert!(calgary_crc == 0xBDA30921);
        assert!(lsp_crc == 0xD313977D);
        assert!(fields_crc == 0x4F618664);
        assert!(asyoulik_crc == 0x015E5966);
        Ok(())
    }
}

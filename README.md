# prisirv

Prisirv is a context mixing archiver based on [lpaq1 by Matt Mahoney.](http://mattmahoney.net/dc/#lpaq)

<pre>
USAGE: PROG_NAME [c|d] [-i [..]] [OPTIONS|FLAGS]

REQUIRED: 
    c,     compress      Compress
    d,     decompress    Decompress
   -i,    -inputs        Specify list of inputs

OPTIONS:
   -out,  -outputdir     Specify output path
   -mem,  -memory        Specify memory usage   (Default - 27 MiB)
   -blk,  -blocksize     Specify block size     (Default - 1 MiB)
   -threads              Specify thread count   (Default - 4)
   -sort                 Sort files             (Default - none)

FLAGS:
   -sld,  -solid         Create solid archive
   -q,    -quiet         Suppress output other than errors
   -clb,  -clobber       Allow file clobbering

Sorting Methods:
   -sort ext      Sort by extension
   -sort name     Sort by name
   -sort len      Sort by length
   -sort prt n    Sort by nth parent directory
   -sort crtd     Sort by creation time
   -sort accd     Sort by last access time
   -sort mod      Sort by last modification time
  
Memory Options:
   -mem 0  6 MB   -mem 5  99 MB
   -mem 1  9 MB   -mem 6  195 MB
   -mem 2  15 MB  -mem 7  387 MB
   -mem 3  27 MB  -mem 8  771 MB
   -mem 4  51 MB  -mem 9  1539 MB
    
Decompression requires same memory option used for compression.
Any memory option specified for decompression will be ignored.
  
EXAMPLE:
  Compress file [\foo\bar.txt] and directory [\baz] into solid archive [\foo\arch],
  sorting files by creation time:

    prisirv c -i \foo\bar.txt \baz -out arch -sld -sort crtd 

  Decompress the archive:

    prisirv d -i \foo\arch.prsv -sld
</pre>


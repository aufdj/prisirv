# prisirv

Prisirv is a context mixing archiver based on lpaq1.

<pre>
USAGE: PROG_NAME [c|d] [-out [path]] [-mem [0..9]] [-sld] [-sort [..]] [-i [files|dirs]] [-q]

Option [c|d] must be first, all other options can be in any order.

OPTIONS:
   c      Compress
   d      Decompress
  -out    Specify output path
  -sld    Create solid archive
  -mem    Specifies memory usage
  -sort   Sort files (solid archives only)
  -i      Denotes list of input files/dirs
  -q      Suppresses output other than errors

   Sorting Methods (Default - none):
      -sort ext     Sort by extension
      -sort prtdir  Sort by parent directory
      -sort crtd    Sort by creation time
      -sort accd    Sort by last access time
      -sort mod     Sort by last modification time
  
   Memory Options (Default - 3):
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

    prisirv c -out arch -sld -sort crtd -i \foo\bar.txt \baz

  Decompress the archive:

    prisirv d -sld \foo\arch.pri
</pre>


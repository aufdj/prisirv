# prisirv

Prisirv is a context mixing archiver based on lpaq1.

<pre>
USAGE: PROG_NAME [c|d] [-out [path]] [-sld] [-sort [..]] [-i [files|dirs]] [-q]

OPTIONS:
   c      Compress
   d      Decompress
  -out    Specify output path
  -sld    Create solid archive
  -sort   Sort files (solid archives only)
  -i      Denotes list of input files/dirs
  -q      Suppresses output other than errors

Sorting Methods:
  ext     Sort by extension
  prtdir  Sort by parent directory
  crtd    Sort by creation time
  accd    Sort by last access time
  mod     Sort by last modification time
  
EXAMPLE:
  Compress file [\foo\bar.txt] and directory [\baz] into solid archive [\foo\arch], 
  sorting files by creation time:

    prisirv c -out arch -sld -sort crtd -i \foo\bar.txt \baz

  Decompress the archive:

    prisirv d -sld \foo\arch.pri
</pre>


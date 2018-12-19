p3dtxt
======

A tool for converting MLOD P3Ds to a text-based representation. This allows using various tools meant for text files for analyzing and modifying P3Ds, and makes resolving merge conflicts easier.

## File format

The format is a fairly direct translation of the binary one. About the only structural change is storing texture and material paths in a lookup table instead of storing them seperately for every face. The final size should be less than 150% of the size of the binary one for most models.

By default, integers are converted to hex and floats to decimal floating point numbers. This can introduce some minor rounding errors when converting back. To prevent this, pass the `--lossless` option to store integers in hex as well. The resolution float is given a `0x` prefix to indicate this. No other values have a `0x` prefix.

**Even with lossless conversion, converting to text and back can introduce some minor differences**: The face data structure in MLOD P3Ds always has enough room for 4 vertices, even if the face only has 3. This padding is not always zeroes (it seems to be old data left behind when rearranging or modifying faces in Object Builder). Since this information is useless, it's ignored during conversion to text, and replaced with zeroes when converting back. It's also discarded when binarizing to ODOL, so the original P3D and one converted to text and back should result in the same ODOL P3D regardless of these differences.

As an example here's the [IV bag](https://github.com/acemod/ACE3/blob/c9a47ec05337d8653d68492d29efccb55dfc1e1d/addons/medical/data/IVBag_500ml.p3d) from ACE as a text-based file:

- [lossy](https://gist.github.com/KoffeinFlummi/a64a158426373e848b3709511551e469)
- [lossless](https://gist.github.com/KoffeinFlummi/a38c238a9b917ab4912d274fb8c60473)

## Usage

```
p3dtxt

Usage:
    p3dtxt bin2txt [-l] [<source> [<target>]]
    p3dtxt txt2bin [<source> [<target>]]
    p3dtxt (-h | --help)
    p3dtxt --version

Commands:
    bin2txt     Convert a regular MLOD P3D into a text-based one.
    txt2bin     Convert a text-based P3D back into a binary one.

Options:
    -l --lossless   Store floats as hex strings to prevent rounding errors.
    -h --help       Show usage information and exit.
       --version    Print the version number and exit.
```

See `p3dtxt --help` for more.

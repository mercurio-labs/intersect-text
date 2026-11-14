# Pinned Unicode scalar data

`UnicodeData.txt` is the unmodified Unicode 13.0.0 database from
https://www.unicode.org/Public/13.0.0/ucd/UnicodeData.txt.
SHA-256: `bdbffbbfc8ad4d3a6d01b5891510458f3d36f7170422af4ea2bed3211a73e8bb`.
The accompanying Unicode license was retrieved from https://www.unicode.org/license.txt.

Java 17 Character uses Unicode 13. Decimal Integer/Long parsing calls the
UTF-16 `char` digit operation, so supplementary decimal digits are excluded.
Boolean equality uses simple upper/lower case mappings. The only non-ASCII
equivalent of a letter in true/false is U+017F (long s).

`python -B tools/scalar_unicode.py` regenerates the Rust subset offline;
`--check` verifies it. Rust builds consume the checked-in generated table and
require neither Python nor Java. These are source-derived rules, not recorded
observations from an executed Java oracle.

`python -B tools/ecore_identifier.py` generates Ecore type-parameter identifier
ranges from the same pinned database; `--check` verifies them. It uses Unicode
code points, including supplementary letters, and Ecore's dollar-sign exclusion.
The generated file is checked by the Python test suite. It introduces no Java
or Python requirement in production builds.

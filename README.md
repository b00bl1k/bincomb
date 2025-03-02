# bincomb

A utility to merge binary files. It can be used to create firmware that
consists of two or more files and requires writing some meta information into
the output file, such as a CRC.

## Grammar

```
script     -> statement* EOF ;
statement  -> expr ":" IDENT ":" IDENT args? EOL ;
args       -> expr ( "," expr )* ;
expr       -> term ;
term       -> primary ( ( "-" | "+" ) primary )* ;
primary    -> NUMBER | STRING | variable ;
variable   -> "$" IDENT "." IDENT ;
```

## Example

```
# This is a comment. Empty lines are allowed

0                :first    :file,"first.bin"
0x20             :second   :file,"second.bin"
$second.start-2  :crc1     :crc16,$second.start,$second.size
```


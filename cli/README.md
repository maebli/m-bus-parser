# M-Bus parser (wired) CLI

This is a command line interface for the m-bus parser. The parser is able to parse the link layer of the m-bus protocol. The application layer is not yet implemented fully, but works partially. It is planned to have various formats for the output. currently the output is in a table format. Input can be a a file or a string of the format "0x00 0x01 0x02 ..."m or "000102..." or "00 01 02...":.

## Example

### File input

```bash
$ cargo run -p m-bus-parser-cli --release -- parse --file ./tests/rscada/test-frames/GWF-MTKcoder.hex

+-----------------------+--------------+---------------+-------------+-----------+---------+--------+
| Identification Number | Manufacturer | Access Number | Status      | Signature | Version | Medium |
+-----------------------+--------------+---------------+-------------+-----------+---------+--------+
| 182007                | GWF          | 76            | No Error(s) | 0         | 53      | Water  |
+-----------------------+--------------+---------------+-------------+-----------+---------+--------+
+-------------------------------------+--------------------+
| Value                               | Data Information   |
+=====================================+====================+
| (182007+0)e0[](FabricationNumber, ) | 0,Inst,BCD 8-digit |
+-------------------------------------+--------------------+
| (269+0)e0[m^3 ]()                   | 0,Inst,BCD 8-digit |
+-------------------------------------+--------------------+

```

### String input

```bash
$ cargo run -p m-bus-parser-cli --release -- parse --data "68 3D 3D 68 08 01 72 00 51 20 02 82 4D 02 04 00 88 00 00 04 07 00 00 00 00 0C 15 03 00 00 00 0B 2E 00 00 00 0B 3B 00 00 00 0A 5A 88 12 0A 5E 16 05 0B 61 23 77 00 02 6C 8C 11 02 27 37 0D 0F 60 00 67 16"
'`
+-----------------------+--------------+---------------+------------------------------------------+-----------+---------+--------+
| Identification Number | Manufacturer | Access Number | Status                                   | Signature | Version | Medium |
+-----------------------+--------------+---------------+------------------------------------------+-----------+---------+--------+
| 2205100               | SLB          | 0             | Permanent error, Manufacturer specific 3 | 0         | 2       | Heat   |
+-----------------------+--------------+---------------+------------------------------------------+-----------+---------+--------+
+---------------------------+-----------------------+
| Value                     | Data Information      |
+===========================+=======================+
| (0+0)e4[W W ]()           | 0,Inst,32-bit Integer |
+---------------------------+-----------------------+
| (3+0)e-1[m^3 ]()          | 0,Inst,BCD 8-digit    |
+---------------------------+-----------------------+
| (0+0)e3[W ]()             | 0,Inst,BCD 6-digit    |
+---------------------------+-----------------------+
| (0+0)e-3[m^3 h^-1 ]()     | 0,Inst,BCD 6-digit    |
+---------------------------+-----------------------+
| (1288+0)e-1[°C ]()        | 0,Inst,BCD 4-digit    |
+---------------------------+-----------------------+
| (516+0)e-1[°C ]()         | 0,Inst,BCD 4-digit    |
+---------------------------+-----------------------+
| (7723+0)e-2[°K ]()        | 0,Inst,BCD 6-digit    |
+---------------------------+-----------------------+
| (4492+0)e0[](TimePoint, ) | 0,Inst,16-bit Integer |
+---------------------------+-----------------------+
| (3383+0)e0[day ]()        | 0,Inst,16-bit Integer |
+---------------------------+-----------------------+


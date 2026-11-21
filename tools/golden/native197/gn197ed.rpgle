      * Stage 2, S2.197: EXCEPT to a program-described DISK file: two
      * exception records added, with a constant, a character field and
      * zoned fields (one with edit code Z) at their end positions, read
      * back with SQL. Captured on PUB400 (tools/golden/native197.txt
      * n197_exceptdisk) for the native unit NEXCDSK.
     H DFTACTGRP(*NO)
     FGN197F    O  A F   20        DISK    USROPN EXTFILE('QTEMP/GN197F')
     D line            S            200    VARYING
     D name            S              8
     D qty             S              5  0
     D got             S             20
     D say             PR
     D  text                        200    VARYING CONST
      /free
        exec sql set option commit = *none;
        exec sql drop table QTEMP/GN197F;
        exec sql create table QTEMP/GN197F (r char(20) not null);
        open gn197f;
        name = 'abc';
        qty = 42;
        except addrec;
        qty = -7;
        except addtwo;
        close gn197f;
        exec sql declare c1 cursor for
                 select r from QTEMP/GN197F order by rrn(GN197F);
        exec sql open c1;
        exec sql fetch c1 into :got;
        dow sqlcode = 0;
          line += '[' + got + ']';
          exec sql fetch c1 into :got;
        enddo;
        exec sql close c1;
        say(line);
        *inlr = *on;
      /end-free
     OGN197F    EADD         ADDREC
     O                                            1 'T'
     O                       NAME                 9
     O                       QTY                 14
     OGN197F    EADD         ADDTWO
     O                                            2 'XY'
     O                       QTY           Z     20
     P say             B
     D say             PI
     D  text                        200    VARYING CONST
     D wbuf            S            200    VARYING
      /free
        wbuf = text;
        exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                 line => :wbuf, overwrite => 'APPEND', end_of_line => 'LF');
      /end-free
     P say             E

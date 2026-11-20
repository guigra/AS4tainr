      * Stage 2, S2.149: compile-time tables: LOOKUP of a table with its
      * alternate (found, then not found), of a table alone, and each
      * table's current element by its name. Captured on PUB400
      * (tools/golden/native149.txt n149_tables) for the native unit NTABLES.
     H DFTACTGRP(*NO)
     D TABA            S              1    DIM(3) CTDATA PERRCD(3)
     D TABB            S              3    DIM(3) ALT(TABA)
     D LINE            S             40    VARYING
     C     'B'           LOOKUP    TABA          TABB                     52
       line = tabb + %char(*in52);
     C     'Z'           LOOKUP    TABA          TABB                     53
       line += tabb + %char(*in53);
     C     'C'           LOOKUP    TABA                                   54
       line += taba + tabb;
       say(line);
       *inlr = *on;

       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                  line => :wbuf, overwrite => 'APPEND', end_of_line => 'LF');
       end-proc;
**
AoneBtwoCthr

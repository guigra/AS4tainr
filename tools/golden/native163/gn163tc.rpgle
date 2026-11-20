      * Stage 2, S2.163: CAT with a table's current element as factor 1
      * and as factor 2 (after LOOKUP of the table with its alternate).
      * Captured on PUB400 (tools/golden/native163.txt n163_tabcat) for
      * the native unit NTABCAT.
     H DFTACTGRP(*NO)
     D TABA            S              1    DIM(3) CTDATA PERRCD(3)
     D TABB            S              5    DIM(3) ALT(TABA)
     D OUT             S             20
     D OUT2            S             20
     D LINE            S             60    VARYING
     C     'B'           LOOKUP    TABA          TABB                     52
     C     'IS:'         CAT       TABB:1        OUT
     C     TABA          CAT(P)    TABB:0        OUT2
       line = %trimr(out) + '|' + %trimr(out2);
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

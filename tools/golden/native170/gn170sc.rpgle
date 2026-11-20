      * Stage 2, S2.170: named constants as character factors of string
      * operations: CAT factor 1 and factor 2, CAT(P), SCAN's search
      * argument and XLATE's from and to strings. Captured on PUB400
      * (tools/golden/native170.txt n170_strconst) for the native unit
      * NSTRCON.
     H DFTACTGRP(*NO)
     D MSG             C                   'MONTH IS'
     D SEP             C                   '-'
     D LOW             C                   'abc'
     D UPP             C                   'ABC'
     D OUT             S             20
     D OUT2            S             20
     D OUT3            S             10
     D POS             S              3  0
     D LINE            S             60    VARYING
     C     MSG           CAT       'MAY':1       OUT
     C     'X'           CAT(P)    SEP:0         OUT2
     C     SEP           SCAN      'A-B'         POS
     C     LOW:UPP       XLATE     'cabbage'     OUT3
       line = %trimr(out) + '|' + %trimr(out2) + '|' + %char(pos)
            + '|' + %trimr(out3);
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

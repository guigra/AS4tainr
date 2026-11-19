      * Stage 2, S2.115: MOVE and MOVEL with dates, times and timestamps:
      * a date to character (default *ISO and factor 1 *MDY), a date to
      * numeric, character and numeric (*YMD) to a date, a time to
      * character, a timestamp to a date, a date into a timestamp.
      * Captured on PUB400 (tools/golden/native115.txt n115_movedate) for
      * the native unit NMOVDAT.
       ctl-opt dftactgrp(*no);
       dcl-s d date inz(d'2026-11-19');
       dcl-s d2 date;
       dcl-s t time inz(t'13.45.10');
       dcl-s z timestamp inz(z'2026-01-02-03.04.05.000006');
       dcl-s c10 char(10);
       dcl-s c8 char(8);
       dcl-s n6 zoned(6:0);
       dcl-s n8 zoned(8:0);
       dcl-s line varchar(200);
     C                   MOVE      D             C10
       line = c10;
     C     *MDY          MOVE      D             C8
       line += ' ' + c8;
     C                   MOVE      D             N8
       line += ' ' + %char(n8);
       c10 = '2025-02-28';
     C                   MOVE      C10           D2
       line += ' ' + %char(d2);
       n6 = 240315;
     C     *YMD          MOVE      N6            D2
       line += ' ' + %char(d2);
     C                   MOVEL     T             C8
       line += ' ' + c8;
     C                   MOVE      Z             D2
       line += ' ' + %char(d2);
     C                   MOVE      D             Z
       line += ' ' + %char(z);
       say(line);
       *inlr = *on;
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                    line => :wbuf, overwrite => 'APPEND',
                    end_of_line => 'LF');
       end-proc;

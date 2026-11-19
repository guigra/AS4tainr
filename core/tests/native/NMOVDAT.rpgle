      * Native unit NMOVDAT (S2.115): MOVE and MOVEL with dates, times and
      * timestamps, the program captured on PUB400 as GN115MVD with DSPLY
      * for its output.
      * Source: golden capture tools/golden/native115 n115_movedate
      * (PUB400, IBM i 7.5, 2026-11-19).
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
       dsply (line);
       *inlr = *on;

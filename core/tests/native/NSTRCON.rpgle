      * Native unit NSTRCON (S2.170): named constants in string operations,
      * the program captured on PUB400 as GN170SC with DSPLY for its output.
      * Source: golden capture tools/golden/native170 n170_strconst (PUB400,
      * IBM i 7.5, 2026-11-20).
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
       dsply (line);
       *inlr = *on;

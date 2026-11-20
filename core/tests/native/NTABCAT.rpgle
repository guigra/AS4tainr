      * Native unit NTABCAT (S2.163): CAT with a table's current element,
      * the program captured on PUB400 as GN163TC with DSPLY for its
      * output.
      * Source: golden capture tools/golden/native163 n163_tabcat (PUB400,
      * IBM i 7.5, 2026-11-20).
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
       dsply (line);
       *inlr = *on;

**
AoneBtwoCthr

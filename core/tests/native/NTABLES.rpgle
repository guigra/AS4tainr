      * Native unit NTABLES (S2.149): compile-time tables and LOOKUP, the
      * program captured on PUB400 as GN149TB with DSPLY for its output.
      * Source: golden capture tools/golden/native149 n149_tables (PUB400,
      * IBM i 7.5, 2026-11-20).
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
       dsply (line);
       *inlr = *on;

**
AoneBtwoCthr

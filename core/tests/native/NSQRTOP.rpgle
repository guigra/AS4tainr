      * Native unit NSQRTOP (S2.148): the SQRT operation, the program
      * captured on PUB400 as GN148SQ with DSPLY for its output.
      * Source: golden capture tools/golden/native148 n148_sqrtop (PUB400,
      * IBM i 7.5, 2026-11-20).
     H DFTACTGRP(*NO)
     D SQ              S              5  2
     D SR              S              7  4
     D N               S             10I 0
     D M               S             10I 0
     D LINE            S             80    VARYING
     C                   SQRT(H)   10            SQ
     C                   SQRT      2             SR
     C                   SQRT(H)   17            N
     C                   SQRT      17            M
       line = %char(sq) + ' ' + %char(sr) + ' ' + %char(n) + ' ' + %char(m);
       dsply (line);
       *inlr = *on;


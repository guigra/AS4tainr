      * Native unit NMOVEAL (S2.154): MOVEA with a literal index, the
      * program captured on PUB400 as GN154MA with DSPLY for its output.
      * Source: golden capture tools/golden/native154 n154_moveal (PUB400,
      * IBM i 7.5, 2026-11-20).
     H DFTACTGRP(*NO)
     D CH              S              1    DIM(6)
     D A3              S              2    DIM(3) INZ('..')
     D B3              S              2    DIM(3) INZ('..')
     D LINE            S             40    VARYING
     D K               S             10I 0
     C                   MOVEA     'ABCDEF'      CH
     C                   MOVEA     'XY'          CH(2)
     C                   MOVEA     CH(4)         A3
     C                   MOVEA(P)  CH(5)         B3(2)
       for k = 1 to 6;
         line += ch(k);
       endfor;
       line += ' ' + a3(1) + a3(2) + a3(3) + ' ' + b3(1) + b3(2) + b3(3);
       dsply (line);
       *inlr = *on;

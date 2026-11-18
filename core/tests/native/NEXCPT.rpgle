      * Native unit NEXCPT (S2.78): EXCEPT to a program-described printer
      * file, the program captured on PUB400 without its reading of its
      * own spooled file: the printed lines are compared with the lines
      * PASE system printed from it. CLOSE of the printer (S2.79).
      * Source: golden capture tools/golden/native78 n78_except (PUB400,
      * IBM i 7.5, 2026-11-18).
       ctl-opt dftactgrp(*no);
     FQPRINT    O    F  132        PRINTER OFLIND(*INOF)
       dcl-s amt packed(7:2);
       dcl-s nm char(10);
       dcl-s qty zoned(5:0);
       amt = 1234.5;
       nm = 'ALPHA';
       qty = 7;
       *in50 = *on;
       except hdr;
       except dtl;
       amt = -42;
       nm = 'BETA';
       *in50 = *off;
       except dtl;
       except;
       except tot;
       except dtl;
       close qprint;
       *inlr = *on;
     OQPRINT    E            HDR            1
     O                                           10 'REPORT'
     O                       PAGE          Z     20
     OQPRINT    E            DTL            1
     O                       NM                  12
     O                       AMT           J     25
     O                       QTY           Z     32
     O               50                          40 'FLAG'
     OQPRINT    E                           2
     O                                           10 'UNNAMED'
     OQPRINT    E            TOT            1
     O                       QTY           ZB    10
     O                                           15 'TOT'
     O                       AMT                 30 '  ,  0.  -'

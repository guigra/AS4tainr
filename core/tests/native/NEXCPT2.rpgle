      * Native unit NEXCPT2 (review of S2.79, 2026-11-18): the program
      * captured on PUB400 as GN92EXC, DSPLY for its output; its printed
      * lines are the test's (not read back from the spooled file).
      * Source: golden capture tools/golden/native92 n92_except (PUB400,
      * IBM i 7.5, 2026-11-18).
       ctl-opt dftactgrp(*no);
     FQPRINT    O    F  132        PRINTER
       dcl-c ew '    0.  ';
       dcl-ds d;
         v varchar(5) inz('ABC');
         raw char(7) pos(1);
       end-ds;
       dcl-s amt packed(5:2);
              amt = 12.5;
       dsply ('B ' + %char(%len(v)) + ' ['
           + %xlate(x'00':'0':%subst(raw:3:5)) + ']');
       except blk;
       dsply ('A ' + %char(%len(v)) + ' ['
           + %xlate(x'00':'0':%subst(raw:3:5)) + ']');
       except ewr;
       local();
       close qprint;
       *inlr = *on;
     OQPRINT    E            BLK            1
     O                       V              B    10
     OQPRINT    E            EWR            1
     O                       AMT                 30 EW
       dcl-proc local;
         dcl-c ew '  ,  0.  -';
         except ewr;
       end-proc;

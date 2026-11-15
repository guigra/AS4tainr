      * Stage 2, S2.39: indicators, lowered to Rust. Expected: what IBM i
      * 7.5 gave (golden capture tools/golden/native46.txt n46_ind, cited
      * in native_units.rs).
       ctl-opt dftactgrp(*no);
       dcl-s n int(10) inz(5);
       dcl-s t varchar(40);
       *in50 = *on;
       *in51 = *off;
       *in52 = n > 3;
       *in53 = n > 3 and n < 4;
       *in54 = not *in52;
       *in55 = *in50 or *in51;
       t = 'IND';
       if *in50;
         t += ' 50';
       endif;
       if not *in51;
         t += ' N51';
       endif;
       if *in52 and not *in53;
         t += ' 52N53';
       endif;
       if *in54;
         t += ' 54';
       endif;
       if *in55;
         t += ' 55';
       endif;
       dsply (t);
     C                   SETON                                        6162
     C                   SETOFF                                       50
       t = 'SET';
       if *in61 and *in62 and not *in50;
         t += ' 61 62 N50';
       endif;
       dsply (t);
       *inlr = *on;

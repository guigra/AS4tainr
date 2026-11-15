      * Stage 2, S2.39: indicators. *INxx set to *ON, *OFF and to a
      * condition, IF on an indicator and NOT,
      * fixed-form SETON and SETOFF. Captured on PUB400
      * (tools/golden/native46.txt n46_ind) for the native unit test NIND.
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
       say(t);
     C                   SETON                                        6162
     C                   SETOFF                                       50
       t = 'SET';
       if *in61 and *in62 and not *in50;
         t += ' 61 62 N50';
       endif;
       say(t);
       *inlr = *on;
      * Write one line to gn1.out in the job's current directory (DSPLY
      * under `system` goes to QSYSOPR, a queue PUB400 users share).
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s lin varchar(200);
         lin = text;
         exec sql call qsys2.ifs_write_utf8(
                    path_name => 'gn1.out', line => :lin,
                    overwrite => 'APPEND', end_of_line => 'LF');
       end-proc;

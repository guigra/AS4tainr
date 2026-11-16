       // Stage 2, S2.50: %ALLOC, %REALLOC and DEALLOC (free form), ALLOC,
       // REALLOC and DEALLOC (fixed form), and their errors under MONITOR.
       // Captured on PUB400 (tools/golden/native51.txt n51_heap) for the native
       // unit NHEAP.
       ctl-opt dftactgrp(*no);
       dcl-s p pointer;
       dcl-s q pointer;
       dcl-s r pointer;
       dcl-s c10 char(10) based(p);
       dcl-s c20 char(20) based(q);
       dcl-s c5 char(5) based(r);
       dcl-s n int(10) inz(0);
       p = %alloc(10);
       c10 = 'HEAPDATA10';
       say('ALLOC [' + c10 + ']');
       p = %realloc(p : 20);
       q = p;
       say('REALLOC [' + %subst(c20 : 1 : 10) + ']');
       dealloc(n) p;
       if p = *null;
         say('DEALLOC N NULL');
       endif;
       dealloc p;
       say('DEALLOC OF NULL OK');
       monitor;
         p = %alloc(n);
         say('ALLOC 0 OK');
       on-error;
         say('ALLOC 0 ERROR ' + %char(%status));
       endmon;
       p = %alloc(4);
       q = p;
       dealloc p;
       monitor;
         dealloc q;
         say('TWICE OK');
       on-error;
         say('TWICE ERROR ' + %char(%status));
       endmon;
       monitor;
         r = %addr(n);
         r = %realloc(r : 8);
         say('REALLOC NOT HEAP OK');
       on-error;
         say('REALLOC NOT HEAP ERROR ' + %char(%status));
       endmon;
     C                   ALLOC     5             R
     C                   EVAL      C5 = 'FIXED'
     C                   EVAL      N = 5
     C                   REALLOC   N             R
       say('FIXED [' + c5 + ']');
       dealloc(n) r;
       if r = *null;
         say('FIXED DEALLOC NULL');
       endif;
       *inlr = *on;
       // Write one line to gn1.out in the job's current directory (DSPLY under
       // `system` goes to QSYSOPR, a queue PUB400 users share).
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s line varchar(200);
         line = text;
         exec sql call qsys2.ifs_write_utf8(
                  path_name => 'gn1.out',
                  line => :line,
                  overwrite => 'APPEND',
                  end_of_line => 'LF');
       end-proc;

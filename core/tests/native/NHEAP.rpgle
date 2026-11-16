       // Stage 2, S2.50: heap storage, lowered to Rust. Expected: what IBM i
       // 7.5 gave (golden capture tools/golden/native51.txt n51_heap, cited in
       // native_units.rs). The capture's fixed-form ALLOC and REALLOC are left
       // out: the interpreter does not take them (fidelity.md).
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
       dsply ('ALLOC [' + c10 + ']');
       p = %realloc(p : 20);
       q = p;
       dsply ('REALLOC [' + %subst(c20 : 1 : 10) + ']');
       dealloc(n) p;
       if p = *null;
         dsply ('DEALLOC N NULL');
       endif;
       dealloc p;
       dsply ('DEALLOC OF NULL OK');
       monitor;
         p = %alloc(n);
         dsply ('ALLOC 0 OK');
       on-error;
         dsply ('ALLOC 0 ERROR ' + %char(%status));
       endmon;
       p = %alloc(4);
       q = p;
       dealloc p;
       monitor;
         dealloc q;
         dsply ('TWICE OK');
       on-error;
         dsply ('TWICE ERROR ' + %char(%status));
       endmon;
       monitor;
         r = %addr(n);
         r = %realloc(r : 8);
         dsply ('REALLOC NOT HEAP OK');
       on-error;
         dsply ('REALLOC NOT HEAP ERROR ' + %char(%status));
       endmon;
       *inlr = *on;

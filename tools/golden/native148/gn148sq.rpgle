      * Stage 2, S2.148: the SQRT operation, with and without (H), into a
      * packed field, a field with more places and an integer. Captured on
      * PUB400 (tools/golden/native148.txt n148_sqrtop) for the native unit
      * NSQRTOP.
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
       say(line);
       *inlr = *on;

       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                  line => :wbuf, overwrite => 'APPEND', end_of_line => 'LF');
       end-proc;

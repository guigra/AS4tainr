     H DFTACTGRP(*NO)
     DA                S              5P 2 INZ(12.34)
     DB                S              5P 2 INZ(45.67)
     DR                S              7P 2
     DSMALL            S              3P 0 INZ(998)
     DZ                S              5P 1
     DM                S              7P 3
     C     A             ADD       B             R
       say('ARITH ADD ' + %char(r));
     C     A             SUB       B             R                    505152
       say('ARITH SUB ' + %char(r) + ' ' + *in50 + *in51 + *in52);
     C                   ADD       5             SMALL
       say('ARITH TRUNC ' + %char(small));
     C     A             MULT      B             Z
       say('ARITH MULT ' + %char(z));
     C     A             MULT(H)   B             Z
       say('ARITH MULTH ' + %char(z));
     C     A             MULT      B             M
       say('ARITH MULTFULL ' + %char(m));
     C                   Z-ADD     B             Z                    505152
       say('ARITH ZADD ' + %char(z) + ' ' + *in50 + *in51 + *in52);
     C                   Z-SUB     A             R                    505152
       say('ARITH ZSUB ' + %char(r) + ' ' + *in50 + *in51 + *in52);
     C                   Z-ADD     0             R                    505152
       say('ARITH ZERO ' + %char(r) + ' ' + *in50 + *in51 + *in52);
     C                   SUB       1             R
       say('ARITH SUB1 ' + %char(r));
       *inlr = *on;
       // Write one line to gn1.out in the job's current directory (DSPLY
       // under `system` goes to QSYSOPR, a queue PUB400 users share).
       // Fixed-form source: every line ends by column 80.
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

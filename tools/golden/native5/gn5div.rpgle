     H DFTACTGRP(*NO)
     DA                S              5P 0 INZ(17)
     DB                S              5P 0 INZ(5)
     DQ                S              5P 0
     DR                S              5P 0
     DQ2               S              7P 2
     DR2               S              7P 2
     DN                S              5P 0 INZ(-17)
     DZ                S              5P 0 INZ(0)
     C     A             DIV       B             Q
     C                   MVR                     R
       say('DIV ' + %char(q) + ' MVR ' + %char(r));
     C     A             DIV       B             Q2
     C                   MVR                     R2
       say('DIV2 ' + %char(q2) + ' MVR2 ' + %char(r2));
     C     N             DIV       B             Q
     C                   MVR                     R
       say('DIVNEG ' + %char(q) + ' MVR ' + %char(r));
     C     A             DIV(H)    B             Q
       say('DIVH ' + %char(q));
     C                   DIV       2             A                    505152
       say('DIVRES ' + %char(a) + ' ' + *in50 + *in51 + *in52);
       monitor;
     C     B             DIV       Z             Q
       say('DIV0 FAIL');
       on-error 00102;
       say('DIV0 00102');
       endmon;
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

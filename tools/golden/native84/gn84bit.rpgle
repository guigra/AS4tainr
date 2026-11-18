      * Stage 2, S2.84: fixed-form TESTN (digits, leading blanks, letters,
      * all blanks, a sign zone on the last digit), TESTZ (zones C, D and
      * F, & and -), BITON, BITOFF and TESTB (literal bits and bits of a
      * field, all off, mixed and on; one bit takes no low indicator,
      * RNF5117). Each shows its indicators (high,
      * low, equal). Captured on PUB400 (tools/golden/native84.txt
      * n84_bitops) for the native unit NBITOPS.
       ctl-opt dftactgrp(*no);
       dcl-s t3 char(3);
       dcl-s b1 char(1) inz(x'00');
       dcl-s m1 char(1) inz(x'81');
       dcl-s line varchar(60);
       line = 'N';
       t3 = '123';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = ' 12';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '1A3';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '   ';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '12C';
     C                   TESTN                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       say(line);
       line = 'Z';
       t3 = 'A';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = 'J';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '1';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '&';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       t3 = '-';
     C                   TESTZ                   T3                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       say(line);
       line = 'B';
     C                   BITON     '0167'        B1
     C                   TESTB     '01'          B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   TESTB     '012'         B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   BITOFF    '0'           B1
     C                   TESTB     '0'           B1                   51  53
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   TESTB     M1            B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
     C                   BITOFF    M1            B1
     C                   TESTB     '167'         B1                   515253
       line += ' ' + %char(*in51) + %char(*in52) + %char(*in53);
       say(line);
       *inlr = *on;
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                  line => :wbuf, overwrite => 'APPEND',
                  end_of_line => 'LF');
       end-proc;
